//! Pointer input types used by the event system.

use keyboard_types::Modifiers;
use kurbo::{Point, Vec2};

/// Identifies a specific button on a pointer device.
#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum PointerButton {
    #[default]
    None = 0,

    /// The primary pointer button, usually the left mouse button.
    Primary = 1,

    /// The secondary pointer button, usually the right mouse button.
    Secondary = 2,

    /// The auxiliary pointer button, usually the wheel or middle mouse button.
    Auxiliary = 3,

    /// The fourth pointer button, usually the back button.
    X1 = 4,

    /// The fifth pointer button, usually the forward button.
    X2 = 5,
}

impl From<isize> for PointerButton {
    fn from(value: isize) -> Self {
        match value {
            0 => PointerButton::None,
            1 => PointerButton::Primary,
            2 => PointerButton::Secondary,
            3 => PointerButton::Auxiliary,
            4 => PointerButton::X1,
            5 => PointerButton::X2,
            _ => PointerButton::None,
        }
    }
}

impl PointerButton {
    /// Returns `true` if this is [`PointerButton::Primary`].
    #[inline]
    pub fn is_primary(self) -> bool {
        self == PointerButton::Primary
    }

    /// Returns `true` if this is [`PointerButton::Secondary`].
    #[inline]
    pub fn is_secondary(self) -> bool {
        self == PointerButton::Secondary
    }

    /// Returns `true` if this is [`PointerButton::Auxiliary`].
    #[inline]
    pub fn is_auxiliary(self) -> bool {
        self == PointerButton::Auxiliary
    }

    /// Returns `true` if this is [`PointerButton::X1`].
    #[inline]
    pub fn is_x1(self) -> bool {
        self == PointerButton::X1
    }

    /// Returns `true` if this is [`PointerButton::X2`].
    #[inline]
    pub fn is_x2(self) -> bool {
        self == PointerButton::X2
    }
}

/// A set of pressed pointer buttons.
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct PointerButtons(u8);

impl PointerButtons {
    /// Creates an empty set of buttons.
    #[inline]
    pub fn empty() -> PointerButtons {
        PointerButtons(0)
    }

    #[inline]
    fn mask(button: PointerButton) -> u8 {
        match button {
            PointerButton::None => 0,
            _ => 1u8 << ((button as u8) - 1),
        }
    }

    /// Adds a button to the set.
    #[inline]
    pub fn insert(&mut self, button: PointerButton) {
        self.0 |= Self::mask(button);
    }

    /// Adds multiple buttons to the set.
    #[inline]
    pub fn insert_all(&mut self, buttons: PointerButtons) {
        self.0 |= buttons.0;
    }

    /// Removes a button from the set.
    #[inline]
    pub fn remove(&mut self, button: PointerButton) {
        self.0 &= !Self::mask(button);
    }

    /// Removes multiple buttons from the set.
    #[inline]
    pub fn remove_all(&mut self, buttons: PointerButtons) {
        self.0 &= !buttons.0;
    }

    /// Returns the set with a button added.
    #[inline]
    pub fn with(mut self, button: PointerButton) -> PointerButtons {
        self.0 |= Self::mask(button);
        self
    }

    /// Returns the set with a button removed.
    #[inline]
    pub fn without(mut self, button: PointerButton) -> PointerButtons {
        self.0 &= !Self::mask(button);
        self
    }

    /// Clears the set.
    #[inline]
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Returns `true` if `button` is in the set.
    #[inline]
    pub fn contains(self, button: PointerButton) -> bool {
        (self.0 & Self::mask(button)) != 0
    }

    /// Returns `true` if the set is empty.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if all `buttons` are in the set.
    #[inline]
    pub fn is_superset(self, buttons: PointerButtons) -> bool {
        self.0 & buttons.0 == buttons.0
    }

    /// Returns `true` if [`PointerButton::Primary`] is in the set.
    #[inline]
    pub fn has_primary(self) -> bool {
        self.contains(PointerButton::Primary)
    }

