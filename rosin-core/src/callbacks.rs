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
pub trait AnimCallback<T>: 'static + Fn(&mut T, Duration) -> (Phase, ShouldStop) {}
impl<F, T> AnimCallback<T> for F where F: 'static + Fn(&mut T, Duration) -> (Phase, ShouldStop) {}

/// `Fn(&T, &mut DrawCtx)`
pub trait DrawCallback<T>: 'static + Fn(&T, &mut DrawCtx) {}
impl<F, T> DrawCallback<T> for F where F: 'static + Fn(&T, &mut DrawCtx) {}

/// `Fn(&mut T, &mut App<T>) -> Phase`
pub trait EventCallback<T>: 'static + Fn(&mut T, &mut EventCtx) -> Phase {}
impl<F, T> EventCallback<T> for F where F: 'static + Fn(&mut T, &mut EventCtx) -> Phase {}

/// `Fn(&T, &mut Style)`
pub trait StyleCallback<T>: 'static + Fn(&T, &mut Style) {}
impl<F, T> StyleCallback<T> for F where F: 'static + Fn(&T, &mut Style) {}

pub type ViewCallback<T> = fn(&T) -> Node<T>;
