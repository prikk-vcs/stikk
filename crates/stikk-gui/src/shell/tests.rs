//! The shell's own tests: that a screen is drawn at all, and that every banner is seen (rule 9).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_core::present::OperationContext;
use stikk_gui_design::Preset;

use crate::harness::{self, Screen};

/// **The machinery itself**: the app draws, the tree can be read, and the pixels come out.
///
/// Not a rule's test — the one that says the harness is measuring something, so a rule's test that
/// finds nothing is a finding and not a broken instrument.
#[test]
fn the_harness_measures_a_drawn_screen() {
    let settings = harness::drawing(Preset::LightAaa);
    let size = harness::logical(harness::WINDOW, 1.0);
    let screen = Screen::open(harness::on_orientation(), &settings, size);

    let texts = screen.texts();
    assert!(
        texts.len() > 5,
        "Orientation drew {} texts, which is too few to be the screen:\n{:#?}",
        texts.len(),
        texts
    );
    assert!(
        texts.iter().any(|t| t.content.contains("prikk 0.59.0")),
        "the version line is on the screen:\n{:#?}",
        texts.iter().map(|t| &t.content).collect::<Vec<_>>()
    );
    let scrollables = screen.scrollables();
    assert!(
        !scrollables.is_empty(),
        "the body is in a scrollable, so a long screen can be read"
    );

    let picture = harness::picture(
        harness::on_orientation,
        &settings,
        size,
        iced::theme::Mode::Light,
        None,
    );
    assert_eq!(picture.width, size.width.round() as u32);
    assert_eq!(picture.height, size.height.round() as u32);
    assert!(
        picture.rgba.iter().any(|&byte| byte != 0),
        "the picture is not blank"
    );
    println!(
        "the harness drew {} texts, {} controls, {} scrollables, and a {}x{} picture",
        texts.len(),
        screen.controls().len(),
        scrollables.len(),
        picture.width,
        picture.height
    );
}

/// **Rule 9: every banner the program sets is drawn.**
///
/// RFC 065's W4 is why the rule exists: prikk refused *"nothing to commit"*, the program closed the
/// card and set a banner, and the prototype drew nothing — so the screen silently did nothing,
/// which is the confusion this project exists to prevent.
///
/// **Every banner the program produces**, not a hand-picked one: each error class whose
/// presentation is a banner is routed through the program, and the text it set must be on the
/// screen.
#[test]
fn every_banner_the_program_sets_is_drawn() {
    let mut drawn = 0;
    for (name, error, op) in banner_errors() {
        let settings = harness::drawing(Preset::LightAaa);
        let size = harness::logical(harness::WINDOW, 1.0);
        let mut screen = Screen::open(harness::on_orientation(), &settings, size);
        screen.stikk_mut().surface_error_for_test(&error, op);
        screen.draw();
        let Some(banner) = screen.stikk().app().banner().map(str::to_string) else {
            continue;
        };
        drawn += 1;
        let texts = screen.texts();
        assert!(
            texts
                .iter()
                .any(|text| text.content.contains(banner.trim())),
            "{name}: the program set the banner {banner:?} and the screen does not show it:\n{:#?}",
            texts.iter().map(|t| &t.content).collect::<Vec<_>>()
        );
        // **The run that fails it**: the same tree with the status area's banner drawing removed.
        let without = screen.texts_without_the_banner_row();
        assert!(
            !without
                .iter()
                .any(|text| text.content.contains(banner.trim())),
            "{name}: with the banner row removed the text must be gone, which is what makes the \
             assertion above load-bearing"
        );
    }
    assert!(drawn >= 3, "only {drawn} banners were produced to draw");
    println!("rule 9: {drawn} banners the program set, every one on the screen");
}

/// Every error class whose presentation sets a banner, with the operation that produced it.
fn banner_errors() -> Vec<(&'static str, stikk_model::StikkError, OperationContext)> {
    vec![
        (
            "a lock conflict",
            stikk_model::StikkError::LockConflict {
                message: "another prikk process holds the lock".to_string(),
            },
            OperationContext::Commit,
        ),
        (
            "an integrity finding",
            stikk_model::StikkError::IntegrityFinding {
                message: "prikk reported an integrity finding".to_string(),
            },
            OperationContext::Verify,
        ),
        (
            "a limit",
            stikk_model::StikkError::Limits {
                message: "at or above the configured limit".to_string(),
            },
            OperationContext::Commit,
        ),
        (
            "an environment fault",
            stikk_model::StikkError::Environment {
                detail: "prikk could not be started".to_string(),
                source: None,
            },
            OperationContext::Orient,
        ),
    ]
}
