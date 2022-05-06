#![forbid(unsafe_code)]

use druid_shell::piet::Piet;
use druid_shell::KeyEvent;
use keyboard_types::Modifiers;

use crate::geometry::Size;
use crate::layout::Layout;
use crate::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A list of events that Nodes can register callbacks for.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum On {
    // Can be used by widgets to signal that they have changed
    Change,
    PointerDown,
    PointerUp,
    PointerMove,
    PointerEnter,
    PointerLeave,
    PointerWheel,
    Keyboard,
    Focus,
    Blur,
    WindowFocus,
    WindowBlur,
    WindowClose,
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

impl Phase {
    pub fn update(&mut self, mut other: Phase) {
        *self = *self.max(&mut other);
    }
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
    pub width: f64,
    pub height: f64,
    pub must_draw: bool,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum PointerButton {
    None,
    Left,
    Right,
    Middle,
    X1,
    X2,
}

impl PointerButton {
    /// Returns `true` if this is `PointerButton::Left`.
    #[inline]
    pub fn is_left(self) -> bool {
        self == PointerButton::Left
    }

    /// Returns `true` if this is `PointerButton::Right`.
    #[inline]
    pub fn is_right(self) -> bool {
        self == PointerButton::Right
    }

    /// Returns `true` if this is `PointerButton::Middle`.
    #[inline]
    pub fn is_middle(self) -> bool {
        self == PointerButton::Middle
    }

    /// Returns `true` if this is `PointerButton::X1`.
    #[inline]
    pub fn is_x1(self) -> bool {
        self == PointerButton::X1
    }

    /// Returns `true` if this is `PointerButton::X2`.
    #[inline]
    pub fn is_x2(self) -> bool {
        self == PointerButton::X2
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct PointerButtons(u8);

impl PointerButtons {
    /// Create a new empty set.
    #[inline]
    pub fn new() -> PointerButtons {
        PointerButtons(0)
    }

    /// Add the `button` to the set.
    #[inline]
    pub fn insert(&mut self, button: PointerButton) {
        self.0 |= 1.min(button as u8) << button as u8;
    }

    /// Remove the `button` from the set.
    #[inline]
    pub fn remove(&mut self, button: PointerButton) {
        self.0 &= !(1.min(button as u8) << button as u8);
    }

    /// Builder-style method for adding the `button` to the set.
    #[inline]
    pub fn with(mut self, button: PointerButton) -> PointerButtons {
        self.0 |= 1.min(button as u8) << button as u8;
        self
    }

    /// Builder-style method for removing the `button` from the set.
    #[inline]
    pub fn without(mut self, button: PointerButton) -> PointerButtons {
        self.0 &= !(1.min(button as u8) << button as u8);
        self
    }

    /// Returns `true` if the `button` is in the set.
    #[inline]
    pub fn contains(self, button: PointerButton) -> bool {
        (self.0 & (1.min(button as u8) << button as u8)) != 0
    }

    /// Returns `true` if the set is empty.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if all the `buttons` are in the set.
    #[inline]
    pub fn is_superset(self, buttons: PointerButtons) -> bool {
        self.0 & buttons.0 == buttons.0
    }

    /// Returns `true` if `PointerButton::Left` is in the set.
    #[inline]
    pub fn has_left(self) -> bool {
        self.contains(PointerButton::Left)
    }

    /// Returns `true` if `PointerButton::Right` is in the set.
    #[inline]
    pub fn has_right(self) -> bool {
        self.contains(PointerButton::Right)
    }

    /// Returns `true` if `PointerButton::Middle` is in the set.
    #[inline]
    pub fn has_middle(self) -> bool {
        self.contains(PointerButton::Middle)
    }

    /// Returns `true` if `PointerButton::X1` is in the set.
    #[inline]
    pub fn has_x1(self) -> bool {
        self.contains(PointerButton::X1)
    }

    /// Returns `true` if `PointerButton::X2` is in the set.
    #[inline]
    pub fn has_x2(self) -> bool {
        self.contains(PointerButton::X2)
    }

    /// Adds all the `buttons` to the set.
    #[inline]
    pub fn extend(&mut self, buttons: PointerButtons) {
        self.0 |= buttons.0;
    }

    /// Returns a union of the values in `self` and `other`.
    #[inline]
    pub fn union(mut self, other: PointerButtons) -> PointerButtons {
        self.0 |= other.0;
        self
    }

    /// Clear the set.
    #[inline]
    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

impl std::fmt::Debug for PointerButtons {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PointerButtons({:05b})", self.0 >> 1)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RawPointerEvent {
    pub window_pos_x: f32,
    pub window_pos_y: f32,
    pub wheel_x: f32,
    pub wheel_y: f32,
    pub button: PointerButton,
    pub buttons: PointerButtons,
    pub mods: Modifiers,
    pub count: u8,
    pub focus: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PointerEvent {
    pub pos_x: f32,
    pub pos_y: f32,
    pub window_pos_x: f32,
    pub window_pos_y: f32,
    pub wheel_x: f32,
    pub wheel_y: f32,
    pub button: PointerButton,
    pub buttons: PointerButtons,
    pub mods: Modifiers,
    pub count: u8,
    pub focus: bool,
}

impl From<RawPointerEvent> for PointerEvent {
    fn from(event: RawPointerEvent) -> Self {
        PointerEvent {
            pos_x: 0.0,
            pos_y: 0.0,
            window_pos_x: event.window_pos_x,
            window_pos_y: event.window_pos_y,
            wheel_x: event.wheel_x,
            wheel_y: event.wheel_y,
            button: event.button,
            buttons: event.buttons,
            mods: event.mods,
            count: event.count,
            focus: event.focus,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EventInfo {
    None,
    Pointer(PointerEvent),
    Keyboard(KeyEvent),
}

pub struct EventCtx<S, H> {
    pub info: EventInfo,
    pub platform_handle: H,
    pub resource_loader: Arc<Mutex<ResourceLoader>>,
    pub focus: Option<Key>,
    pub style: Style,
    pub(crate) layout: Layout,
    pub(crate) anim_tasks: Rc<RefCell<Vec<Box<dyn AnimCallback<S>>>>>,
    pub(crate) change: bool,
}

impl<S, H> EventCtx<S, H> {
    #[inline]
    pub fn blur(&mut self) {
        self.focus = None;
    }

    #[inline]
    pub fn focus_on(&mut self, key: Key) {
        self.focus = Some(key);
    }

    #[inline]
    pub fn start_animation(&mut self, callback: impl Fn(&mut S, Duration) -> (Phase, ShouldStop) + 'static) {
        self.anim_tasks.borrow_mut().push(Box::new(callback));
    }

    #[inline]
    pub fn emit_change(&mut self) {
        self.change = true;
    }

    #[inline]
    pub fn width(&self) -> f32 {
        self.layout.size.width
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.layout.size.height
    }

    #[inline]
    pub fn pointer(&self) -> Option<&PointerEvent> {
        if let EventInfo::Pointer(event) = &self.info {
            Some(event)
        } else {
            None
        }
    }

    #[inline]
    pub fn keyboard(&self) -> Option<&KeyEvent> {
        if let EventInfo::Keyboard(event) = &self.info {
            Some(event)
        } else {
            None
        }
    }
}

/// `Fn(&mut S, Duration) -> (Phase, ShouldStop)`
pub trait AnimCallback<S>: 'static + Fn(&mut S, Duration) -> (Phase, ShouldStop) {}
impl<F, S> AnimCallback<S> for F where F: 'static + Fn(&mut S, Duration) -> (Phase, ShouldStop) {}

/// `Fn(&S, &mut DrawCtx)`
pub trait DrawCallback<S>: 'static + Fn(&S, &mut DrawCtx) {}
impl<F, S> DrawCallback<S> for F where F: 'static + Fn(&S, &mut DrawCtx) {}

/// `Fn(&mut S, &mut EventCtx<S, H>) -> Phase`
pub trait EventCallback<S, H>: 'static + Fn(&mut S, &mut EventCtx<S, H>) -> Option<Phase> {}
impl<F, S, H> EventCallback<S, H> for F where F: 'static + Fn(&mut S, &mut EventCtx<S, H>) -> Option<Phase> {}

/// `Fn(&S, Size)`
pub trait LayoutCallback<S>: 'static + Fn(&S, Size) {}
impl<F, S> LayoutCallback<S> for F where F: 'static + Fn(&S, Size) {}

/// `Fn(&S, &mut Style)`
pub trait StyleCallback<S>: 'static + Fn(&S, &mut Style) {}
impl<F, S> StyleCallback<S> for F where F: 'static + Fn(&S, &mut Style) {}

pub type ViewCallback<S, H> = fn(&S) -> Node<S, H>;
