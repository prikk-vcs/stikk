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
//! `branch-list-v1` and `tag-list-v1`, and nothing more. It has **no floats read**, no derive, and
//! no configurability.
//!
//! # What it accepts, and what it refuses — in two halves
//!
//! The halves are **a field stikk reads** and **a value it only skips**, and RFC 064 is why they
//! differ. Until then they did not: one additive prikk field holding `-1` made a whole report
//! unreadable, for every released stikk, because the refusal happened in the parser and the parser
//! does not know which fields anyone wants (RFC 064 F1, measured). **stikk promised prikk the
//! other behaviour** in our letter 024 §2 — that prikk may add a field without stikk's readers
//! caring — and this is that promise kept.
//!
//! **A value stikk only skips may be any valid JSON.** A negative, fractional or exponent-bearing
//! number, an integer above `u64::MAX`, and nesting past [`MAX_DEPTH`] all read: the first group
//! as [`Json::OtherNumber`], holding prikk's own text, and the last as [`Json::TooDeep`], holding
//! nothing. Both are inert.
//!
//! **A field stikk reads keeps exactly the rule it had.** Every accessor refuses both variants —
//! [`Json::OtherNumber`] with its own sentence, [`Json::TooDeep`] with its own — and **naming the
//! field**, which the parser's refusal could not do. No accessor turns either into a value: not 0,
//! not empty, not absent.
//!
//! **And invalid JSON is still invalid, everywhere.** `-` alone, `1.`, `1e+`, an unclosed string,
//! a stray closer, a bad escape: refused whether or not anyone reads the field it sits in, and
//! refused past [`MAX_DEPTH`] too, where the value is scanned rather than built. *Skipped* never
//! means *accepted*.
//!
//! It has **no `Serialize` half either** — nothing stikk sends prikk is JSON. What it does have, since
//! RFC 035, is one *display* renderer ([`Json::render`]): Patch detail's raw view shows prikk's own
//! fields for an operation, and the alternative was teaching the parser to record byte spans so the
//! original text could be sliced back out. The renderer writes a parsed value as JSON text for a human
//! to read, in prikk's key order, and is never read back by anything.
//!
//! **It cannot panic on any input.** Every entry point returns [`Result`], every index is bounds-checked
//! through iterators, recursion is capped — the workspace lint set forbids `unwrap`/`expect`/indexing in
//! this crate, and the fuzz-shaped tests in `json/tests.rs` feed it truncated, nested and hostile input.
//! A front-end that panicked a user's terminal on a malformed read would be a worse failure than not
//! reading it at all.
//!
//! # Two places this reader is laxer than the spec, written down for the day someone swaps it
//!
//! Both matter only at the swap, and silently, which is why they are here rather than nowhere:
//!
//! - **Duplicate keys.** [`Json::get`] returns the **first** match; `serde_json` keeps the **last**.
//!   prikk emits no duplicate key in any of the three schemas, so the two agree today and would
//!   disagree without a compiler error.
//! - **Leading zeros.** `007` reads as `7`, where strict JSON refuses it outright. Laxness in the
//!   direction that cannot invent a value — but laxness, and the paragraph above is careful about what
//!   this reader refuses, so it should be equally careful about where it does not.
//!
//! **This is a decision worth overruling if you disagree** (flagged in RFC 026 A's review request): the
//! alternative is `serde_json`, which is better-tested than anything written here and costs a wider
//! tree. The three parsers above this are written against [`Json`], not against the reader, so swapping
//! it is a contained change.

use stikk_model::StikkError;

type Result<T> = std::result::Result<T, StikkError>;

