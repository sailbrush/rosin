#![forbid(unsafe_code)]

use crate::prelude::*;
use crate::widgets::*;

// ---------- Button ----------
#[track_caller]
pub fn button<T>(text: &'static str, callback: impl Fn(&mut T, &mut EventCtx) -> Phase + 'static) -> Node<T> {
    label(text).event(On::MouseDown, callback)
}
