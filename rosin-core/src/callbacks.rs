#![forbid(unsafe_code)]

use crate::prelude::*;

use std::time::Duration;

/// A list of events that Nodes can register callbacks for.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum On {
    MouseDown,
    MouseUp,
    Hover,

    Change, // Can be used by widgets to signal that they have changed
    Focus,
    Blur, // TODO - cache id on focus, so blur doesn't have to search
}

/// A return type for callbacks to signal which render phase to skip to.
#[must_use]
#[derive(Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Phase {
    Idle = 0,
    Draw = 1,
    Layout = 2,
    Build = 3,
}

/// A return type for tasks and animations to signal if they should stop running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShouldStop {
    Yes,
    No,
}

pub struct EventCtx {
    /*
Needs to provide:
    - Event info
    - The integration's Window Handle
    - Access to Sheet Loader
    - Things like blur/focus/animations

For providing a mechanism to cause other windows to redraw, use a global object
*/}

/// `Fn(&mut T, Duration) -> (Phase, ShouldStop)`
pub trait AnimCallback<S>: 'static + Fn(&mut S, Duration) -> (Phase, ShouldStop) {}
impl<F, S> AnimCallback<S> for F where F: 'static + Fn(&mut S, Duration) -> (Phase, ShouldStop) {}

/// `Fn(&T, &mut DrawCtx)`
pub trait DrawCallback<S>: 'static + Fn(&S, &mut DrawCtx) {}
impl<F, S> DrawCallback<S> for F where F: 'static + Fn(&S, &mut DrawCtx) {}

/// `Fn(&mut T, &mut App<T>) -> Phase`
pub trait EventCallback<S>: 'static + Fn(&mut S, &mut EventCtx) -> Phase {}
impl<F, S> EventCallback<S> for F where F: 'static + Fn(&mut S, &mut EventCtx) -> Phase {}

/// `Fn(&T, &mut Style)`
pub trait StyleCallback<S>: 'static + Fn(&S, &mut Style) {}
impl<F, S> StyleCallback<S> for F where F: 'static + Fn(&S, &mut Style) {}

pub type ViewCallback<S> = fn(&S) -> Node<S>;
