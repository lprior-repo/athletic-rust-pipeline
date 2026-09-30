pub(super) fn scan_tag(body: &str) -> Option<(usize, String, Option<String>)> {
    let mut scan = TagScan::default();
    for (index, ch) in body.char_indices() {
        if scan.feed(ch) {
            return Some((index, scan.name, scan.class));
        }
    }
    None
}

#[derive(Clone, Copy, Default)]
enum State {
    #[default]
    InName,
    BeforeAttr,
    AttrName,
    AfterName,
    BeforeValue,
    Quoted(char),
    Unquoted,
    Slash,
}

#[derive(Default)]
struct TagScan {
    name: String,
    state: State,
    current: String,
    value: String,
    class: Option<String>,
}

impl TagScan {
    fn feed(&mut self, ch: char) -> bool {
        match self.state {
            State::InName => self.in_name(ch),
            State::BeforeAttr => self.before_attr(ch),
            State::AttrName => self.attr_name(ch),
            State::AfterName => self.after_name(ch),
            State::BeforeValue => self.before_value(ch),
            State::Quoted(quote) => self.quoted(ch, quote),
            State::Unquoted => self.unquoted(ch),
            State::Slash => self.slash(ch),
        }
    }

    fn in_name(&mut self, ch: char) -> bool {
        match ch {
            '>' => return true,
            '/' => self.state = State::Slash,
            ch if ch.is_ascii_whitespace() => self.state = State::BeforeAttr,
            ch => self.name.push(ch.to_ascii_lowercase()),
        }
        false
    }

    fn before_attr(&mut self, ch: char) -> bool {
        match ch {
            '>' => return true,
            '/' => self.state = State::Slash,
            ch if ch.is_ascii_whitespace() => {}
            ch => self.attr_start(ch),
        }
        false
    }

    fn attr_name(&mut self, ch: char) -> bool {
        match ch {
            '>' => return true,
            '=' => self.state = State::BeforeValue,
            '/' => self.state = State::Slash,
            ch if ch.is_ascii_whitespace() => self.state = State::AfterName,
            ch => self.current.push(ch.to_ascii_lowercase()),
        }
        false
    }

    fn after_name(&mut self, ch: char) -> bool {
        match ch {
            '=' => self.state = State::BeforeValue,
            '>' => return true,
            ch if ch.is_ascii_whitespace() => {}
            ch => self.attr_start(ch),
        }
        false
    }

    fn before_value(&mut self, ch: char) -> bool {
        match ch {
            '>' => return true,
            quote @ ('"' | '\'') => {
                self.value.clear();
                self.state = State::Quoted(quote);
            }
            ch if ch.is_ascii_whitespace() => {}
            ch => {
                self.value.clear();
                self.value.push(ch);
                self.state = State::Unquoted;
            }
        }
        false
    }

    fn quoted(&mut self, ch: char, quote: char) -> bool {
        if ch == quote {
            self.record();
            self.state = State::BeforeAttr;
        } else {
            self.value.push(ch);
        }
        false
    }

    fn unquoted(&mut self, ch: char) -> bool {
        if ch == '>' {
            self.record();
            return true;
        }
        if ch.is_ascii_whitespace() {
            self.record();
            self.state = State::BeforeAttr;
            return false;
        }
        self.value.push(ch);
        false
    }

    fn slash(&mut self, ch: char) -> bool {
        match ch {
            '>' => return true,
            ch if ch.is_ascii_whitespace() => self.state = State::BeforeAttr,
            ch => self.attr_start(ch),
        }
        false
    }

    fn attr_start(&mut self, ch: char) {
        self.current.clear();
        self.current.push(ch.to_ascii_lowercase());
        self.state = State::AttrName;
    }

    fn record(&mut self) {
        let previous = self.class.take();
        self.class = record_class(&self.current, &self.value, previous);
    }
}

fn record_class(name: &str, value: &str, class: Option<String>) -> Option<String> {
    if name == "class" && class.is_none() {
        Some(value.to_string())
    } else {
        class
    }
}
