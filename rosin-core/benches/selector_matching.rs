use std::{str::FromStr, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use kurbo::{Size, Vec2};

use rosin_core::{css::Stylesheet, prelude::*};

const TREE_DEPTH: usize = 8; // depth 8 + fanout 3 = 9,841 nodes
const FANOUT: usize = 3;

fn classes_for(depth: usize, child_i: usize, path_hash: u64) -> &'static str {
    // Try to keep these as &'static str so we don't allocate class strings while building the tree.
    // The idea is to produce a lot of overlap + some variation.
    const POOL: &[&str] = &[
        "panel",
        "panel elevated",
        "panel elevated theme-dark",
        "row",
        "row wrap",
        "col",
        "col grow",
        "text",
        "text subtle",
        "text heading",
        "btn",
        "btn primary",
        "btn secondary",
        "btn danger",
        "icon",
        "icon small",
        "input",
        "input focusable",
        "list",
        "list item",
        "card",
        "card interactive hoverable",
        "hoverable",
        "focusable",
        "activeable",
        "theme",
        "theme accent",
    ];

    // Mix depth/child index/path hash to avoid repeating the exact same class pattern everywhere.
    let idx = ((depth as u64 * 1315423911) ^ (child_i as u64 * 2654435761) ^ path_hash) as usize;
    POOL[idx % POOL.len()]
}

fn build_subtree(state: &Stylesheet, ui: &mut Ui<Stylesheet, ()>, depth: usize, child_i: usize, path_hash: u64) {
    // Simple LCG-ish hash to vary classes with structure.
    let h = path_hash.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) ^ (depth as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ (child_i as u64);

    let classes = classes_for(depth, child_i, h);

    // Attach the stylesheet at some internal nodes too, so active_sheets stack logic is exercised.
    let mut node = ui.node().classes(classes);

    if depth == TREE_DEPTH || depth.is_multiple_of(3) {
        node = node.style_sheet(state);
    }

    if depth == 0 {
        return;
    }

    node.children(|ui| {
        for i in 0..FANOUT {
            build_subtree(state, ui, depth - 1, i, h);
        }
    });
}

fn complicated_tree(state: &Stylesheet, ui: &mut Ui<Stylesheet, ()>) {
    ui.node().style_sheet(state).classes("root theme").children(|ui| {
        for i in 0..FANOUT {
            build_subtree(state, ui, TREE_DEPTH, i, 0xDEADBEEF);
        }
    });
}

fn make_diverse_stylesheet() -> String {
    let mut css = String::with_capacity(64 * 1024);

    // Baseline.
    css.push_str(
        r#"
        * { z-index: 0; }
        .root { position: self-directed; space: 1s 1s 1s 1s; z-index: 1; color: #222; }
        .theme { --accent: #06F; --muted: #777; --pad: 2s; }
        .theme-dark { --accent: #4AF; --muted: #AAA; color: #EEE; }
        .accent { color: var(--accent); }
        .subtle { color: var(--muted); }
        "#,
    );

    // A bunch of class rules that overlap so rule matching work increases.
    css.push_str(
        r#"
        .panel { position: parent-directed; space: var(--pad) var(--pad) var(--pad) var(--pad); z-index: 2; }
        .panel.elevated { z-index: 10; text-shadow: 1px 1px 2px rgba(0,0,0,0.25); }
        .card { space: 2s 2s 2s 2s; z-index: 3; }
        .row { space: 1s 2s 1s 2s; }
        .col { space: 2s 1s 2s 1s; }
        .grow { z-index: 4; }
        .text { color: #333; }
        .text.heading { color: #111; }
        .btn { z-index: 5; position: self-directed; }
        .btn.primary { color: var(--accent); }
        .btn.secondary { color: #0A0; }
        .btn.danger { color: #C00; }
        .icon { z-index: 6; }
        .input { z-index: 7; }
        "#,
    );

    // Descendant and child combinator stress.
    css.push_str(
        r#"
        .panel > .row .text { color: var(--muted); }
        .panel .btn { z-index: 20; }
        .card > .row > .col .btn.primary { z-index: 30; }
        .list .item .text { color: #555; }
        .list > .item { space: 1s 1s 1s 1s; }
        .panel .icon.small { z-index: 25; }
        "#,
    );

    // Pseudos: hover/focus/active/enabled.
    css.push_str(
        r#"
        .hoverable:hover { z-index: 100; color: #00F; }
        .focusable:focus { z-index: 101; color: #F0A; }
        .activeable:active { z-index: 102; color: #F50; }
        .btn:enabled { z-index: 110; }
        "#,
    );

    // Add lots of similar rules to scale up rule count and indexing pressure.
    // These all key by the final class, so they'll be candidates frequently.
    for i in 0..200 {
        // Alternate targets across common terminal classes.
        let terminal = match i % 6 {
            0 => ".panel",
            1 => ".row",
            2 => ".col",
            3 => ".text",
            4 => ".btn",
            _ => ".icon",
        };

        let (z, space) = (10 + i, (i % 5) as f32 + 1.0);

        // Vary selector structure a bit.
        if i % 3 == 0 {
            css.push_str(&format!(r#"{terminal} {{ z-index: {z}; space: {s}s {s}s {s}s {s}s; }}"#, terminal = terminal, z = z, s = space));
        } else if i % 3 == 1 {
            css.push_str(&format!(r#".panel {a} {terminal} {{ z-index: {z}; }}"#, a = if i % 2 == 0 { ">" } else { "" }, terminal = terminal, z = z));
        } else {
            css.push_str(&format!(
                r#".theme {terminal} {{ color: #{:02X}{:02X}{:02X}; }}"#,
                (i * 7) as u8,
                (i * 13) as u8,
                (i * 17) as u8,
                terminal = terminal
            ));
        }

        css.push('\n');
    }

    css
}

fn bench_style_pass(c: &mut Criterion) {
    let css = make_diverse_stylesheet();
    let sheet = Stylesheet::from_str(&css).expect("Failed to parse CSS");

    let size = Size::new(1920.0, 1080.0);
    let scale = Vec2::new(1.0, 1.0);

    c.bench_function("style_pass", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;

            for _ in 0..iters {
                let translation_map = TranslationMap::new(langid!("en-US"));

                let mut viewport = Viewport::new(complicated_tree, size, scale, translation_map);
                viewport.frame(&sheet);

                total += viewport.get_perf_info().style_time;
            }

            total
        })
    });
}

criterion_group!(benches, bench_style_pass);
criterion_main!(benches);
