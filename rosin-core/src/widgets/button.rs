#![forbid(unsafe_code)]

use crate::prelude::*;
use crate::widgets::*;

// ---------- Button ----------
pub fn button<S, H>(text: &'static str, callback: impl Fn(&mut S, &mut EventCtx<S, H>) -> Option<Phase> + 'static) -> View<S, H> {
    label(text).event(On::PointerDown, callback)
}
