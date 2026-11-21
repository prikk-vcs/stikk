//! The palette's own contrast test (RFC 059 Handoff A §1.3).
//!
//! **The ratios are computed, never asserted by hand.** A later change to a value in `theme.rs` that
//! fell below 7 : 1 would pass review and fail here, which is the whole point of RFC 059's test plan:
//! the bar is checked by arithmetic on the values themselves.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// One row of [`MEASURED`]: a theme, a depth, and the four roles' worst-case ratios.
type MeasuredRow = (SchemeKind, Depth, [(&'static str, f64); 4]);

/// A depth's name, for the table this test prints. `Depth`'s own word for it arrives with the
/// Orientation row that shows it (RFC 059 decision 10).
fn depth_name(depth: Depth) -> &'static str {
    match depth {
        Depth::TrueColour => "24-bit colour",
        Depth::Ansi256 => "256 colours",
    }
}

/// The worst-case ratio of each role over its theme's reference backgrounds, at one depth.
fn worst(colour: Color, kind: SchemeKind) -> f64 {
    let Checkable::Known(rgb) = checkable(colour) else {
        panic!("{}: {colour:?} is not a colour stikk chose", kind.name());
    };
    kind.references()
        .iter()
        .map(|&bg| contrast_ratio(rgb, bg))
        .fold(f64::INFINITY, f64::min)
}

/// **`palette059.txt`'s own worst-case column**, by theme, depth and role — the report RFC 059
/// decision 3 chose the values from (`.git-exclude/reports/059-readable-colours/palette059.txt`).
///
/// Here so the cross-check is permanent rather than a thing someone did once: `theme.rs` and the
/// measurement agree **to two decimals**, or this test says which role drifted. If a value in
/// `theme.rs` is ever changed deliberately, `palette059.py` is re-run and this table moves with it.
const MEASURED: [MeasuredRow; 4] = [
    (
        SchemeKind::Dark,
        Depth::TrueColour,
        [
            ("dim", 7.20),
            ("accent", 7.21),
            ("ok", 7.25),
            ("warn", 7.23),
        ],
    ),
    (
        SchemeKind::Dark,
        Depth::Ansi256,
        [
            ("dim", 7.31),
            ("accent", 7.85),
            ("ok", 7.20),
            ("warn", 9.11),
        ],
    ),
    (
        SchemeKind::Light,
        Depth::TrueColour,
        [
            ("dim", 7.21),
            ("accent", 7.20),
            ("ok", 7.26),
            ("warn", 7.21),
        ],
    ),
    (
        SchemeKind::Light,
        Depth::Ansi256,
        [
            ("dim", 7.71),
            ("accent", 14.42),
            ("ok", 7.38),
            ("warn", 9.59),
        ],
    ),
];

/// Every colour stikk chooses, against every reference background of its theme, at both depths
/// (RFC 059 decision 3).
///
/// Prints the table it checked, so a reader of the review request sees the same numbers the assertion
/// saw.
#[test]
fn every_chosen_colour_reaches_seven_to_one_on_its_references() {
    let mut rows = 0_usize;
    for (kind, scheme) in [
        (SchemeKind::Dark, Scheme::dark()),
        (SchemeKind::Light, Scheme::light()),
    ] {
        for depth in [Depth::TrueColour, Depth::Ansi256] {
            let palette = scheme.for_depth(depth);
            println!(
                "== {} theme, {} ({} reference backgrounds)",
                kind.name(),
                depth_name(depth),
                kind.references().len()
            );
            for (role, colour) in palette.roles() {
                // `fg` is the terminal's own pair in both of these themes: stikk chose nothing, so
                // there is nothing of stikk's to hold to 7 : 1 (decision 2).
                if checkable(colour) == Checkable::TerminalsOwn {
                    println!("  {role:<7} the terminal's own colour");
                    continue;
                }
                let Checkable::Known(rgb) = checkable(colour) else {
                    panic!(
                        "{} {role}: {colour:?} is a colour only the terminal knows",
                        kind.name()
                    );
                };
                let per: Vec<String> = kind
                    .references()
                    .iter()
                    .map(|&bg| format!("{:.2}", contrast_ratio(rgb, bg)))
                    .collect();
                let w = worst(colour, kind);
                println!(
                    "  {role:<7} #{:02x}{:02x}{:02x}  worst {w:.2}:1  [{}]",
                    rgb.0,
                    rgb.1,
                    rgb.2,
                    per.join(", ")
                );
                for &bg in kind.references() {
                    let ratio = contrast_ratio(rgb, bg);
                    assert!(
                        ratio >= MINIMUM_CONTRAST,
                        "{} {role} ({colour:?}) is {ratio:.2}:1 on #{:02x}{:02x}{:02x}, below {MINIMUM_CONTRAST}:1",
                        kind.name(),
                        bg.0,
                        bg.1,
                        bg.2,
                    );
                    rows += 1;
                }
            }
        }
    }
    // 4 roles x (6 + 3) references x 2 depths.
    assert_eq!(rows, 72, "every role x reference x depth was checked");
}

/// **The values in `theme.rs` are the measured ones**, to two decimals (Handoff A §1.3).
#[test]
fn the_palette_agrees_with_palette059_to_two_decimals() {
    for (kind, depth, expected) in MEASURED {
        let scheme = match kind {
            SchemeKind::Dark => Scheme::dark(),
            SchemeKind::Light => Scheme::light(),
        };
        let palette = scheme.for_depth(depth);
        for (role, measured) in expected {
            let colour = palette
                .roles()
                .into_iter()
                .find_map(|(name, colour)| (name == role).then_some(colour))
                .expect("a role in the palette");
            let here = worst(colour, kind);
            assert!(
                (here - measured).abs() < 0.005,
                "{} {} {role}: theme.rs is {here:.2}:1, palette059.txt says {measured:.2}:1",
                kind.name(),
                depth_name(depth),
            );
        }
    }
}

/// The 256-colour values are **chosen**, and the standard xterm table is what makes that meaningful
/// (F3): index 18 is `#000087`, 231 is white, 16 is black, and 251 is on the grey ramp's cube side.
#[test]
fn the_xterm_table_is_the_standard_one() {
    assert_eq!(xterm_rgb(16), Some((0, 0, 0)));
    assert_eq!(xterm_rgb(18), Some((0, 0, 0x87)));
    assert_eq!(xterm_rgb(22), Some((0, 0x5f, 0)));
    assert_eq!(xterm_rgb(88), Some((0x87, 0, 0)));
    assert_eq!(xterm_rgb(114), Some((0x87, 0xd7, 0x87)));
    assert_eq!(xterm_rgb(117), Some((0x87, 0xd7, 0xff)));
    assert_eq!(xterm_rgb(222), Some((0xff, 0xd7, 0x87)));
    assert_eq!(xterm_rgb(231), Some((0xff, 0xff, 0xff)));
    assert_eq!(xterm_rgb(239), Some((0x4e, 0x4e, 0x4e)));
    assert_eq!(xterm_rgb(251), Some((0xc6, 0xc6, 0xc6)));
    // The grey ramp: 232 is `8 + 10 x 0`, 255 is `8 + 10 x 23`.
    assert_eq!(xterm_rgb(232), Some((8, 8, 8)));
    assert_eq!(xterm_rgb(255), Some((238, 238, 238)));
    // **The low sixteen are the terminal's own**, with no known value — the same unknown as a named
    // colour, and the reason RFC 059 chooses no index below 16.
    for index in 0..16 {
        assert_eq!(xterm_rgb(index), None, "index {index}");
    }
}

/// The formula is WCAG's: black on white is 21 : 1, and a colour against itself is 1 : 1.
#[test]
fn the_contrast_formula_is_wcags() {
    let black = (0, 0, 0);
    let white = (0xff, 0xff, 0xff);
    assert!((contrast_ratio(black, white) - 21.0).abs() < 0.001);
    assert!((contrast_ratio(white, black) - 21.0).abs() < 0.001);
    assert!((contrast_ratio(black, black) - 1.0).abs() < 0.001);
    assert!((relative_luminance(white) - 1.0).abs() < 0.001);
    assert!(relative_luminance(black).abs() < 0.001);
}

/// **No named ANSI colour is left in the palette** (Handoff A §1.2, trap 5). A named colour is
/// whatever the terminal's scheme says it is, which is the contrast stikk cannot compute (F1).
#[test]
fn no_palette_role_is_a_colour_only_the_terminal_knows() {
    for scheme in [Scheme::dark(), Scheme::light()] {
        for depth in [Depth::TrueColour, Depth::Ansi256] {
            let palette = scheme.for_depth(depth);
            for (role, colour) in palette.roles() {
                assert_ne!(
                    checkable(colour),
                    Checkable::TerminalsGuess,
                    "{} {role} at {}: {colour:?}",
                    palette.kind.name(),
                    depth_name(depth)
                );
            }
            if let Some(bg) = palette.bg {
                assert_ne!(
                    checkable(bg),
                    Checkable::TerminalsGuess,
                    "{} bg",
                    palette.kind.name()
                );
            }
        }
    }
}