/// The JSON shapes prikk's reports use, plus one for every **other** valid number (RFC 064
/// decision 1).
///
/// **No floats are read**: every number stikk reads from these schemas is a non-negative integer,
/// and accepting a float where a `u64` is stored would be the reader inventing a value. But a
/// number stikk does not read is prikk's business, not stikk's — so one is held as
/// [`Json::OtherNumber`], which no accessor accepts.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    Number(u64),
    /// A valid JSON number stikk has nowhere to put: negative, fractional, exponent-bearing, or a
    /// non-negative integer above `u64::MAX`. Its **exact text**, as prikk wrote it.
    ///
    /// **Inert.** Every accessor refuses it, naming the field (RFC 064 decision 3), so a field
    /// stikk reads keeps exactly the rule it had. It exists so that the same value in a field
    /// stikk *skips* costs nothing — which, before RFC 064, made the whole report unreadable
    /// (F1).
    OtherNumber(String),
    /// A **valid** JSON value nested deeper than [`MAX_DEPTH`], scanned and discarded (RFC 064
    /// decision 2). It carries nothing: not the text, not the shape, not a count.
    ///
    /// **Inert, and checked.** Every accessor refuses it, naming the field, so a field stikk
    /// reads keeps the rule it had. And "skipped" never means "accepted": the scan holds the
    /// whole JSON grammar, so malformed input past the limit is still refused.
    TooDeep,
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
            Some(Json::TooDeep) => Err(schema(&too_deep(key))),
            Some(other) => Err(schema(&format!(
                "field `{key}` should be a string, got {}",
                kind_of(other)
            ))),
            None => Err(schema(&format!("missing field `{key}`"))),
        }
    }

    /// A required integer field.
    ///
    /// **A number stikk cannot hold gets its own sentence** (RFC 064 decision 3): *should be a
    /// number, got a number* would tell a reader nothing, and what they need to know is that the
    /// value is a number of a kind stikk has nowhere to put.
    pub(crate) fn u64_field(&self, key: &str) -> Result<u64> {
        match self.get(key) {
            Some(Json::Number(n)) => Ok(*n),
            Some(Json::OtherNumber(_)) => Err(schema(&format!(
                "field `{key}` is a number stikk does not hold (negative, fractional, or larger \
                 than 2^64 - 1)"
            ))),
            Some(Json::TooDeep) => Err(schema(&too_deep(key))),
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
            Some(Json::TooDeep) => Err(schema(&too_deep(key))),
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
            Some(Json::TooDeep) => Err(schema(&too_deep(key))),
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
            // **Not `None`** (RFC 064 §5 trap 3): a skipped value is not an absent one.
            Some(Json::TooDeep) => Err(schema(&too_deep(key))),
            Some(other) => Err(schema(&format!(
                "field `{key}` should be a string or null, got {}",
                kind_of(other)
            ))),
        }
    }
}

impl Json {
    /// This value as JSON text, indented for a terminal — **prikk's fields, in prikk's order**, for
    /// Patch detail's raw view (RFC 035 decision 5).
    ///
    /// Object keys keep the order prikk wrote them, because [`Json::Object`] is a `Vec` of pairs rather
    /// than a map: the raw view shows the report's own shape, not an alphabetised version of it. Strings
    /// are re-escaped as JSON requires, so what comes back is readable JSON rather than a parsed value
    /// printed loosely — and a control character in repository content cannot reach the terminal from
    /// here even before `inert` sees it (`C-T2a`), since `\u001b` is written as an escape, not a byte.
    ///
    /// Not a round trip and not promised to be one: `007` renders as `7`, and whitespace is stikk's.
    pub(crate) fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        let pad = |out: &mut String, depth: usize| {
            for _ in 0..depth {
                out.push_str("  ");
            }
        };
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(true) => out.push_str("true"),
            Json::Bool(false) => out.push_str("false"),
            Json::Number(n) => out.push_str(&n.to_string()),
            // **Its own text, unescaped and unquoted** (RFC 064 decision 4): the number grammar
            // admits only ASCII digits, `-`, `.`, `e`, `E` and `+`, so there is nothing here for
            // an escape table to do and nothing a terminal could act on.
            Json::OtherNumber(text) => out.push_str(text),
            // **stikk's own placeholder, never the skipped text** (RFC 064 decision 4). The
            // skipped bytes were never parsed into strings, so they never passed through the
            // escape table -- a control character or a bidi override in there would reach the
            // terminal from this one `push_str` (`C-T2a`). Written through `write_string` so it
            // is a JSON string like any other value.
            Json::TooDeep => write_string(out, TOO_DEEP_PLACEHOLDER),
            Json::String(s) => write_string(out, s),
            Json::Array(items) if items.is_empty() => out.push_str("[]"),
            Json::Array(items) => {
                out.push_str("[\n");
                for (at, item) in items.iter().enumerate() {
                    pad(out, depth + 1);
                    item.write(out, depth + 1);
                    if at + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                pad(out, depth);
                out.push(']');
            }
            Json::Object(fields) if fields.is_empty() => out.push_str("{}"),
            Json::Object(fields) => {
                out.push_str("{\n");
                for (at, (key, value)) in fields.iter().enumerate() {
                    pad(out, depth + 1);
                    write_string(out, key);
                    out.push_str(": ");
                    value.write(out, depth + 1);
                    if at + 1 < fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                pad(out, depth);
                out.push('}');
            }
        }
    }
}

