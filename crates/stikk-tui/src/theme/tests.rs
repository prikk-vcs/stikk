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
            // `plain` chooses no colour and `high-contrast` paints its own pair, so `palette059.txt`
            // has no row for either.
            SchemeKind::Plain | SchemeKind::HighContrast => continue,
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

// ---------------------------------------------------------------------------------------------
// The resolver (RFC 059 decisions 4, 5, 6 and 9; Handoff A §2)
//
// **Every one of these runs with no terminal and no environment variable.** `resolve` reads
// nothing: `crate::run` does the reading and hands it over as an `Environment`, which is what lets
// each band below be an ordinary unit test (trap 2).
// ---------------------------------------------------------------------------------------------

/// `system`, with nothing else said.
fn system() -> Environment {
    Environment::configured(ConfigTheme::System)
}

/// What the terminal answered: which way round it is, and the background itself.
fn answered(light: bool, background: (u8, u8, u8)) -> Option<Answer> {
    Some(Answer { light, background })
}

/// Decision 4: `COLORTERM`, `WT_SESSION` and the platform pick the depth, and a terminal that
/// promises nothing gets the 256-colour values.
#[test]
fn the_depth_follows_what_the_terminal_promised() {
    let truecolor = Environment {
        colorterm: Some("truecolor".to_string()),
        ..system()
    };
    let bit24 = Environment {
        colorterm: Some("24bit".to_string()),
        ..system()
    };
    let windows_terminal = Environment {
        wt_session: true,
        ..system()
    };
    let windows = Environment {
        windows: true,
        ..system()
    };
    for (what, environment) in [
        ("COLORTERM=truecolor", truecolor),
        ("COLORTERM=24bit", bit24),
        ("WT_SESSION set", windows_terminal),
        ("Windows", windows),
    ] {
        assert_eq!(
            resolve(&environment).depth,
            Depth::TrueColour,
            "{what} promises 24-bit colour"
        );
    }
    // **Nothing promised is 256 colours**, which is macOS Terminal: it sets no `COLORTERM` (F3).
    assert_eq!(resolve(&system()).depth, Depth::Ansi256);
    // A `COLORTERM` that says something else is not a promise.
    let other = Environment {
        colorterm: Some("8bit".to_string()),
        ..system()
    };
    assert_eq!(resolve(&other).depth, Depth::Ansi256);
    // The depth is decided for an explicit theme too — it is the terminal's, not the theme's.
    let dark_on_windows = Environment {
        windows: true,
        ..Environment::configured(ConfigTheme::Dark)
    };
    assert_eq!(resolve(&dark_on_windows).depth, Depth::TrueColour);
}

/// Decision 5: an answer gives `light` or `dark`, and says so.
#[test]
fn an_answer_picks_the_palette_and_is_named_as_the_reason() {
    let light = Environment {
        answer: answered(true, (0xff, 0xff, 0xff)),
        ..system()
    };
    let resolved = resolve(&light);
    assert_eq!(resolved.palette.kind, SchemeKind::Light);
    assert_eq!(resolved.reason, Reason::Detected { light: true });
    assert_eq!(resolved.fell_back, 0);

    let dark = Environment {
        answer: answered(false, (0x2e, 0x34, 0x40)),
        ..system()
    };
    let resolved = resolve(&dark);
    assert_eq!(resolved.palette.kind, SchemeKind::Dark);
    assert_eq!(resolved.reason, Reason::Detected { light: false });
    assert_eq!(resolved.fell_back, 0);
}

/// Decision 5: no answer, but `COLORFGBG`'s last field says which way round the terminal is — and
/// **anything else in that field is no answer at all**.
#[test]
fn colorfgbg_decides_only_where_it_says_something() {
    for value in ["0;15", "15", "default;15", "0;default;7", " 7 ", "7"] {
        let environment = Environment {
            colorfgbg: Some(value.to_string()),
            ..system()
        };
        let resolved = resolve(&environment);
        assert_eq!(
            resolved.palette.kind,
            SchemeKind::Light,
            "COLORFGBG={value:?} names a light background"
        );
        assert_eq!(resolved.reason, Reason::FromColorFgBg { light: true });
    }
    for value in ["15;0", "0", "15;8", "7;6", "1"] {
        let environment = Environment {
            colorfgbg: Some(value.to_string()),
            ..system()
        };
        let resolved = resolve(&environment);
        assert_eq!(
            resolved.palette.kind,
            SchemeKind::Dark,
            "COLORFGBG={value:?} names a dark background"
        );
        assert_eq!(resolved.reason, Reason::FromColorFgBg { light: false });
    }
    // **Not an answer:** a colour index above 15, a word, an empty field, no field at all. A
    // terminal that writes one of these has not said which way round it is.
    for value in [
        "0;9",
        "0;33",
        "0;default",
        "15;",
        "",
        ";",
        "0;15;x",
        "0;0x7",
    ] {
        let environment = Environment {
            colorfgbg: Some(value.to_string()),
            ..system()
        };
        let resolved = resolve(&environment);
        assert_eq!(
            resolved.palette.kind,
            SchemeKind::Plain,
            "COLORFGBG={value:?} says nothing stikk can use"
        );
        assert_eq!(resolved.reason, Reason::NoAnswer);
    }
}

