//! Prints the prikk **minor just below the band where prikk refuses the switch itself**, as one
//! number on one line — `58` while [`stikk_core::switch::PRIKK_REFUSES_THE_SWITCH_FROM_MINOR`] is
//! 59.
//!
//! Consumed by `.github/workflows/real-binary.yml` to decide which third prikk to install for the
//! suite's `STIKK_TEST_PRIKK_BELOW_SWITCH_BIN` slot (RFC 074 decision 4), so the workflow never
//! hardcodes a version that could drift from the constant that defines the band — moving that
//! constant moves what CI installs.
//!
//! **A separate example, not a third word from `print_version_matrix`.** That one prints
//! `<floor> <ceiling>` and the workflow reads it with `read -r FLOOR CEILING`, where a third word
//! would land silently inside `CEILING` and install the wrong binary. Two numbers from two
//! commands cannot do that to each other.

fn main() {
    println!(
        "{}",
        stikk_core::switch::PRIKK_REFUSES_THE_SWITCH_FROM_MINOR.saturating_sub(1)
    );
}
