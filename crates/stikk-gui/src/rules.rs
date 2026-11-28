//! **RFC 069's rules, held** — the tests whose subject is a rule and not one module.
//!
//! Every one names the rule it holds, and the ones RFC 069 says *"must fail"* have their failing
//! run beside them: the same tree, with that one override off.
//!
//! **The matrix** is RFC 070 decision 7's: four presets × 100 % and 150 % × 1280 × 720 and
//! 800 × 600, over the screens this handoff draws.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use iced::Rectangle;
use stikk_app::App;
use stikk_gui_design::Preset;

use crate::harness::{self, Screen, TextBox};

/// One scene the matrix measures: what it is called, the program it starts from, and the key that
/// opens its card, if it has one.
type Scene = (&'static str, fn() -> App, Option<stikk_core::Key>);

/// The four scenes the matrix measures: the two screens, the worst case of the first, and the one
/// card.
///
/// A card is reached by the key the catalog binds to it, through the ordinary message path, so the
/// scene is one a user can be in.
fn scenes() -> Vec<Scene> {
    vec![
        ("Orientation", harness::on_orientation, None),
        ("History", harness::on_history, None),
        (
            "Orientation at its longest",
            harness::on_orientation_at_its_longest,
            None,
        ),
        (
            "the Glossary",
            harness::on_orientation,
            Some(stikk_core::Key::Help),
        ),
    ]
}

/// Open one scene at one cell of the matrix.
fn scene(
    fixture: fn() -> App,
    open: Option<stikk_core::Key>,
    preset: Preset,
    zoom: f32,
    window: iced::Size,
) -> Screen {
    let settings = harness::drawing(preset);
    let size = harness::logical(window, zoom);
    let mut screen = Screen::open(fixture(), &settings, size);
    // The high-contrast pair needs the desktop's own mode as well as the configured level.
    screen.desktop_says(if preset.is_dark() {
        iced::theme::Mode::Dark
    } else {
        iced::theme::Mode::Light
    });
    screen.window_says(size, 1.0);
    if let Some(key) = open {
        screen.press(key);
    }
    screen
}

// ---------------------------------------------------------------------------------------------
// Rule 1 — one design crate, and the views import only from it
// ---------------------------------------------------------------------------------------------

/// Every source file of this crate, with its path, for the source checks.
fn sources(crate_name: &str) -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crates directory")
        .join(crate_name)
        .join("src");
    let mut files = Vec::new();
    let mut stack = vec![root];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs")
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                files.push((path.display().to_string(), text));
            }
        }
    }
    assert!(!files.is_empty(), "the source check found no files to read");
    files
}

/// **The views**: the files that draw, which are what rules 1 and 13 are about.
///
/// The measuring instruments are not views — [`crate::harness`] reads Snora's own contrast
/// utilities to measure pixels, and could not do its job without them — and neither are the
/// records. Naming the views rather than excluding the rest is the honest shape: a new drawing
/// file has to be added here, and a reviewer sees the list.
fn views() -> Vec<(String, String)> {
    const DRAWS: &[&str] = &[
        "shell.rs",
        "card.rs",
        "view.rs",
        "orientation.rs",
        "history.rs",
    ];
    let files: Vec<(String, String)> = sources("stikk-gui")
        .into_iter()
        .filter(|(path, _)| DRAWS.iter().any(|name| path.ends_with(name)))
        .collect();
    assert_eq!(
        files.len(),
        DRAWS.len(),
        "every view in the list was found: {:?}",
        files.iter().map(|(path, _)| path).collect::<Vec<_>>()
    );
    files
}

