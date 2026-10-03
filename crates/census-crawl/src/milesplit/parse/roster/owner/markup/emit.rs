use super::{Declaration, Markup};
use html5gum::{Emitter, Error, State};

impl Emitter for Markup {
    type Token = Declaration;

    fn set_last_start_tag(&mut self, tag: Option<&[u8]>) {
        self.last_start_tag.clear();
        self.last_start_tag.push(tag.map_or(Default::default(), core::convert::identity));
    }

    fn pop_token(&mut self) -> Option<Self::Token> {
        self.queued.take()
    }

    fn emit_string(&mut self, bytes: &[u8]) {
        if self.script {
            self.body.extend_from_slice(bytes);
        }
    }

    fn init_start_tag(&mut self) {
        self.reset_tag(false);
    }

    fn init_end_tag(&mut self) {
        self.reset_tag(true);
    }

    fn emit_current_tag(&mut self) -> Option<State> {
        self.finish_attribute();
        self.emit_declaration();
        self.next_state()
    }

    fn push_tag_name(&mut self, bytes: &[u8]) {
        self.tag.push(bytes);
    }

    fn init_attribute(&mut self) {
        self.finish_attribute();
        self.attribute.clear();
    }

    fn push_attribute_name(&mut self, bytes: &[u8]) {
        self.attribute.push(bytes);
    }

    fn push_attribute_value(&mut self, bytes: &[u8]) {
        if let Some(attribute) = self.selected_attribute() {
            self.fields.push(attribute, bytes);
        }
    }

    fn current_is_appropriate_end_tag_token(&mut self) -> bool {
        self.closing
            && !self.tag.bytes().is_empty()
            && self.tag.bytes() == self.last_start_tag.bytes()
    }

    fn emit_eof(&mut self) {
        if self.script {
            self.script = false;
            self.queued = Some(Declaration::Script(std::mem::take(&mut self.body)));
        }
    }

    fn should_emit_errors(&mut self) -> bool {
        false
    }

    fn emit_error(&mut self, _: Error) {}
    fn set_self_closing(&mut self) {}
    fn emit_current_comment(&mut self) {}
    fn emit_current_doctype(&mut self) {}
    fn init_comment(&mut self) {}
    fn init_doctype(&mut self) {}
    fn push_comment(&mut self, _: &[u8]) {}
    fn push_doctype_name(&mut self, _: &[u8]) {}
    fn push_doctype_public_identifier(&mut self, _: &[u8]) {}
    fn push_doctype_system_identifier(&mut self, _: &[u8]) {}
    fn set_doctype_public_identifier(&mut self, _: &[u8]) {}
    fn set_doctype_system_identifier(&mut self, _: &[u8]) {}
    fn set_force_quirks(&mut self) {}
}