/// Decision 5: no answer at all is `plain` — safe on any background.
#[test]
fn no_answer_at_all_is_plain() {
    let resolved = resolve(&system());
    assert_eq!(resolved.palette.kind, SchemeKind::Plain);
    assert_eq!(resolved.reason, Reason::NoAnswer);
    for (role, colour) in resolved.palette.roles() {
        assert_eq!(colour, Color::Reset, "{role} is the terminal's own");
    }
    assert_eq!(resolved.palette.bg, None, "plain paints no background");
}

/// Decision 9: `NO_COLOR` *"present and not an empty string"*.
#[test]
fn no_color_is_honoured_when_it_is_set_to_anything_at_all() {
    for value in ["1", "0", "no", "true", " "] {
        let environment = Environment {
            no_color: Some(value.to_string()),
            ..system()
        };
        let resolved = resolve(&environment);
        assert_eq!(
            resolved.palette.kind,
            SchemeKind::Plain,
            "NO_COLOR={value:?} is set"
        );
        assert_eq!(resolved.reason, Reason::NoColor);
        // **And no query is wanted**: the answer could not change what this resolves to.
        assert!(
            !resolved.wants_an_answer,
            "NO_COLOR={value:?} sends no query"
        );
    }
    // **Empty is not set**, so `system` goes on to ask.
    let empty = Environment {
        no_color: Some(String::new()),
        ..system()
    };
    let resolved = resolve(&empty);
    assert!(resolved.wants_an_answer, "an empty NO_COLOR is ignored");
    assert_eq!(resolved.reason, Reason::NoAnswer);
}

/// Decision 9: **an explicit theme in the config wins over `NO_COLOR`**, as no-color.org's own FAQ
/// asks — *"user-level configuration files … should override the `NO_COLOR` environment variable"*.
#[test]
fn an_explicit_theme_wins_over_no_color() {
    for (theme, kind) in [
        (ConfigTheme::Dark, SchemeKind::Dark),
        (ConfigTheme::Light, SchemeKind::Light),
    ] {
        let environment = Environment {
            no_color: Some("1".to_string()),
            ..Environment::configured(theme)
        };
        let resolved = resolve(&environment);
        assert_eq!(resolved.palette.kind, kind);
        assert_eq!(resolved.reason, Reason::FromConfig(kind));
        assert!(!resolved.wants_an_answer);
    }
}

/// **Two themes ask the terminal, and no other does** — `system` (decision 5) and `high-contrast`
/// (decision 8). This is the same question `crate::run` asks before it calls `detect`, so a theme
/// that must not query cannot start querying without this test failing.
#[test]
fn system_and_high_contrast_ask_the_terminal_and_nothing_else_does() {
    assert!(resolve(&system()).wants_an_answer, "system asks");
    // **`high-contrast` asks as well**, and needs no answer: the answer only decides which way round
    // its own pair is painted.
    assert!(
        resolve(&Environment::configured(ConfigTheme::HighContrast)).wants_an_answer,
        "high-contrast asks"
    );
    // **And it asks under `NO_COLOR` too**, because an explicit theme in the config wins over
    // `NO_COLOR` (decision 9): the answer can still change which way round the pair goes, so it is
    // still worth asking for. A query is not colour output.
    let high_contrast_with_no_color = Environment {
        no_color: Some("1".to_string()),
        ..Environment::configured(ConfigTheme::HighContrast)
    };
    assert!(
        resolve(&high_contrast_with_no_color).wants_an_answer,
        "high-contrast asks under NO_COLOR as well"
    );

    // **Every theme that decides without the terminal sends nothing** — `plain` included, which
    // chooses no colour at all and so has nothing an answer could change.
    for theme in [ConfigTheme::Dark, ConfigTheme::Light, ConfigTheme::Plain] {
        assert!(
            !resolve(&Environment::configured(theme)).wants_an_answer,
            "{theme:?} sends no query"
        );
    }
    // `system` with `NO_COLOR` resolves to `plain` whatever the terminal would say, so it does not ask.
    let no_color = Environment {
        no_color: Some("1".to_string()),
        ..system()
    };
    assert!(
        !resolve(&no_color).wants_an_answer,
        "system under NO_COLOR sends no query"
    );
}

