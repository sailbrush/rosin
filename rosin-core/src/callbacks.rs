#![forbid(unsafe_code)]

use druid_shell::piet::Piet;
use druid_shell::{KeyEvent, MouseEvent};

use crate::geometry::Size;
use crate::prelude::*;

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A list of events that Nodes can register callbacks for.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum On {
    // Can be used by widgets to signal that they have changed
    Change,
    MouseDown,
    MouseUp,
    MouseMove,
    MouseEnter,
    MouseLeave,
    MouseWheel,
    KeyDown,
    KeyUp,
    Focus,
    Blur,
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

pub struct DrawCtx<'a, 'b> {
    pub piet: &'a mut Piet<'b>,
    pub style: &'a Style,
    pub width: f32,
    pub height: f32,
    pub must_draw: bool,
}

pub enum EventInfo {
    None,
    Mouse(MouseEvent),
    Key(KeyEvent),
}

pub struct EventCtx<S, H> {
    pub event_info: EventInfo,
    pub window_handle: H,
    pub resource_loader: Arc<Mutex<ResourceLoader>>,
    pub focus: Option<Key>,
    pub(crate) anim_tasks: Vec<Box<dyn AnimCallback<S>>>,
    pub(crate) change: bool,
}

impl<S, H> EventCtx<S, H> {
    pub fn blur(&mut self) {
        self.focus = None;
    }

    pub fn focus_on(&mut self, key: Key) {
        self.focus = Some(key);
    }

    pub fn start_animation(&mut self, callback: impl Fn(&mut S, Duration) -> (Phase, ShouldStop) + 'static) {
        self.anim_tasks.push(Box::new(callback));
    }

    pub fn emit_change(&mut self) {
        self.change = true;
    }
}

/// `Fn(&mut S, Duration) -> (Phase, ShouldStop)`
pub trait AnimCallback<S>: 'static + Fn(&mut S, Duration) -> (Phase, ShouldStop) {}
impl<F, S> AnimCallback<S> for F where F: 'static + Fn(&mut S, Duration) -> (Phase, ShouldStop) {}

/// `Fn(&S, &mut DrawCtx)`
pub trait DrawCallback<S>: 'static + Fn(&S, &mut DrawCtx) {}
impl<F, S> DrawCallback<S> for F where F: 'static + Fn(&S, &mut DrawCtx) {}

/// `Fn(&mut S, &mut EventCtx<S, H>) -> Phase`
pub trait EventCallback<S, H>: 'static + Fn(&mut S, &mut EventCtx<S, H>) -> Phase {}
impl<F, S, H> EventCallback<S, H> for F where F: 'static + Fn(&mut S, &mut EventCtx<S, H>) -> Phase {}

/// `Fn(&S, &mut Style)`
pub trait LayoutCallback<S>: 'static + Fn(&mut S, Size) {}
impl<F, S> LayoutCallback<S> for F where F: 'static + Fn(&mut S, Size) {}

/// `Fn(&S, &mut Style)`
pub trait StyleCallback<S>: 'static + Fn(&S, &mut Style) {}
impl<F, S> StyleCallback<S> for F where F: 'static + Fn(&S, &mut Style) {}

pub type ViewCallback<S, H> = fn(&S) -> Node<S, H>;