/// A line of code, with the comments and the string literals taken out.
///
/// **A source check that reads prose finds the rule's own documentation** and reports the file
/// that explains the rule as the file that breaks it — which is how a name scan earns its
/// reputation. Only code is checked.
fn code_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (at, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("#!") {
            continue;
        }
        let code = match line.split_once("//") {
            Some((before, _)) => before,
            None => line,
        };
        // Drop every double-quoted literal, so a template's own words are not read as code.
        let mut kept = String::with_capacity(code.len());
        let mut inside = false;
        let mut escaped = false;
        for ch in code.chars() {
            match (inside, ch) {
                (true, '\\') if !escaped => escaped = true,
                (true, '"') if !escaped => inside = false,
                (true, _) => escaped = false,
                (false, '"') => inside = true,
                (false, _) => kept.push(ch),
            }
        }
        out.push((at + 1, kept));
    }
    out
}

/// **Rule 1's check:** the views name no `snora::design` path and no `iced::Color`.
///
/// It is a name scan, kept because it is cheap, and it does not prove the sizes — rule 2's H test
/// does that, on the rendered tree.
#[test]
fn no_view_names_snora_design_or_iced_colour() {
    let mut found = Vec::new();
    for (path, text) in views() {
        for (at, line) in code_lines(&text) {
            for banned in ["snora::design", "iced::Color"] {
                if line.contains(banned) {
                    found.push(format!("{path}:{at}: {banned}"));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "a view reached past the design crate:\n{}",
        found.join("\n")
    );
}

/// **The run that fails rule 1's check**: the same scan over a file that does name one.
///
/// Without this, a scan that silently matched nothing would pass for ever.
#[test]
fn the_rule_1_check_catches_a_file_that_names_them() {
    let offending = "use snora::design::Tokens;\nlet c = iced::Color::WHITE;\n";
    let hits: Vec<_> = code_lines(offending)
        .into_iter()
        .filter(|(_, line)| line.contains("snora::design") || line.contains("iced::Color"))
        .collect();
    assert_eq!(hits.len(), 2, "both are found: {hits:?}");
    // And the comments and literals it must *not* read:
    let innocent = "// snora::design is the design crate's own\nlet s = \"iced::Color\";\n";
    let misses: Vec<_> = code_lines(innocent)
        .into_iter()
        .filter(|(_, line)| line.contains("snora::design") || line.contains("iced::Color"))
        .collect();
    assert!(
        misses.is_empty(),
        "a comment and a string literal are not code: {misses:?}"
    );
}

/// **Rule 2's name check:** the design crate exposes no Semibold weight.
///
/// Stated as a name check, as the rule does. Noto Sans CJK has no SemiBold, so Japanese would fall
/// back to a serif face (knotra paid a release for this).
#[test]
fn the_design_crate_names_no_semibold_weight() {
    let mut found = Vec::new();
    for (path, text) in sources("stikk-gui-design") {
        for (at, line) in code_lines(&text) {
            let flattened = line.replace(' ', "").to_lowercase();
            if flattened.contains("semibold") || flattened.contains("weight::medium") {
                found.push(format!("{path}:{at}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a weight between Regular and Bold is named:\n{}",
        found.join("\n")
    );
}

/// **Rule 10's name check:** the design crate exposes no animation or shadow token beyond the one
/// that marks a floating card, which is Snora's own and applied by its renderer.
#[test]
fn the_design_crate_names_no_animation_or_extra_shadow_token() {
    let mut found = Vec::new();
    for (path, text) in sources("stikk-gui-design") {
        for (at, line) in code_lines(&text) {
            let flattened = line.replace(' ', "").to_lowercase();
            for banned in ["animation", "transition", "easing", "duration"] {
                if flattened.contains(banned) {
                    found.push(format!("{path}:{at}: {banned}"));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "an animation token is named:\n{}",
        found.join("\n")
    );
}

/// **Rule 13's check:** no view formats a user-facing string except through a template.
///
/// `format!` in a view is what the check looks for. The template table's own `fill` is the one
/// place that formats a sentence, and the records and the state log are not user-facing.
#[test]
fn no_view_formats_a_user_facing_string_except_through_a_template() {
    let mut found = Vec::new();
    for (path, text) in views() {
        for (at, line) in code_lines(&text) {
            if line.contains("format!") {
                found.push(format!("{path}:{at}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a sentence is assembled outside the template table:\n{}",
        found.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// Rule 2 — every text has a role size, measured on the rendered tree
// ---------------------------------------------------------------------------------------------

/// Every role's line height: the height one line of that role is laid out at.
fn role_line_heights() -> Vec<(f32, &'static str)> {
    let typography = snora::design::Typography::default_roles();
    vec![
        (
            typography.body_small.size * typography.body_small.line_height,
            "meta",
        ),
        (typography.body.size * typography.body.line_height, "body"),
        (
            typography.title.size * typography.title.line_height,
            "title",
        ),
        (
            typography.heading.size * typography.heading.line_height,
            "heading",
        ),
    ]
}

/// Which role a laid-out text's height belongs to: its height is a whole number of that role's
/// lines.
fn role_of(height: f32) -> Option<&'static str> {
    role_line_heights().into_iter().find_map(|(line, name)| {
        let lines = (height / line).round();
        (lines >= 1.0 && (height - lines * line).abs() <= 0.1).then_some(name)
    })
}

/// **Rule 2, H:** every text in the rendered tree of every screen, at every zoom, has a size from
/// the role table.
///
/// **Measured on the tree, not by scanning names.** A text's laid-out height is its role's size
/// times its line height, once per line, which is a fact about what was drawn and not about what
/// the source says. knotra's 212 hand-written size sites had only a name scan, and a name scan
/// cannot see a size composed at run time.
#[test]
fn every_text_has_a_role_size() {
    let mut measured = 0;
    for (name, fixture, open) in scenes() {
        for (preset, zoom, window) in harness::matrix() {
            let screen = scene(fixture, open, preset, zoom, window);
            for text in screen.texts() {
                if text.content.trim().is_empty() {
                    continue;
                }
                measured += 1;
                assert!(
                    role_of(text.bounds.height).is_some(),
                    "{name} at {} {zoom}x {}x{}: {:?} was laid out {} high, which is no role's \
                     line height ({:?})",
                    preset.name(),
                    window.width,
                    window.height,
                    text.content,
                    text.bounds.height,
                    role_line_heights(),
                );
            }
        }
    }
    println!("rule 2: {measured} texts measured on the tree, every one at a role's size");
    assert!(measured > 100, "the matrix measured {measured} texts");
}

// ---------------------------------------------------------------------------------------------
// Rule 3 — the contrast floors, from the rendered pixels
// ---------------------------------------------------------------------------------------------

/// Rule 3's floor for content text: 7 : 1 in every preset.
const TEXT_FLOOR: f32 = 7.0;
/// Rule 3's floor for a label: 4.5 : 1, and 7 : 1 in a high-contrast preset.
const LABEL_FLOOR: f32 = 4.5;

/// **Rule 3, H:** every text, in every preset, measured **from the rendered pixels**, meets its
/// own floor.
///
/// The measurement is the extreme pair of luminances inside a text's own rectangle: a glyph's core
/// is drawn at full coverage in the text's colour, and the rest of the rectangle is the surface
/// behind it.
///
/// **Two floors, as rule 3's table has two rows:** content text at 7 : 1, and a label — a text
/// inside a control — at 4.5 : 1, both one step higher in the high-contrast presets. Which a text
/// is comes from the tree: a label is one whose box is inside a control's.
///
/// **Only a text that is wholly on screen is measured.** A row clipped at the fold of a scrolling
/// card shows a sliver of itself — 1.3 px of a 22.4 px line, measured — and a sliver holds no
/// glyph to measure, so it would read as 1 : 1 and say nothing about the colours.
#[test]
fn every_text_meets_the_contrast_floor() {
    let mut worst = f32::INFINITY;
    let mut worst_where = String::new();
    let mut measured = 0;
    for (name, fixture, open) in scenes() {
        for preset in Preset::every() {
            let zoom = 1.0;
            let window = harness::WINDOW;
            let screen = scene(fixture, open, preset, zoom, window);
            let mode = if preset.is_dark() {
                iced::theme::Mode::Dark
            } else {
                iced::theme::Mode::Light
            };
            let settings = harness::drawing(preset);
            let picture = harness::picture(
                fixture,
                &settings,
                harness::logical(window, zoom),
                mode,
                open,
            );
            // **A card dims the page behind it** (rule 10's scrim, knotra's RFC-034 lesson), so
            // what is behind one is not content a user is reading and is not held to a floor.
            // When a card is open, the measurement stays inside its frame.
            let frame = screen.card_frame();
            let controls = screen.controls();
            for text in screen.texts() {
                if text.content.trim().is_empty() {
                    continue;
                }
                let Some(area) = text.visible_bounds else {
                    continue;
                };
                if !covers(area, text.bounds) {
                    continue;
                }
                if frame.is_some_and(|frame| !overlaps(area, frame)) {
                    continue;
                }
                let is_label = controls.iter().any(|control| covers(*control, text.bounds));
                let floor = floor_for(is_label, preset);
                let Some(ratio) = picture.contrast_in(area) else {
                    continue;
                };
                measured += 1;
                if ratio < worst {
                    worst = ratio;
                    worst_where = format!("{name}/{}: {:?}", preset.name(), text.content);
                }
                assert!(
                    ratio >= floor,
                    "{name} at {}: {:?} was drawn at {ratio:.2} : 1, below {floor} : 1",
                    preset.name(),
                    text.content,
                );
            }
        }
    }
    println!(
        "rule 3: {measured} texts measured from the pixels; the worst was {worst:.2} : 1 \
         ({worst_where})"
    );
    assert!(measured > 100, "the suite measured {measured} texts");
}

/// Which of rule 3's two floors a text is held to.
fn floor_for(is_label: bool, preset: Preset) -> f32 {
    match (is_label, preset.is_high_contrast()) {
        (true, false) => LABEL_FLOOR,
        (true, true) | (false, _) => TEXT_FLOOR,
    }
}

// ---------------------------------------------------------------------------------------------
// Rule 3 — the unavailable-control overrides, and the run that fails without them
// ---------------------------------------------------------------------------------------------

/// Every label in the specimen, with the contrast it was drawn at.
fn specimen_contrasts(
    preset: Preset,
    overrides: stikk_gui_design::Overrides,
) -> Vec<(String, f32)> {
    let size = iced::Size::new(420.0, 360.0);
    let mut texts: Vec<TextBox> = Vec::new();
    {
        // The guard is dropped with this block, before the picture is taken: the harness's lock is
        // not re-entrant, and `picture_of_specimen` takes it for itself.
        let _one_at_a_time = harness::rendering_for_a_specimen();
        let collect = |candidate: iced_test::selector::Candidate<'_>| -> Option<()> {
            if let iced_test::selector::Candidate::Text {
                bounds,
                visible_bounds,
                content,
                ..
            } = candidate
            {
                texts.push(TextBox {
                    content: content.to_string(),
                    bounds,
                    visible_bounds,
                });
            }
            None
        };
        let mut simulator = iced_test::Simulator::with_size(
            iced::Settings::default(),
            size,
            harness::specimen(preset, overrides),
        );
        let _ = simulator.find(collect);
    }
    let picture = harness::picture_of_specimen(harness::Spec { preset, overrides }, size);
    texts
        .into_iter()
        .filter_map(|text| {
            let area = text.visible_bounds?;
            let ratio = picture.contrast_in(area)?;
            Some((text.content, ratio))
        })
        .collect()
}

/// **Rule 3's unavailable floors:** an unavailable control's label meets 4.5 : 1 and its outline
/// 3 : 1 — 7 : 1 and 4.5 : 1 in the high-contrast presets — and it keeps its shape.
#[test]
fn an_unavailable_control_stays_readable() {
    for preset in Preset::every() {
        let overrides = harness::stikk_overrides();
        let floor = floor_for(true, preset);
        for (label, ratio) in specimen_contrasts(preset, overrides) {
            assert!(
                ratio >= floor,
                "{}: {label:?} was drawn at {ratio:.2} : 1, below {floor} : 1",
                preset.name()
            );
        }
        println!(
            "rule 3, {}: {:?}",
            preset.name(),
            specimen_contrasts(preset, overrides)
                .into_iter()
                .map(|(label, ratio)| format!("{label} {ratio:.2}"))
                .collect::<Vec<_>>()
        );
    }
}

/// **The run that fails it:** Snora's own disabled style, which multiplies every alpha by 0.45
/// (`snora-style-0.65.0/src/button.rs`, `disabled_alpha`).
#[test]
fn without_the_unavailable_overrides_the_floors_fail() {
    let overrides = harness::stikk_overrides().without_disabled_contrast();
    let mut below = Vec::new();
    for preset in Preset::every() {
        let floor = floor_for(true, preset);
        for (label, ratio) in specimen_contrasts(preset, overrides) {
            if ratio < floor {
                below.push(format!("{} {label:?} {ratio:.2} : 1", preset.name()));
            }
        }
    }
    assert!(
        !below.is_empty(),
        "without stikk's overrides, an unavailable control is below its floor"
    );
    println!(
        "rule 3's failing run: {} labels below their floor, the first being {:?}",
        below.len(),
        below.first()
    );
}

// ---------------------------------------------------------------------------------------------
// Rule 4 — every label fully visible, and clear of the scrollbar's own rectangle
// ---------------------------------------------------------------------------------------------

/// How wide iced's own scrollbar is, with no spacing: `Scrollbar::default()`'s `width` plus twice
/// its `margin` (`iced_widget-0.14.2/src/scrollable.rs:330-340`).
const BAR_WIDTH: f32 = 10.0;

/// **Every scrolling surface that has not reserved its bar's room** (rule 4's own words).
///
/// The deterministic half of the guard, and the one the failing run asserts: with
/// `Scrollbar::spacing` set, `Scrollable::layout` gives its content
/// `width - (bar + 2 × margin + spacing)` and the bar sits beside the text; with `spacing: None`
/// it falls through to `layout(0.0, 0.0)` and the content is given the **whole** width, bar strip
/// included, so the bar is painted over whatever lands there
/// (`iced_widget-0.14.2/src/scrollable.rs:474-534`; RFC 065 M5 measured it covering the end of a
/// line at 200 %).
///
/// Whether a glyph happens to land in that strip depends on where the wrap falls — measured, it
/// can miss by a glyph's width — so the geometry is what is asserted, and the text check below is
/// the second line.
fn surfaces_not_reserving_the_bar(screen: &Screen) -> Vec<String> {
    screen
        .scrollables()
        .into_iter()
        .filter(harness::ScrollBox::scrolls)
        .filter(|scrollable| scrollable.content.width > scrollable.bounds.width - BAR_WIDTH + 0.1)
        .map(|scrollable| {
            format!(
                "a scrolling surface {} wide gave its content {} of it, leaving nothing for the \
                 {BAR_WIDTH} px bar",
                scrollable.bounds.width, scrollable.content.width
            )
        })
        .collect()
}

/// Every label that runs into a scrollable's own bar, or is cut off.
fn labels_in_the_bar(screen: &Screen) -> Vec<String> {
    let bars: Vec<Rectangle> = screen
        .scrollables()
        .into_iter()
        .filter(harness::ScrollBox::scrolls)
        .map(|scrollable| Rectangle {
            x: scrollable.bounds.x + scrollable.bounds.width - BAR_WIDTH,
            y: scrollable.bounds.y,
            width: BAR_WIDTH,
            height: scrollable.bounds.height,
        })
        .collect();
    let mut found = Vec::new();
    for text in screen.texts() {
        if text.content.trim().is_empty() {
            continue;
        }
        // **Cut off across the line**: the layout gave it a width the window does not show.
        //
        // Only across the line, because **a card is scrolled**: a row below the fold has no
        // visible bounds at all, and that is what the scrollable is for. What rule 4 forbids is a
        // label whose *words* are cut — which in a window that never shrinks a text means its
        // width — and a label the bar is painted over, which is the next check.
        if let Some(visible) = text.visible_bounds
            && visible.width + 0.1 < text.bounds.width
        {
            found.push(format!(
                "{:?} is cut across the line: laid out {} wide, {} shown",
                text.content, text.bounds.width, visible.width
            ));
        }
        // **Under the bar**: `visible_bounds` passes a glyph the bar is painted over (RFC 065 M5),
        // so the comparison is against the bar's own rectangle.
        for bar in &bars {
            if text.visible_bounds.is_some() && overlaps(text.bounds, *bar) {
                found.push(format!(
                    "{:?} runs under the scrollbar: text {:?}, bar {:?}",
                    text.content, text.bounds, bar
                ));
            }
        }
    }
    found
}

/// Whether `outer` holds all of `inner`, to a tenth of a pixel.
fn covers(outer: Rectangle, inner: Rectangle) -> bool {
    inner.x >= outer.x - 0.1
        && inner.y >= outer.y - 0.1
        && inner.x + inner.width <= outer.x + outer.width + 0.1
        && inner.y + inner.height <= outer.y + outer.height + 0.1
}

/// Whether two rectangles share any area, to a tenth of a pixel.
fn overlaps(a: Rectangle, b: Rectangle) -> bool {
    a.x + a.width > b.x + 0.1
        && b.x + b.width > a.x + 0.1
        && a.y + a.height > b.y + 0.1
        && b.y + b.height > a.y + 0.1
}

/// **Rule 4, H — the label-fit guard:** every label is fully visible, and clear of the scrollbar's
/// own rectangle, in all four presets, at both zooms and both window sizes.
#[test]
fn every_label_fits_and_is_clear_of_the_scrollbar() {
    let mut cells = 0;
    for (name, fixture, open) in scenes() {
        for (preset, zoom, window) in harness::matrix() {
            let screen = scene(fixture, open, preset, zoom, window);
            let mut found = surfaces_not_reserving_the_bar(&screen);
            found.extend(labels_in_the_bar(&screen));
            assert!(
                found.is_empty(),
                "{name} at {} {zoom}x {}x{}:\n{}",
                preset.name(),
                window.width,
                window.height,
                found.join("\n")
            );
            cells += 1;
        }
    }
    println!("rule 4: {cells} cells of the matrix, every label whole and clear of the bar");
}

/// **The run that fails rule 4:** the same tree with `Scrollbar::spacing` removed.
///
/// RFC 069 names the commit confirmation at 150 %, which is a **writing card** and so is not in
/// this handoff (RFC 070's *What this RFC does not do*). What is measured instead is **Orientation
/// at its longest, at 150 %** — the scene with the most wrapped prose this handoff can draw, where
/// a sentence fills the body's width and the bar is painted over its end. **Stated, not
/// substituted silently.**
///
/// It is the body's own scrollable rather than a card's for one measured reason: a card is 560
/// logical pixels wide whatever the window is, so its longest sentence at 150 % is below the fold
/// and a text below the fold is not drawn at all. The body fills the window, so its longest
/// sentence is on screen and under the bar.
#[test]
fn without_the_scrollbar_room_the_guard_fails() {
    let mut settings = harness::drawing(Preset::LightAaa);
    settings.overrides = settings.overrides.without_scrollbar_spacing();
    let size = harness::logical(harness::WINDOW, 1.5);
    let mut screen = Screen::open(harness::on_orientation_at_its_longest(), &settings, size);
    screen.window_says(size, 1.0);

    let found = surfaces_not_reserving_the_bar(&screen);
    assert!(
        !found.is_empty(),
        "with `Scrollbar::spacing` removed, the body's own scrolling surface gives its content the \
         whole width and the bar is painted over it"
    );
    println!(
        "rule 4's failing run: {} findings, the first being {:?}",
        found.len(),
        found.first()
    );
    // And with the override on, the same scene reserves it.
    let mut with = harness::drawing(Preset::LightAaa);
    with.overrides = harness::stikk_overrides();
    let mut reserved = Screen::open(harness::on_orientation_at_its_longest(), &with, size);
    reserved.window_says(size, 1.0);
    assert!(
        surfaces_not_reserving_the_bar(&reserved).is_empty(),
        "the same scene with the override on reserves the bar's room"
    );
}

// ---------------------------------------------------------------------------------------------
// Rule 5 — 44 px targets
// ---------------------------------------------------------------------------------------------

/// Rule 5's floor, in logical pixels.
const TARGET: f32 = 44.0;

/// **Rule 5, H:** every control's hit area is at least 44 × 44 logical pixels, at every zoom.
#[test]
fn every_control_has_a_44_px_target() {
    let mut measured = 0;
    for (preset, zoom, window) in harness::matrix() {
        let screen = scene(
            harness::on_orientation,
            Some(stikk_core::Key::Help),
            preset,
            zoom,
            window,
        );
        for control in screen.controls() {
            measured += 1;
            assert!(
                control.width >= TARGET - 0.1 && control.height >= TARGET - 0.1,
                "a control at {} {zoom}x measured {}x{}, under {TARGET}x{TARGET}",
                preset.name(),
                control.width,
                control.height
            );
        }
    }
    assert!(measured > 0, "the matrix found no control to measure");
    println!("rule 5: {measured} controls measured, every one at or above {TARGET} px");
}

/// **The run that fails rule 5:** Snora's own 24 px floor, which its buttons measure 27.5–28.3 px
/// against (RFC 065 M4).
#[test]
fn without_the_target_override_the_floor_fails() {
    let mut settings = harness::drawing(Preset::LightAaa);
    settings.overrides = settings.overrides.without_targets_44();
    let size = harness::logical(harness::WINDOW, 1.0);
    let mut screen = Screen::open(harness::on_orientation(), &settings, size);
    screen.window_says(size, 1.0);
    screen.press(stikk_core::Key::Help);

    let controls = screen.controls();
    assert!(!controls.is_empty(), "the card has a control to measure");
    assert!(
        controls.iter().any(|c| c.height < TARGET),
        "with Snora's own floor the card's control is under {TARGET} px: {controls:?}"
    );
    println!(
        "rule 5's failing run: the control measured {:?}",
        controls.first()
    );
}

/// Every text the matrix drew, for a record that can be read beside a capture.
#[allow(dead_code)]
fn report(texts: &[TextBox]) -> String {
    texts
        .iter()
        .map(|t| format!("{:>8.3} {:?}", t.bounds.height, t.content))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------------------------
// Rule 2 — the interface font, named, and the generic family that is not it
// ---------------------------------------------------------------------------------------------

/// The face `fontdb` resolves for a family, as the database answers it.
///
/// `iced_graphics::text::font_system()` is the global `FontSystem` every iced program draws with,
/// and `raw()` gives its `cosmic_text::FontSystem`, whose `db()` is the `fontdb` that resolves a
/// family name to a face. Asking it is the logic level: no window, no renderer, no capture.
fn face_for(family: iced_graphics::text::cosmic_text::fontdb::Family<'_>) -> Option<String> {
    let mut system = iced_graphics::text::font_system()
        .write()
        .expect("the font system is not poisoned");
    let db = system.raw().db();
    let query = iced_graphics::text::cosmic_text::fontdb::Query {
        families: &[family],
        ..Default::default()
    };
    let id = db.query(&query)?;
    let face = db.face(id)?;
    face.families.first().map(|(name, _)| name.clone())
}

/// **Rule 2: the font database resolves the name the app gives it to that face** — and
/// `Font::DEFAULT` does not resolve to the same thing, which is why the name has to be given.
///
/// `Font::DEFAULT` is `Family::SansSerif` (`iced_core-0.14.0/src/font.rs:19-24`), the **generic**
/// family, which cosmic-text 0.15 sets to **"Open Sans"** by name after loading the database
/// (`src/font/system.rs:157-159`, its own *"TODO: configurable default fonts"*) — so it is not the
/// face fontconfig picks for `sans-serif`, and not the user's own choice.
///
/// **This is the failing case stated as a measurement**, not as a second code path: the two
/// answers are read from one database in one test, and the test says which is which. On a runner
/// with neither face installed the generic family answers something else again, and the record
/// says what it was.
#[test]
fn naming_the_interface_font_resolves_to_that_face() {
    use iced_graphics::text::cosmic_text::fontdb::Family;

    let generic = face_for(Family::SansSerif);
    let named = stikk_gui_design::interface_font_name();
    println!(
        "rule 2: the generic sans-serif family resolves to {generic:?}; stikk names {named:?}"
    );
    let Some(named) = named else {
        println!("rule 2: no interface font could be learned here, so iced's fallback draws");
        return;
    };
    let resolved = face_for(Family::Name(named));
    assert_eq!(
        resolved.as_deref(),
        Some(named),
        "the database resolves the name stikk gives it to that very face"
    );
    // **The two need not differ on every machine** — a system whose own interface font *is* Open
    // Sans would answer the same twice — so the assertion is that the named one is honoured, and
    // the record carries both answers.
    if generic.as_deref() != Some(named) {
        println!(
            "rule 2: naming the font changes which face draws: {generic:?} -> {:?}",
            Some(named)
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Rule 12 — ids, refs, paths and keys are isolated
// ---------------------------------------------------------------------------------------------

/// **Rule 12, H: every id, ref and path the views render is wrapped in the isolates.**
///
/// Checked on the **rendered text**, as the rule says: each value is looked for in the tree, and it
/// must appear between U+2066 and U+2069 and never bare.
#[test]
fn every_id_and_ref_in_history_is_isolated() {
    let settings = harness::drawing(Preset::LightAaa);
    let size = harness::logical(harness::WINDOW, 1.0);
    let screen = Screen::open(harness::on_history(), &settings, size);
    let drawn: Vec<String> = screen
        .texts()
        .into_iter()
        .map(|text| text.content)
        .collect();
    let view = harness::history_view();

    let mut values = vec![view.reff.clone()];
    for block in &view.blocks {
        values.push(block.block_id.chars().take(12).collect());
        values.push(block.kind.clone());
    }
    for value in values {
        let isolated = format!("\u{2066}{value}\u{2069}");
        assert!(
            drawn.iter().any(|text| text.contains(&isolated)),
            "{value:?} is drawn isolated; the screen says:\n{drawn:#?}"
        );
        assert!(
            !drawn
                .iter()
                .any(|text| text.contains(&value) && !text.contains(&isolated)),
            "{value:?} is nowhere drawn bare"
        );
    }
    println!("rule 12: every id, ref and kind on History is between the isolates");
}

/// **And the same on Orientation**, whose ids are the ref state and the queue's target.
#[test]
fn every_id_and_ref_on_orientation_is_isolated() {
    let settings = harness::drawing(Preset::LightAaa);
    let size = harness::logical(harness::WINDOW, 1.0);
    let screen = Screen::open(harness::on_orientation(), &settings, size);
    let drawn: Vec<String> = screen
        .texts()
        .into_iter()
        .map(|text| text.content)
        .collect();
    let view = harness::orientation_view();
    for value in [view.main_ref_state.clone(), view.queued_target.clone()]
        .into_iter()
        .flatten()
    {
        let isolated = format!("\u{2066}{value}\u{2069}");
        assert!(
            drawn.iter().any(|text| text.contains(&isolated)),
            "{value:?} is drawn isolated; the screen says:\n{drawn:#?}"
        );
    }
    // **prikk's own version line is prose, not a name**, so it is not isolated: it is a sentence
    // prikk wrote, and the prose rule is the one that may not drop a character from it.
    assert!(
        drawn
            .iter()
            .any(|text| text.contains(&view.prikk_version) && !text.contains('\u{2066}')),
        "prikk's version line is prose and is drawn as prikk wrote it"
    );
}
