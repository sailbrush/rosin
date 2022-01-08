#![forbid(unsafe_code)]

use crate::prelude::*;
use crate::widgets::*;

// ---------- Button ----------
pub fn button<S>(text: &'static str, callback: impl Fn(&mut S, &mut EventCtx) -> Phase + 'static) -> Node<S> {
    label(text).event(On::MouseDown, callback)
}
