use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::{
    kurbo::{Size, Vec2},
    prelude::*,
};

struct TextState {
    sheet: Stylesheet,
    text: UIString,
}

fn text_view(state: &TextState, ui: &mut Ui<TextState, ()>) {
    ui.node().style_sheet(&state.sheet).children(|ui| {
        ui.node().classes("label").text(state.text.clone());
    });
}

fn text_viewport(text: UIString, fixed_size: bool) -> (TextState, Viewport<TextState, ()>) {
    let css = if fixed_size {
        ".label { width: 100px; height: 40px; }"
    } else {
        ".label { width: auto; height: auto; }"
    };
    let state = TextState {
        sheet: css.parse().unwrap(),
        text,
    };
    let mut viewport = Viewport::new(text_view, Size::new(500.0, 500.0), Vec2::new(1.0, 1.0), TranslationMap::new(langid!("en-US")));
    viewport.frame(&state);
    (state, viewport)
}

#[test]
fn refreshed_text_tracks_new_dependencies_and_reuses_cache() {
    for fixed_size in [false, true] {
        let choice = Var::new(true);
        let a = Var::new(String::from("first"));
        let b = Var::new(String::from("second"));
        let calls = Arc::new(AtomicUsize::new(0));
        let text = {
            let (choice, a, b, calls) = (choice.downgrade(), a.downgrade(), b.downgrade(), calls.clone());
            UIString::deferred(move |_| {
                calls.fetch_add(1, Ordering::Relaxed);
                if choice.get().unwrap() { a.get().unwrap() } else { b.get().unwrap() }
            })
        };
        let (mut state, mut viewport) = text_viewport(text, fixed_size);
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        // Rebuilds should retain spare capacity instead of allocating a new map.
        let deps = &mut viewport.curr_tree.text_cache.values_mut().next().unwrap().deps.deps;
        deps.reserve(32);
        let capacity = deps.capacity();

        choice.set(false);
        viewport.dispatch_event_queue(&mut state, &());
        viewport.frame(&state);
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert!(viewport.curr_tree.text_cache.values().next().unwrap().deps.deps.capacity() >= capacity);

        // Discarded dependencies must not cause another frame.
        a.set(String::from("unused"));
        viewport.dispatch_event_queue(&mut state, &());
        assert!(viewport.is_idle());

        viewport.require_draw();
        viewport.frame(&state);
        assert_eq!(calls.load(Ordering::Relaxed), 2, "cache hit resolved text again");

        b.set(String::from("updated"));
        viewport.dispatch_event_queue(&mut state, &());
        assert!(!viewport.is_idle(), "new text dependency was lost on cache reuse");
        viewport.frame(&state);
        assert_eq!(calls.load(Ordering::Relaxed), 3);
    }
}

#[test]
fn dropped_text_removes_cached_output() {
    for fixed_size in [false, true] {
        let text = Var::new(String::from("old text"));
        let (mut state, mut viewport) = text_viewport(text.downgrade().into(), fixed_size);
        assert_eq!(viewport.curr_tree.text_cache.len(), 1);

        drop(text);
        viewport.dispatch_event_queue(&mut state, &());
        viewport.frame(&state);
        assert!(viewport.curr_tree.text_cache.is_empty());

        viewport.require_draw();
        viewport.frame(&state);
        assert!(viewport.curr_tree.text_cache.is_empty());
    }
}

#[test]
fn refreshed_style_drops_obsolete_dependencies() {
    struct State {
        choice: Var<bool>,
        a: Var<f32>,
        b: Var<f32>,
    }
    fn view(_: &State, ui: &mut Ui<State, ()>) {
        ui.node().on_style(|state, style| {
            style.opacity = if state.choice.get() { state.a.get() } else { state.b.get() };
        });
    }
    let mut state = State {
        choice: Var::new(true),
        a: Var::new(0.5),
        b: Var::new(0.8),
    };
    let mut viewport = Viewport::new(view, Size::new(500.0, 500.0), Vec2::new(1.0, 1.0), TranslationMap::new(langid!("en-US")));
    viewport.frame(&state);

    state.choice.set(false);
    viewport.dispatch_event_queue(&mut state, &());
    viewport.frame(&state);
    assert_eq!(viewport.curr_tree.style_cache[0].opacity, 0.8);

    state.a.set(0.6);
    viewport.dispatch_event_queue(&mut state, &());
    assert!(viewport.is_idle());

    state.b.set(0.9);
    viewport.dispatch_event_queue(&mut state, &());
    assert!(!viewport.is_idle());
    viewport.frame(&state);
    assert_eq!(viewport.curr_tree.style_cache[0].opacity, 0.9);
}
