//! A minimal JSON reader for prikk's `--format json` reports (RFC 026 §6).
//!
//! # Why stikk parses JSON by hand
//!
//! **stikk's dependency tree is one crate deep** — `ratatui`, and nothing else outside the workspace.
//! That is not an accident: RFC 020 built a supply-chain gate because a `time` advisory reached stikk
//! transitively and forced an MSRV raise that nothing in CI had predicted. `serde` + `serde_json` would
//! add roughly eight crates, two of them proc-macros, to read three flat schemas.
//!
//! **This reader is scoped to exactly that job**: enough JSON to read `log-report-v1`,
//! `branch-list-v1` and `tag-list-v1`, and nothing more. It has no `Serialize` half, no floats, no
//! derive, and no configurability.
//!
//! **It cannot panic on any input.** Every entry point returns [`Result`], every index is bounds-checked
//! through iterators, and depth is capped — the workspace lint set forbids `unwrap`/`expect`/indexing in
//! this crate, and the fuzz-shaped tests in `json/tests.rs` feed it truncated, nested and hostile input.
//! A front-end that panicked a user's terminal on a malformed read would be a worse failure than not
//! reading it at all.
//!
//! **This is a decision worth overruling if you disagree** (flagged in RFC 026 A's review request): the
//! alternative is `serde_json`, which is better-tested than anything written here and costs a wider
//! tree. The three parsers above this are written against [`Json`], not against the reader, so swapping
//! it is a contained change.

use stikk_model::StikkError;

type Result<T> = std::result::Result<T, StikkError>;

/// The only JSON shapes prikk's reports use. No floats: every number in the three schemas is a
/// non-negative integer, and accepting a float would mean accepting a value stikk has nowhere to put.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    Number(u64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The value of `key`, if this is an object containing it.
    pub(crate) fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// A required string field — the error names the field, because a missing one is a schema change
    /// and the reader's job is to say which.
    pub(crate) fn str_field(&self, key: &str) -> Result<&str> {
        match self.get(key) {
            Some(Json::String(s)) => Ok(s),
            Some(other) => Err(schema(&format!(
                "field `{key}` should be a string, got {}",
                kind_of(other)
            ))),
            None => Err(schema(&format!("missing field `{key}`"))),
        }
    }

    /// A required integer field.
    pub(crate) fn u64_field(&self, key: &str) -> Result<u64> {
        match self.get(key) {
            Some(Json::Number(n)) => Ok(*n),
            Some(other) => Err(schema(&format!(
                "field `{key}` should be a number, got {}",
                kind_of(other)
            ))),
            None => Err(schema(&format!("missing field `{key}`"))),
        }
    }

    /// A required boolean field.
    pub(crate) fn bool_field(&self, key: &str) -> Result<bool> {
        match self.get(key) {
            Some(Json::Bool(b)) => Ok(*b),
            Some(other) => Err(schema(&format!(
                "field `{key}` should be a boolean, got {}",
                kind_of(other)
            ))),
            None => Err(schema(&format!("missing field `{key}`"))),
        }
    }

    /// A required array field.
    pub(crate) fn array_field(&self, key: &str) -> Result<&[Json]> {
        match self.get(key) {
            Some(Json::Array(items)) => Ok(items),
            Some(other) => Err(schema(&format!(
                "field `{key}` should be an array, got {}",
                kind_of(other)
            ))),
            None => Err(schema(&format!("missing field `{key}`"))),
        }
    }

    /// An optional string field: absent or `null` both read as `None`, which is how prikk spells
    /// "no value" in these schemas (`"reason": null`, `"previous_ref_state_id": null`).
    pub(crate) fn opt_str_field(&self, key: &str) -> Result<Option<&str>> {
        match self.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::String(s)) => Ok(Some(s)),
            Some(other) => Err(schema(&format!(
                "field `{key}` should be a string or null, got {}",
                kind_of(other)
            ))),
        }
    }
}

fn kind_of(value: &Json) -> &'static str {
    match value {
        Json::Null => "null",
        Json::Bool(_) => "a boolean",
        Json::Number(_) => "a number",
        Json::String(_) => "a string",
        Json::Array(_) => "an array",
        Json::Object(_) => "an object",
    }
}

/// A schema mismatch is an [`StikkError::Environment`] rather than a refusal: prikk answered, and the
/// answer was not the shape this stikk knows. That is a version-skew condition, the same class
/// `version.rs` handles, not something the user did.
fn schema(detail: &str) -> StikkError {
    StikkError::environment_msg(format!(
        "prikk's JSON report is not the shape stikk expects: {detail}"
    ))
}

/// The maximum nesting depth this reader accepts.
///
/// prikk's deepest report nests three levels (report → array → object → array → object). Sixteen is
/// far above anything the schemas use and far below anything that could exhaust the stack — a bound
/// rather than a guess, because recursion over attacker-shaped input without one is how a reader
/// becomes a crash.
const MAX_DEPTH: usize = 16;

/// Parse `text` as a single JSON value, rejecting trailing content.
pub(crate) fn parse(text: &str) -> Result<Json> {
    let bytes: Vec<char> = text.chars().collect();
    let mut cursor = Cursor {
        chars: &bytes,
        at: 0,
    };
    cursor.skip_whitespace();
    let value = cursor.value(0)?;
    cursor.skip_whitespace();
    if cursor.peek().is_some() {
        return Err(schema("trailing content after the top-level value"));
    }
    Ok(value)
}

