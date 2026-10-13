//! Tests for the scoped JSON reader.
//!
//! **The point of most of these is that nothing panics.** This crate forbids `unwrap`/`expect`/
//! indexing in product code, but a reader over hostile input needs more than a lint: these feed it
//! truncated, mis-nested, over-deep and malformed text and require an `Err`, because a front-end that
//! panicked a user's terminal on a malformed read would be worse than not reading at all (`NFR-P01`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;

fn obj(text: &str) -> Json {
    match parse(text) {
        Ok(value) => value,
        Err(e) => panic!("expected valid JSON, got {e}"),
    }
}

#[test]
fn reads_the_shapes_prikks_reports_use() {
    let value = obj(r#"{"a": "x", "b": 12, "c": true, "d": null, "e": [1, 2], "f": {"g": "y"}}"#);
    assert_eq!(value.str_field("a").ok(), Some("x"));
    assert_eq!(value.u64_field("b").ok(), Some(12));
    assert_eq!(value.bool_field("c").ok(), Some(true));
    assert_eq!(value.opt_str_field("d").ok(), Some(None));
    assert_eq!(value.array_field("e").map(<[Json]>::len).ok(), Some(2));
    assert_eq!(
        value.get("f").and_then(|f| f.str_field("g").ok()),
        Some("y")
    );
}

#[test]
fn an_absent_field_and_a_null_field_differ_for_required_reads_and_not_for_optional_ones() {
    let value = obj(r#"{"here": null}"#);
    // Optional: both spellings of "no value" agree.
    assert_eq!(value.opt_str_field("here").ok(), Some(None));
    assert_eq!(value.opt_str_field("absent").ok(), Some(None));
    // Required: a null is a schema mismatch, an absence is a missing field, and the messages differ
    // because the fixes differ.
    assert!(value.str_field("here").is_err());
    assert!(value.str_field("absent").is_err());
}

#[test]
fn a_wrong_type_names_the_field_and_what_it_found() {
    let value = obj(r#"{"count": "twelve"}"#);
    let err = value
        .u64_field("count")
        .expect_err("a string is not a number");
    let text = err.to_string();
    assert!(text.contains("count"), "{text}");
    assert!(text.contains("string"), "{text}");
}

#[test]
fn escapes_and_unicode_round_trip() {
    let value = obj(r#"{"s": "a\"b\\c\ndAé"}"#);
    assert_eq!(value.str_field("s").ok(), Some("a\"b\\c\ndAé"));
}

#[test]
fn a_surrogate_pair_becomes_one_character_and_a_lone_surrogate_becomes_a_replacement() {
    assert_eq!(obj(r#""😀""#), Json::String("😀".to_string()));
    // An unpaired high surrogate is not worth refusing a whole report over: it degrades to U+FFFD,
    // which `text::inert` would render anyway.
    assert_eq!(obj(r#""\ud83d""#), Json::String("\u{FFFD}".to_string()));
}

/// **No floats.** Every number in prikk's three schemas is a count or a sequence number, and quietly
/// truncating `1.5` into a `u64` would be the reader inventing a value stikk then displays as fact.
#[test]
fn a_fractional_number_is_refused_rather_than_truncated() {
    let err = parse(r#"{"n": 1.5}"#).expect_err("floats are not part of these schemas");
    assert!(err.to_string().contains("integer"), "{err}");
}

#[test]
fn an_oversized_integer_is_refused_rather_than_wrapped() {
    assert!(parse(r#"{"n": 99999999999999999999999}"#).is_err());
}

/// Every one of these must return `Err`. **None may panic** — that is the whole assertion.
#[test]
fn malformed_input_errors_and_never_panics() {
    for text in [
        "",
        "   ",
        "{",
        "}",
        "[",
        "[1,",
        r#"{"a""#,
        r#"{"a":}"#,
        r#"{"a": 1,}"#,
        r#"{"a" 1}"#,
        r#""unterminated"#,
        r#""bad \q escape""#,
        r#""truncated \u12""#,
        "tru",
        "nul",
        "-1",
        "+1",
        ".5",
        r#"{"a": 1} trailing"#,
        "\u{0}",
        "\u{1b}[2J",
    ] {
        assert!(
            parse(text).is_err(),
            "expected an error for {text:?}, got a value"
        );
    }
}

/// Depth is bounded, so deeply nested input is refused rather than recursing until the stack ends.
#[test]
fn nesting_past_the_bound_errors_rather_than_overflowing_the_stack() {
    let deep = format!("{}{}", "[".repeat(200), "]".repeat(200));
    assert!(parse(&deep).is_err());
    // And a depth the schemas actually use is fine.
    assert!(parse(&format!("{}{}", "[".repeat(8), "]".repeat(8))).is_ok());
}

#[test]
fn empty_containers_are_values_not_errors() {
    assert_eq!(obj("{}"), Json::Object(Vec::new()));
    assert_eq!(obj("[]"), Json::Array(Vec::new()));
    assert_eq!(
        obj(r#"{"tags": []}"#)
            .array_field("tags")
            .map(<[Json]>::len)
            .ok(),
        Some(0)
    );
}

/// The display renderer (RFC 035 decision 5): prikk's keys in prikk's order, values as given, and
/// control characters written as JSON escapes rather than passed through as bytes.
#[test]
fn render_writes_prikks_keys_in_prikks_order() {
    let value = obj(r#"{"zebra": 1, "apple": {"inner": [true, null, "x"]}, "empty": {}}"#);
    assert_eq!(
        value.render(),
        "{\n  \"zebra\": 1,\n  \"apple\": {\n    \"inner\": [\n      true,\n      null,\n      \
         \"x\"\n    ]\n  },\n  \"empty\": {}\n}",
        "`apple` must not sort above `zebra`: the raw view shows the report's own shape"
    );
}

/// A string value with a newline, a quote, a backslash and an escape byte comes back as JSON — which
/// is also why a control character in repository content cannot reach the terminal from here, before
/// `inert` ever sees it (`C-T2a`).
#[test]
fn render_escapes_what_json_requires_and_every_other_control_character() {
    let value = obj("{\"text\": \"a\\nb\\\"c\\\\d\\u001be\\tf\"}");
    assert_eq!(
        value.render(),
        "{\n  \"text\": \"a\\nb\\\"c\\\\d\\u001be\\tf\"\n}"
    );
    // And it is JSON this very reader accepts again, which is the cheapest proof the escaping is right.
    assert_eq!(parse(&value.render()).ok(), Some(value));
}

/// Review v2 §2: the raw view escapes what does not show as itself, not only category `Cc` — and the
/// result is still JSON **this reader** accepts, surrogate pairs included. Rendering and parsing are
/// the two halves that have to agree for the raw view to be what it claims to be.
#[test]
fn render_escapes_every_character_that_does_not_show_as_itself() {
    let value = Json::Object(vec![(
        "text".to_string(),
        Json::String("a\u{202e}b\u{200b}c\u{a0}d\u{e0001}e日本".to_string()),
    )]);
    assert_eq!(
        value.render(),
        "{\n  \"text\": \"a\\u202eb\\u200bc\\u00a0d\\udb40\\udc01e日本\"\n}",
        "visible non-ASCII stays; a bidi override, a zero-width space, a non-breaking space and a \
         tag character do not"
    );
    // The astral escape comes back as the one character it spells, not as two unpaired surrogates.
    assert_eq!(parse(&value.render()).ok(), Some(value));
}
