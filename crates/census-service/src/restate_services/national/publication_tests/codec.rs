use super::TestResult;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub kind: u16,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(kind: u16, payload: Vec<u8>) -> Self {
        Self { kind, payload }
    }

    pub fn encode(&self) -> TestResult<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend(self.kind.to_be_bytes());
        bytes.extend([0_u8; 2]);
        bytes.extend(u32::try_from(self.payload.len())?.to_be_bytes());
        bytes.extend(&self.payload);
        Ok(bytes)
    }

    pub fn bytes(&self, tag: u64) -> TestResult<&[u8]> {
        field(&self.payload, tag)?
            .ok_or_else(|| format!("missing field {tag} in {:?}", self).into())
    }

    pub fn number(&self, tag: u64) -> TestResult<u64> {
        let mut bytes = self.bytes(tag)?;
        varint(&mut bytes)
    }

    pub fn text(&self, tag: u64) -> TestResult<String> {
        Ok(std::str::from_utf8(self.bytes(tag)?)?.to_owned())
    }
}

pub fn frames(mut bytes: &[u8]) -> TestResult<Vec<Frame>> {
    let mut frames = Vec::new();
    while !bytes.is_empty() {
        let header = take(&mut bytes, 8)?;
        let kind = u16::from_be_bytes(header.get(..2).ok_or("short type")?.try_into()?);
        let length = u32::from_be_bytes(header.get(4..8).ok_or("short length")?.try_into()?);
        frames.push(Frame::new(
            kind,
            take(&mut bytes, usize::try_from(length)?)?.to_vec(),
        ));
        if frames.len() > 4096 {
            return Err("fixture frame budget exhausted".into());
        }
    }
    Ok(frames)
}

pub fn integer(tag: u64, value: u64) -> Vec<u8> {
    let mut bytes = encode_varint(tag << 3);
    bytes.extend(encode_varint(value));
    bytes
}

pub fn blob(tag: u64, value: &[u8]) -> TestResult<Vec<u8>> {
    let mut bytes = encode_varint((tag << 3) | 2);
    bytes.extend(encode_varint(u64::try_from(value.len())?));
    bytes.extend(value);
    Ok(bytes)
}

pub fn value(content: &[u8]) -> TestResult<Vec<u8>> {
    blob(1, content)
}

pub fn notification(kind: u16, id: u64, content: &[u8]) -> TestResult<Frame> {
    let mut payload = integer(1, id);
    payload.extend(blob(5, &value(content)?)?);
    Ok(Frame::new(kind, payload))
}

pub fn field(mut bytes: &[u8], tag: u64) -> TestResult<Option<&[u8]>> {
    while !bytes.is_empty() {
        let key = varint(&mut bytes)?;
        let value = match key & 7 {
            0 => {
                let before = bytes;
                varint(&mut bytes)?;
                before
                    .get(
                        ..before
                            .len()
                            .checked_sub(bytes.len())
                            .ok_or("varint length")?,
                    )
                    .ok_or("short integer")?
            }
            2 => {
                let length = usize::try_from(varint(&mut bytes)?)?;
                take(&mut bytes, length)?
            }
            _ => return Err("unsupported fixture wire type".into()),
        };
        if key >> 3 == tag {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn take<'a>(bytes: &mut &'a [u8], length: usize) -> TestResult<&'a [u8]> {
    let value = bytes.get(..length).ok_or("short fixture frame")?;
    *bytes = bytes.get(length..).ok_or("short fixture remainder")?;
    Ok(value)
}

fn varint(bytes: &mut &[u8]) -> TestResult<u64> {
    let mut value = 0_u64;
    for shift in (0..64).step_by(7) {
        let byte = *take(bytes, 1)?.first().ok_or("short varint")?;
        value |= u64::from(byte & 127)
            .checked_shl(shift)
            .ok_or("varint overflow")?;
        if byte & 128 == 0 {
            return Ok(value);
        }
    }
    Err("fixture varint overflow".into())
}

fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    for _ in 0..10 {
        let byte = u8::try_from(value & 127).map_or(0, core::convert::identity);
        value >>= 7;
        bytes.push(if value == 0 { byte } else { byte | 128 });
        if value == 0 {
            break;
        }
    }
    bytes
}
