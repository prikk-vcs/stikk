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

/// **No floats are read.** Every number stikk reads from prikk's schemas is a count or a sequence
/// number, and quietly truncating `1.5` into a `u64` would be the reader inventing a value stikk
/// then displays as fact.
///
/// **RFC 064 moved where that refusal happens, not whether it does**: the value is now held as
/// [`Json::OtherNumber`] so that a field stikk skips costs nothing, and the refusal is the
/// accessor's — naming the field, which it could not before.
#[test]
fn a_fractional_number_is_refused_rather_than_truncated() {
    let value = obj(r#"{"n": 1.5}"#);
    let err = value
        .u64_field("n")
        .expect_err("floats are not part of these schemas");
    assert!(err.to_string().contains("field `n`"), "{err}");
    assert!(
        err.to_string().contains("a number stikk does not hold"),
        "{err}"
    );
}

#[test]
fn an_oversized_integer_is_refused_rather_than_wrapped() {
    let value = obj(r#"{"n": 99999999999999999999999}"#);
    let err = value.u64_field("n").expect_err("it does not fit a u64");
    assert!(err.to_string().contains("field `n`"), "{err}");
    // **Never wrapped, and never 0** (RFC 064 §5 trap 3).
    assert!(value.u64_field("n").is_err());
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
        // `-1` moved out of this list at RFC 064: it is valid JSON, and the reader now holds it
        // (`every_valid_json_number_reads_into_its_variant`). What is still not a number is here.
        "-",
        "-a",
        "1.",
        "1e",
        "1e+",
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

// ---------------------------------------------------------------------------------------------
// RFC 064: every valid number is read; only the ones stikk reads are refused (decisions 1, 3, 4).
// ---------------------------------------------------------------------------------------------

/// **Each valid JSON number reads, and lands in the right variant.** A non-negative integer that
/// fits a `u64` is the one stikk stores; everything else is held as its own text.
#[test]
fn every_valid_json_number_reads_into_its_variant() {
    for (text, want) in [
        ("0", Json::Number(0)),
        ("12", Json::Number(12)),
        ("18446744073709551615", Json::Number(u64::MAX)),
        // The documented leading-zero laxity, unchanged by RFC 064.
        ("007", Json::Number(7)),
        ("-1", Json::OtherNumber("-1".to_string())),
        ("-0", Json::OtherNumber("-0".to_string())),
        ("0.5", Json::OtherNumber("0.5".to_string())),
        ("-0.5", Json::OtherNumber("-0.5".to_string())),
        ("1e3", Json::OtherNumber("1e3".to_string())),
        ("1E3", Json::OtherNumber("1E3".to_string())),
        ("1e+3", Json::OtherNumber("1e+3".to_string())),
        ("1e-3", Json::OtherNumber("1e-3".to_string())),
        ("1.5e-3", Json::OtherNumber("1.5e-3".to_string())),
        (
            "18446744073709551616",
            Json::OtherNumber("18446744073709551616".to_string()),
        ),
        (
            "999999999999999999999999999999",
            Json::OtherNumber("999999999999999999999999999999".to_string()),
        ),
    ] {
        assert_eq!(
            parse(text).unwrap_or_else(|e| panic!("{text} should read: {e}")),
            want,
            "{text}"
        );
        // ...and the same inside a document, where the scanner is reached through `value`.
        let inside = format!(r#"{{"x": {text}}}"#);
        assert_eq!(
            obj(&inside).get("x"),
            Some(&want),
            "{text}, inside an object"
        );
    }
}

/// **Text that is not a JSON number is still refused** — RFC 064 widens what is read, not what is
/// accepted.
#[test]
fn text_that_is_not_a_json_number_is_still_refused() {
    for text in [
        "-", "-a", "1.", "-1.", ".5", "1e", "1e+", "1e-", "+1", "1.e3", "--1",
    ] {
        let err = parse(text).err().unwrap_or_else(|| {
            panic!("{text} is not a JSON number and must be refused");
        });
        assert_eq!(err.class(), "environment", "{text}: {err}");
    }
}

/// **`u64_field` on a number stikk cannot hold names the field**, and says what kind of number it
/// is (RFC 064 decision 3) — *should be a number, got a number* would tell a reader nothing.
#[test]
fn u64_field_on_a_number_stikk_does_not_hold_names_the_field() {
    for text in ["-1", "0.5", "1e3", "18446744073709551616"] {
        let value = obj(&format!(r#"{{"count": {text}}}"#));
        let err = value
            .u64_field("count")
            .err()
            .unwrap_or_else(|| panic!("{text} must not be read as a count"));
        let said = err.to_string();
        assert!(said.contains("field `count`"), "{text}: {said}");
        assert!(
            said.contains("a number stikk does not hold"),
            "{text}: {said}"
        );
        assert!(
            said.contains("larger than 2^64 - 1"),
            "{text}: the sentence says which kinds: {said}"
        );
    }
}

/// **Every other accessor names it too**, through `kind_of` — and none of them turns it into a
/// value (RFC 064 §5 trap 3).
#[test]
fn every_accessor_refuses_a_number_stikk_does_not_hold_and_names_it() {
    let value = obj(r#"{"x": -1}"#);
    for said in [
        value.str_field("x").err().map(|e| e.to_string()),
        value.bool_field("x").err().map(|e| e.to_string()),
        value.array_field("x").err().map(|e| e.to_string()),
        value.opt_str_field("x").err().map(|e| e.to_string()),
    ] {
        let said = said.unwrap_or_else(|| panic!("every accessor must refuse it"));
        assert!(said.contains("field `x`"), "{said}");
        assert!(said.contains("a number stikk does not hold"), "{said}");
    }
    // `opt_str_field` does **not** read it as absent, which would be the reader inventing a value.
    assert!(value.opt_str_field("x").is_err());
}

/// **The raw view writes it as its own text** (RFC 064 decision 4), and the text is ASCII digits
/// and the number grammar's punctuation — nothing an escape table would have to touch.
#[test]
fn the_raw_view_writes_a_number_stikk_does_not_hold_as_its_text() {
    let value = obj(r#"{"a": -1, "b": 0.5, "c": 1e+3, "d": 18446744073709551616}"#);
    let rendered = value.render();
    for text in ["-1", "0.5", "1e+3", "18446744073709551616"] {
        assert!(rendered.contains(text), "{text}: {rendered}");
    }
    // Not quoted as a string, which would change what the raw view says the report holds.
    assert!(!rendered.contains("\"-1\""), "{rendered}");
    assert!(
        rendered
            .chars()
            .all(|ch| !ch.is_control() || ch == '\n' || ch == '\t'),
        "{rendered}"
    );
}