/// What the raw view shows in place of a value the reader skipped (RFC 064 decision 4).
///
/// **stikk's own text, and short.** The alternative — the skipped bytes — is the one thing this
/// variant exists not to carry.
pub(crate) const TOO_DEEP_PLACEHOLDER: &str = "\u{2026}nested deeper than stikk shows\u{2026}";

/// Write `text` as a JSON string literal, through the one escape table the workspace has
/// ([`crate::escape`]) — the same bytes prikk itself would have written.
///
/// **Shared with Patch detail's quoted content rows on purpose** (review v2 §2): this function and
/// `stikk-core`'s `quoted` used to hold two copies of the table, and the copies escaped category `Cc`
/// alone, so a bidi override or a zero-width character reached the terminal from both.
fn write_string(out: &mut String, text: &str) {
    out.push('"');
    crate::escape::escape_into(out, text);
    out.push('"');
}

fn kind_of(value: &Json) -> &'static str {
    match value {
        Json::Null => "null",
        Json::Bool(_) => "a boolean",
        Json::Number(_) => "a number",
        // **Named, so every accessor's wrong-type sentence names it too** (RFC 064 decision 3).
        Json::OtherNumber(_) => "a number stikk does not hold",
        Json::TooDeep => "nested deeper than stikk reads",
        Json::String(_) => "a string",
        Json::Array(_) => "an array",
        Json::Object(_) => "an object",
    }
}

/// The one sentence for a field whose value was skipped (RFC 064 decision 3).
///
/// **Every accessor gives it, naming the field.** A reader that fell through to `kind_of`'s
/// wrong-type sentence would say *should be a string, got nested deeper than stikk reads*, which
/// reads as a type confusion rather than as the limit it is.
fn too_deep(key: &str) -> String {
    format!("field `{key}` is nested deeper than stikk reads")
}

/// A schema mismatch is an [`StikkError::Environment`] rather than a refusal: prikk answered, and the
/// answer was not the shape this stikk knows. That is a version-skew condition, the same class
/// `version.rs` handles, not something the user did.
fn schema(detail: &str) -> StikkError {
    StikkError::environment_msg(format!(
        "prikk's JSON report is not the shape stikk expects: {detail}"
    ))
}