    /// Returns `true` if [`PointerButton::Secondary`] is in the set.
    #[inline]
    pub fn has_secondary(self) -> bool {
        self.contains(PointerButton::Secondary)
    }

    /// Returns `true` if [`PointerButton::Auxiliary`] is in the set.
    #[inline]
    pub fn has_auxiliary(self) -> bool {
        self.contains(PointerButton::Auxiliary)
    }

    /// Returns `true` if [`PointerButton::X1`] is in the set.
    #[inline]
    pub fn has_x1(self) -> bool {
        self.contains(PointerButton::X1)
    }

    /// Returns `true` if [`PointerButton::X2`] is in the set.
    #[inline]
    pub fn has_x2(self) -> bool {
        self.contains(PointerButton::X2)
    }
}

impl From<u8> for PointerButtons {
    fn from(value: u8) -> Self {
        PointerButtons(value & 0b1_1111)
    }
}

impl std::fmt::Debug for PointerButtons {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PointerButtons({:05b})", self.0)
    }
}

/// The device type associated with a pointer event.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PointerType {
    #[default]
    Mouse,
    Pen,
}

/// Information about a pointer event.
#[derive(Debug, Clone, Copy, Default)]
pub struct PointerEvent {
    /// The position of the pointer event in the viewport.
    pub viewport_pos: Point,

    /// The scroll amount.
    pub wheel_delta: Vec2,

    /// The button responsible for a pointer event.
    /// This will always be [`PointerButton::None`] for an [`On::PointerMove`](crate::events::On::PointerMove) event.
    pub button: PointerButton,

    /// Pointer buttons being held down during a move or after a click event.
    /// It will contain the button that caused an [`On::PointerDown`](crate::events::On::PointerDown) event,
    /// but it will not contain the button that caused an [`On::PointerUp`](crate::events::On::PointerUp) event.
    pub buttons: PointerButtons,

    /// Keyboard modifier keys pressed at the time of the event.
    pub mods: Modifiers,

    /// The number of clicks associated with this event.
    /// This will always be 0 for [`On::PointerUp`](crate::events::On::PointerUp) and [`On::PointerMove`](crate::events::On::PointerMove) events.
    pub count: u8,

    /// This is set to `true` if the pointer event caused the window to gain focus.
    pub did_focus_window: bool,

    /// The normalized pressure of the pointer input in the range 0 to 1, where 0 and 1 represent
    /// the minimum and maximum pressure the hardware is capable of detecting, respectively.
    pub pressure: f32,

    /// The normalized tangential pressure of the pointer input
    /// in the range -1 to 1, where 0 is the neutral position of the control.
    pub tangential_pressure: f32,

    /// The tilt of the pen in the X and Y axis, from -1 to 1.
    pub tilt: Vec2,

    /// The clockwise rotation of the pen stylus around
    /// its major axis in degrees, with a value in the range 0 to 359.
    pub twist: f32,

    /// Indicates the device type that caused the event.
    pub pointer_type: PointerType,
}

impl PointerEvent {
    #[inline]
    pub(crate) fn synthetic_move(viewport_pos: Point) -> Self {
        Self {
            viewport_pos,
            button: PointerButton::None,
            buttons: PointerButtons::empty(),
            count: 0,
            ..Self::default()
        }
    }

    #[inline]
    pub(crate) fn synthetic_primary_down(viewport_pos: Point, click_count: u8) -> Self {
        let buttons = PointerButtons::empty().with(PointerButton::Primary);
        Self {
            viewport_pos,
            button: PointerButton::Primary,
            buttons,
            count: click_count.max(1),
            pressure: 1.0,
            ..Self::default()
        }
    }

    #[inline]
    pub(crate) fn synthetic_primary_up(viewport_pos: Point) -> Self {
        Self {
            viewport_pos,
            button: PointerButton::Primary,
            buttons: PointerButtons::empty(),
            count: 0,
            pressure: 0.0,
            ..Self::default()
        }
    }
}
