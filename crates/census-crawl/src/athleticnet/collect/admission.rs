use crate::{CrawlError, CrawlResult};

const MAX_WORK: usize = 8192;
const MAX_TEXT: usize = 4096;
const MAX_DEPTH: usize = 64;

#[derive(Clone, Copy)]
enum Token {
    Outside,
    Text(usize),
    Escaped(usize),
}

struct Budget {
    token: Token,
    depth: usize,
    work: usize,
}

pub(super) fn check(body: &[u8]) -> CrawlResult<()> {
    body.iter()
        .try_fold(
            Budget {
                token: Token::Outside,
                depth: 0,
                work: 0,
            },
            |budget, byte| budget.accept(*byte),
        )
        .map(|_| ())
}

impl Budget {
    fn accept(mut self, byte: u8) -> CrawlResult<Self> {
        self.token = match (self.token, byte) {
            (Token::Text(_), b'"') => Token::Outside,
            (Token::Text(length), b'\\') => {
                Token::Escaped(grow(length, MAX_TEXT, "NET JSON string")?)
            }
            (Token::Text(length) | Token::Escaped(length), _) => {
                Token::Text(grow(length, MAX_TEXT, "NET JSON string")?)
            }
            (Token::Outside, b'"') => {
                self.work = grow(self.work, MAX_WORK, "NET JSON tokens")?;
                Token::Text(0)
            }
            (Token::Outside, b'{' | b'[') => {
                self.depth = grow(self.depth, MAX_DEPTH, "NET JSON depth")?;
                self.work = grow(self.work, MAX_WORK, "NET JSON tokens")?;
                Token::Outside
            }
            (Token::Outside, b'}' | b']') => {
                self.depth = self
                    .depth
                    .checked_sub(1)
                    .ok_or_else(|| CrawlError::Invariant {
                        detail: "NET JSON closing delimiter has no opener".to_string(),
                    })?;
                Token::Outside
            }
            (Token::Outside, b',') => {
                self.work = grow(self.work, MAX_WORK, "NET JSON tokens")?;
                Token::Outside
            }
            (Token::Outside, _) => Token::Outside,
        };
        Ok(self)
    }
}

fn grow(value: usize, limit: usize, resource: &'static str) -> CrawlResult<usize> {
    let requested = value.checked_add(1).ok_or_else(|| CrawlError::Arithmetic {
        detail: "NET JSON budget overflow".to_string(),
    })?;
    if requested > limit {
        return Err(CrawlError::Resource {
            resource,
            requested,
            limit,
        });
    }
    Ok(requested)
}
