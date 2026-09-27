use crate::{
    model::{MAX_TOKENS_CAP, REQUEST_CAP},
    ModelError, ModelOptions,
};
use census_domain::model::ReviewPacket;
use serde::ser::{Serialize, Serializer};

#[derive(Default)]
struct BoundedWriter {
    buf: Vec<u8>,
    attempted: usize,
}

impl std::io::Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.attempted = match self.buf.len().checked_add(bytes.len()) {
            Some(length) => length,
            None => usize::MAX,
        };
        if self.attempted > REQUEST_CAP {
            return Err(std::io::Error::other(
                "model request exceeds its size limit",
            ));
        }
        if self.attempted > self.buf.capacity() {
            let capacity = self
                .buf
                .capacity()
                .saturating_mul(2)
                .max(256)
                .max(self.attempted)
                .min(REQUEST_CAP);
            self.buf
                .reserve_exact(capacity.saturating_sub(self.buf.len()));
        }
        self.buf.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct RequestBody<'a> {
    packet: &'a ReviewPacket,
    options: &'a ModelOptions,
}

impl Serialize for RequestBody<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("RequestBody", 6)?;
        state.serialize_field("model", self.options.model_name())?;
        state.serialize_field(
            "messages",
            &Messages {
                packet: self.packet,
            },
        )?;
        let capped = self.options.max_tokens().min(MAX_TOKENS_CAP);
        state.serialize_field("temperature", &0u32)?;
        state.serialize_field("max_tokens", &capped)?;
        state.serialize_field("response_format", &response_format())?;
        state.serialize_field("chat_template_kwargs", &ChatTemplateKwargs)?;
        state.end()
    }
}

struct Messages<'a> {
    packet: &'a ReviewPacket,
}

impl Serialize for Messages<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&SystemMessage)?;
        seq.serialize_element(&UserMessage {
            packet: self.packet,
        })?;
        seq.end()
    }
}

struct SystemMessage;

impl Serialize for SystemMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Message", 2)?;
        state.serialize_field("role", "system")?;
        state.serialize_field("content", system_prompt())?;
        state.end()
    }
}

struct UserMessage<'a> {
    packet: &'a ReviewPacket,
}

impl Serialize for UserMessage<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Message", 2)?;
        state.serialize_field("role", "user")?;
        state.serialize_field(
            "content",
            &DisplayAsString(PacketDisplay {
                packet: self.packet,
            }),
        )?;
        state.end()
    }
}

struct DisplayAsString<T: std::fmt::Display>(T);

impl<T: std::fmt::Display> Serialize for DisplayAsString<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.0)
    }
}

struct PacketDisplay<'a> {
    packet: &'a ReviewPacket,
}

impl std::fmt::Display for PacketDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "subject_id: {}", self.packet.subject_id)?;
        writeln!(f, "subject: {}\n", self.packet.subject)?;
        writeln!(f, "cases:")?;
        for case in &self.packet.cases {
            writeln!(f, "- case_id: {}", case.case_id)?;
            writeln!(f, "  family: {}", case.family)?;
            writeln!(f, "  detail: {}", case.detail)?;
        }
        writeln!(f, "\nevidence:")?;
        for fact in &self.packet.evidence {
            writeln!(f, "- {}: {} = {}", fact.source, fact.field, fact.value)?;
        }
        Ok(())
    }
}

fn response_format() -> serde_json::Value {
    serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "review_verdicts",
            "strict": true,
            "schema": verdict_schema()
        }
    })
}

struct ChatTemplateKwargs;

impl Serialize for ChatTemplateKwargs {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("enable_thinking", &false)?;
        map.end()
    }
}

pub fn build_request_body(
    packet: &ReviewPacket,
    options: &ModelOptions,
) -> Result<Vec<u8>, ModelError> {
    let mut writer = BoundedWriter::default();
    let body = RequestBody { packet, options };
    serde_json::to_writer(&mut writer, &body).map_err(|_| {
        if writer.attempted > REQUEST_CAP {
            ModelError::RequestTooLarge {
                bytes: writer.attempted,
            }
        } else {
            ModelError::RequestFailed
        }
    })?;
    Ok(writer.buf)
}

fn system_prompt() -> &'static str {
    "You adjudicate retained census findings about schools, meets and athletes.\n\
     Each case names a field that no stored source resolved.\n\
     \n\
     Answer with one verdict per case, in the order given:\n\
     - If the subject's own evidence settles the field, answer value_proposed and put the field\n\
       name and the value in `field` and `value`.\n\
     - Otherwise answer insufficient_evidence and leave `field` and `value` empty strings.\n\
     \n\
     Rules:\n\
     - Use only the evidence given; never invent a school, a state or a venue that is not implied\n\
       by it.\n\
     - The value must be the shortest form the field asks for: a two-letter state code for `state`,\n\
       a two-letter state code for `state`.\n\
     - `confidence` is 0-100 and reports how certain the evidence makes you, not how helpful the\n\
       answer would be.\n\
     - `rationale` is one sentence naming the evidence you used.\n\
     - Never answer for a case id you were not given."
}

fn verdict_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "subject_id": { "type": "string" },
            "verdicts": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "case_id": { "type": "string" },
                        "kind": {
                            "type": "string",
                            "enum": ["value_proposed", "insufficient_evidence"],
                        },
                        "field": { "type": "string" },
                        "value": { "type": "string" },
                        "confidence": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "rationale": { "type": "string" },
                    },
                    "required": [
                        "case_id", "kind", "field", "value", "confidence", "rationale",
                    ],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["subject_id", "verdicts"],
        "additionalProperties": false,
    })
}