/// **Decision 6's guard**, on the two backgrounds the palette's references do not cover: a mid-grey,
/// where nothing stikk chose can reach 7 : 1, and a tinted dark scheme, where everything still does.
#[test]
fn the_guard_sends_a_role_back_to_the_terminals_own_colour_below_seven_to_one() {
    // A mid-grey is the worst background there is: both palettes are built against backgrounds that
    // are decidedly one thing or the other, and against this one every role falls.
    for (light, kind) in [(false, SchemeKind::Dark), (true, SchemeKind::Light)] {
        for depth in [
            ("", Depth::Ansi256),
            ("COLORTERM=truecolor", Depth::TrueColour),
        ] {
            let environment = Environment {
                answer: answered(light, (0x80, 0x80, 0x80)),
                colorterm: (!depth.0.is_empty()).then(|| "truecolor".to_string()),
                ..system()
            };
            let resolved = resolve(&environment);
            assert_eq!(resolved.depth, depth.1);
            assert_eq!(resolved.palette.kind, kind, "the detected palette is kept");
            assert_eq!(
                resolved.fell_back,
                4,
                "{} at {:?} on #808080: every chosen role falls",
                kind.name(),
                depth.1
            );
            for (role, colour) in resolved.palette.roles() {
                assert_eq!(
                    colour,
                    Color::Reset,
                    "{} {role} fell back to the terminal's own colour",
                    kind.name()
                );
            }
            // **The reason is still the answer**, not `plain`: the terminal did answer, and what the
            // row has to say is that every role fell back on top of that (decision 10).
            assert_eq!(resolved.reason, Reason::Detected { light });
        }
    }

    // A tinted dark scheme none of the references includes. The dark palette clears 7 : 1 on it at
    // both depths, so nothing falls and nothing is dimmed for no reason.
    for depth in [Depth::Ansi256, Depth::TrueColour] {
        let environment = Environment {
            answer: answered(false, (0x3b, 0x2f, 0x4a)),
            colorterm: matches!(depth, Depth::TrueColour).then(|| "truecolor".to_string()),
            ..system()
        };
        let resolved = resolve(&environment);
        assert_eq!(resolved.depth, depth);
        assert_eq!(resolved.fell_back, 0, "#3b2f4a at {depth:?}");
        assert_eq!(
            resolved.palette,
            Scheme::dark().for_depth(depth),
            "the palette is untouched at {depth:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The Orientation row (RFC 059 decision 10; Handoff A §4)
// ---------------------------------------------------------------------------------------------

/// **One reason, one sentence** — every way the palette can have been decided, in the words decision
/// 10 gives it.
#[test]
fn the_colours_row_says_why_this_palette_and_at_which_depth() {
    let detected_light = Environment {
        answer: answered(true, (0xff, 0xff, 0xff)),
        ..system()
    };
    let detected_dark = Environment {
        answer: answered(false, (0x1e, 0x1e, 0x1e)),
        ..system()
    };
    let fgbg_light = Environment {
        colorfgbg: Some("0;15".to_string()),
        ..system()
    };
    let fgbg_dark = Environment {
        colorfgbg: Some("15;0".to_string()),
        ..system()
    };
    let no_color = Environment {
        no_color: Some("1".to_string()),
        ..system()
    };
    for (environment, expected) in [
        (detected_light, "light background, detected · 256 colours"),
        (detected_dark, "dark background, detected · 256 colours"),
        (fgbg_light, "light background, from COLORFGBG · 256 colours"),
        (fgbg_dark, "dark background, from COLORFGBG · 256 colours"),
        (
            system(),
            "no answer from the terminal, so plain colours · 256 colours",
        ),
        (no_color, "plain, because NO_COLOR is set · 256 colours"),
        (
            Environment::configured(ConfigTheme::Dark),
            "dark, from the config · 256 colours",
        ),
        (
            Environment::configured(ConfigTheme::Light),
            "light, from the config · 256 colours",
        ),
        (
            Environment::configured(ConfigTheme::Plain),
            "plain, from the config · 256 colours",
        ),
        (
            Environment::configured(ConfigTheme::HighContrast),
            "high contrast, from the config · 256 colours",
        ),
    ] {
        assert_eq!(resolve(&environment).colours_row(), expected);
    }

    // **The depth is the terminal's, so it changes under every reason**: the same config on a
    // terminal that promises 24-bit colour says so.
    let on_truecolour = Environment {
        colorterm: Some("truecolor".to_string()),
        ..Environment::configured(ConfigTheme::Dark)
    };
    assert_eq!(
        resolve(&on_truecolour).colours_row(),
        "dark, from the config · 24-bit colour"
    );
}

/// Decision 10's last clause: **how many roles fell back**, and only when some did.
#[test]
fn the_colours_row_counts_the_roles_that_fell_back() {
    // A mid-grey background: every chosen colour falls (decision 6), and the row says so on top of
    // the reason, which is still the terminal's answer.
    let grey = Environment {
        answer: answered(false, (0x80, 0x80, 0x80)),
        ..system()
    };
    assert_eq!(
        resolve(&grey).colours_row(),
        "dark background, detected · 256 colours · 4 colours fell back to the terminal's own, \
         below 7:1 on that background"
    );
    // One is singular, and none says nothing at all — the counts themselves are the guard's own
    // test; this is about the sentence.
    let one = Resolved {
        fell_back: 1,
        ..resolve(&grey)
    };
    assert_eq!(
        one.colours_row(),
        "dark background, detected · 256 colours · 1 colour fell back to the terminal's own, \
         below 7:1 on that background"
    );
    let none = Resolved {
        fell_back: 0,
        ..resolve(&grey)
    };
    assert_eq!(
        none.colours_row(),
        "dark background, detected · 256 colours"
    );
}