/// The maximum nesting depth this reader **builds**.
///
/// prikk's deepest report nests three levels (report → array → object → array → object). Sixteen is
/// far above anything the schemas use and far below anything that could exhaust the stack — a bound
/// rather than a guess, because recursion over attacker-shaped input without one is how a reader
/// becomes a crash.
///
/// **Past it the reader skips rather than refuses** (RFC 064 decision 2): the value is scanned by
/// [`Cursor::skip_value`], which uses a `Vec` instead of the call stack, checked against the whole
/// JSON grammar, and discarded as [`Json::TooDeep`]. So the recursion stays bounded at sixteen —
/// the reason this constant exists — while a report that nests something deeper somewhere stikk
/// never looks stays readable.
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
            return self.skip_value();
        }
        self.skip_whitespace();
        match self.peek() {
            Some('{') => self.object(depth),
            Some('[') => self.array(depth),
            Some('"') => Ok(Json::String(self.string()?)),
            Some('t' | 'f') => self.boolean(),
            Some('n') => self.null(),
            // **`-` is a number's first character** (RFC 064 decision 1), where it used to be
            // *"unexpected `-`"* — the sentence F1 measured on a field nobody reads.
            Some(ch) if ch.is_ascii_digit() || ch == '-' => self.number(),
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

    /// **RFC 8259's whole number grammar**, `-? int frac? exp?`, with two outcomes (RFC 064
    /// decision 1).
    ///
    /// - a value with no `-`, no fraction and no exponent that fits a `u64` is [`Json::Number`],
    ///   exactly as before;
    /// - **every other valid number** is [`Json::OtherNumber`], holding its text as prikk wrote
    ///   it. No accessor accepts one, so a field stikk reads keeps today's rule and only the
    ///   sentence improves;
    /// - **text that is not a JSON number at all is still refused**: `-` alone, `-a`, `1.`, `1e`,
    ///   `1e+`. (`.5` and `+1` never reach here — [`Cursor::value`] has no case for `.` or `+`.)
    ///
    /// **The documented leading-zero laxity stays exactly as it is** (`007` reads as `7`; see the
    /// module doc). Strict JSON refuses it, and changing that is outside RFC 064.
    fn number(&mut self) -> Result<Json> {
        let start = self.at;
        let negative = self.peek() == Some('-');
        if negative {
            self.at += 1;
        }
        // The integer part. One digit at least -- `-` alone, or `-a`, is not a number.
        if !self.take_digits() {
            return Err(schema(
                "a number with no digits where JSON requires at least one",
            ));
        }
        let mut fractional = false;
        if self.peek() == Some('.') {
            self.at += 1;
            fractional = true;
            if !self.take_digits() {
                return Err(schema("a number with no digits after `.`"));
            }
        }
        let mut exponent = false;
        if matches!(self.peek(), Some('e' | 'E')) {
            self.at += 1;
            exponent = true;
            if matches!(self.peek(), Some('+' | '-')) {
                self.at += 1;
            }
            if !self.take_digits() {
                return Err(schema("a number with no digits after its exponent"));
            }
        }
        let text: String = self
            .chars
            .get(start..self.at)
            .unwrap_or_default()
            .iter()
            .collect();
        if !negative
            && !fractional
            && !exponent
            && let Ok(value) = text.parse::<u64>()
        {
            return Ok(Json::Number(value));
        }
        Ok(Json::OtherNumber(text))
    }

    /// Scan one whole JSON value and discard it (RFC 064 decision 2).
    ///
    /// **Iterative, and that is the rule, not an optimisation** (§5 trap 1). `open` holds one
    /// entry per container the scan is inside, so the call stack does not grow with the input's
    /// depth: a recursive version passes every test here except the 100 000-level one, which is
    /// the test this rule exists for. Nothing is allocated per value — `open` holds one byte of
    /// shape information per level and the strings are scanned, not built.
    ///
    /// **It checks the same grammar the recursive parser does**, because "skipped" must not mean
    /// "anything": objects are `"key": value` pairs with commas between and a matching closer;
    /// arrays are values with commas between; strings go through the existing scanner, escapes
    /// and all; `true`, `false` and `null` must be spelled out; and numbers go through
    /// [`Cursor::number`]. Malformed input past the limit is refused exactly as it would be
    /// above the limit.
    ///
    /// Returns [`Json::TooDeep`] once the value it was called on closes.
    fn skip_value(&mut self) -> Result<Json> {
        /// What the scan is inside, and whether it has seen anything yet.
        enum Open {
            Array,
            Object,
        }
        let mut open: Vec<Open> = Vec::new();
        loop {
            self.skip_whitespace();
            // ---- one value
            match self.peek() {
                Some('{') => {
                    self.at += 1;
                    open.push(Open::Object);
                    self.skip_whitespace();
                    if self.peek() == Some('}') {
                        self.at += 1;
                        open.pop();
                    } else {
                        // A key must come first, and the value after it is the next iteration's.
                        self.skip_whitespace();
                        self.string()?;
                        self.skip_whitespace();
                        self.expect(':')?;
                        continue;
                    }
                }
                Some('[') => {
                    self.at += 1;
                    open.push(Open::Array);
                    self.skip_whitespace();
                    if self.peek() == Some(']') {
                        self.at += 1;
                        open.pop();
                    } else {
                        continue;
                    }
                }
                Some('"') => {
                    self.string()?;
                }
                Some('t' | 'f') => {
                    self.boolean()?;
                }
                Some('n') => {
                    self.null()?;
                }
                Some(ch) if ch.is_ascii_digit() || ch == '-' => {
                    self.number()?;
                }
                Some(ch) => return Err(schema(&format!("unexpected `{ch}`"))),
                None => return Err(schema("unexpected end of input")),
            }
            // ---- one value has closed: what follows it?
            loop {
                let Some(inside) = open.last() else {
                    // The value this scan was called on is complete.
                    return Ok(Json::TooDeep);
                };
                self.skip_whitespace();
                match (inside, self.next()) {
                    (Open::Array, Some(',')) | (Open::Object, Some(',')) => break,
                    (Open::Array, Some(']')) | (Open::Object, Some('}')) => {
                        open.pop();
                    }
                    (Open::Array, Some(ch)) => {
                        return Err(schema(&format!("expected `,` or `]`, found `{ch}`")));
                    }
                    (Open::Object, Some(ch)) => {
                        return Err(schema(&format!("expected `,` or `}}`, found `{ch}`")));
                    }
                    (Open::Array, None) => return Err(schema("unterminated array")),
                    (Open::Object, None) => return Err(schema("unterminated object")),
                }
            }
            // A comma inside an object means another key, and then its value.
            if matches!(open.last(), Some(Open::Object)) {
                self.skip_whitespace();
                self.string()?;
                self.skip_whitespace();
                self.expect(':')?;
            }
        }
    }

    /// Advance over one or more ASCII digits, reporting whether there were any.
    fn take_digits(&mut self) -> bool {
        let start = self.at;
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.at += 1;
        }
        self.at > start
    }
}

#[cfg(test)]
mod tests;
