//! Tests for identity newtypes (design `stikk-04` TS-01).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

const VALID: &str = "cc86f0ec0fedc824d3e6b1481a4a43abcdbf25626b99eae5174aa295d9146b6f";

#[test]
fn parses_a_valid_object_id() {
    let id = ObjectId::parse(VALID).expect("valid id");
    assert_eq!(id.as_str(), VALID);
    assert_eq!(id.abbreviated(8), "cc86f0ec");
}

#[test]
fn rejects_wrong_length() {
    assert!(ObjectId::parse("cc86f0ec").is_err());
    assert!(ObjectId::parse(&format!("{VALID}0")).is_err());
}

#[test]
fn rejects_uppercase_and_non_hex() {
    // prikk only ever emits lowercase hex; anything else means we misread its output.
    assert!(ObjectId::parse(&VALID.to_uppercase()).is_err());
    let non_hex = "zz86f0ec0fedc824d3e6b1481a4a43abcdbf25626b99eae5174aa295d9146b6f";
    assert!(ObjectId::parse(non_hex).is_err());
}

#[test]
fn ref_names_reject_empty_and_control_chars() {
    assert!(RefName::parse("").is_err());
    assert!(RefName::parse("heads/ma\u{7}in").is_err());
    assert!(RefName::parse("heads/main").is_ok());
}

#[test]
fn received_refs_are_recognized() {
    assert!(RefName::parse("remotes/heads/main").unwrap().is_received());
    assert!(!RefName::parse("heads/main").unwrap().is_received());
}
