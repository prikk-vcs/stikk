//! Prints the prikk minor-version range stikk currently targets, as `<floor> <ceiling>` on one line.
//!
//! Consumed by the real-binary integration suite's CI workflow (`.github/workflows/real-binary.yml`;
//! `TS-07`, RFC 019 §6) to decide which two prikk versions to `cargo install`, so the workflow never
//! hardcodes a version number that could drift from [`stikk_prikk::version`]'s own bounds — raising the
//! validated ceiling there widens what the suite installs and exercises automatically.

fn main() {
    let (floor, ceiling) = stikk_prikk::version::supported_minor_range();
    println!("{floor} {ceiling}");
}