struct Cursor<'a> {
    chars: &'a [char],
    at: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek();
        if ch.is_some() {
            self.at += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.at += 1;
        }
    }

    fn expect(&mut self, want: char) -> Result<()> {
        match self.next() {
            Some(ch) if ch == want => Ok(()),
            Some(ch) => Err(schema(&format!("expected `{want}`, found `{ch}`"))),
            None => Err(schema(&format!("expected `{want}`, found end of input"))),
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json> {
        if depth > MAX_DEPTH {
            return Err(schema("nested deeper than stikk reads"));
        }
        self.skip_whitespace();
        match self.peek() {
            Some('{') => self.object(depth),
            Some('[') => self.array(depth),
            Some('"') => Ok(Json::String(self.string()?)),
            Some('t' | 'f') => self.boolean(),
            Some('n') => self.null(),
            Some(ch) if ch.is_ascii_digit() => self.number(),
            Some(ch) => Err(schema(&format!("unexpected `{ch}`"))),
            None => Err(schema("unexpected end of input")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json> {
        self.expect('{')?;
        let mut fields = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.at += 1;
            return Ok(Json::Object(fields));
        }
        loop {
            self.skip_whitespace();
            let key = self.string()?;
            self.skip_whitespace();
            self.expect(':')?;
            let value = self.value(depth + 1)?;
            fields.push((key, value));
            self.skip_whitespace();
            match self.next() {
                Some(',') => {}
                Some('}') => return Ok(Json::Object(fields)),
                Some(ch) => return Err(schema(&format!("expected `,` or `}}`, found `{ch}`"))),
                None => return Err(schema("unterminated object")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.at += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_whitespace();
            match self.next() {
                Some(',') => {}
                Some(']') => return Ok(Json::Array(items)),
                Some(ch) => return Err(schema(&format!("expected `,` or `]`, found `{ch}`"))),
                None => return Err(schema("unterminated array")),
            }
        }
    }

    fn string(&mut self) -> Result<String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.next() {
                None => return Err(schema("unterminated string")),
                Some('"') => return Ok(out),
                Some('\\') => match self.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{8}'),
                    Some('f') => out.push('\u{c}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => out.push(self.unicode_escape()?),
                    Some(ch) => return Err(schema(&format!("unknown escape `\\{ch}`"))),
                    None => return Err(schema("escape at end of input")),
                },
                Some(ch) => out.push(ch),
            }
        }
    }

    /// A `\uXXXX` escape, including the surrogate pair form.
    ///
    /// An unpaired surrogate becomes `U+FFFD` rather than an error: prikk does not emit one, but a
    /// report is not worth refusing over a character stikk renders inert anyway (`C-T2a`).
    fn unicode_escape(&mut self) -> Result<char> {
        let first = self.hex4()?;
        if (0xD800..0xDC00).contains(&first) {
            // A high surrogate: a low one must follow to form a character.
            if self.peek() == Some('\\') {
                let save = self.at;
                self.at += 1;
                if self.peek() == Some('u') {
                    self.at += 1;
                    let second = self.hex4()?;
                    if (0xDC00..0xE000).contains(&second) {
                        let combined = 0x1_0000 + ((first - 0xD800) << 10) + (second - 0xDC00);
                        return Ok(char::from_u32(combined).unwrap_or('\u{FFFD}'));
                    }
                }
                self.at = save;
            }
            return Ok('\u{FFFD}');
        }
        Ok(char::from_u32(first).unwrap_or('\u{FFFD}'))
    }

    fn hex4(&mut self) -> Result<u32> {
        let mut value = 0u32;
        for _ in 0..4 {
            let ch = self
                .next()
                .ok_or_else(|| schema("truncated `\\u` escape"))?;
            let digit = ch
                .to_digit(16)
                .ok_or_else(|| schema("non-hex digit in a `\\u` escape"))?;
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn boolean(&mut self) -> Result<Json> {
        if self.literal("true") {
            Ok(Json::Bool(true))
        } else if self.literal("false") {
            Ok(Json::Bool(false))
        } else {
            Err(schema("expected `true` or `false`"))
        }
    }

    fn null(&mut self) -> Result<Json> {
        if self.literal("null") {
            Ok(Json::Null)
        } else {
            Err(schema("expected `null`"))
        }
    }

    fn literal(&mut self, word: &str) -> bool {
        let end = self.at + word.chars().count();
        let matches = self
            .chars
            .get(self.at..end)
            .is_some_and(|slice| slice.iter().copied().eq(word.chars()));
        if matches {
            self.at = end;
        }
        matches
    }

    /// A non-negative integer. **No floats, no exponents, no leading `-`** — every number in prikk's
    /// three schemas is a count or a sequence number, and silently accepting `1.5` where stikk stores a
    /// `u64` would be the reader inventing a value.
    fn number(&mut self) -> Result<Json> {
        let start = self.at;
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.at += 1;
        }
        if matches!(self.peek(), Some('.' | 'e' | 'E')) {
            return Err(schema(
                "a fractional or exponent number, where stikk expects an integer",
            ));
        }
        let text: String = self
            .chars
            .get(start..self.at)
            .unwrap_or_default()
            .iter()
            .collect();
        text.parse::<u64>()
            .map(Json::Number)
            .map_err(|_| schema("an integer too large for stikk to hold"))
    }
}

#[cfg(test)]
mod tests;
