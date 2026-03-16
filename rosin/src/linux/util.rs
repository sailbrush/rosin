use rosin_core::{
    keyboard_types::{Code, KeyboardEvent, Location, Modifiers},
    kurbo::{Point, Vec2},
    prelude::{Key, KeyState, NamedKey, PointerButton, PointerButtons, PointerEvent, PointerType},
};
use x11rb::protocol::xproto::{ButtonPressEvent, ButtonReleaseEvent, KeyButMask, KeyPressEvent, KeyReleaseEvent, MotionNotifyEvent};

pub(crate) fn panic_and_print(msg: String) -> ! {
    println!("{}", msg);
    std::process::abort()
}

pub(crate) fn convert_keyboard_event_pressed_x11(kpe: &KeyPressEvent) -> KeyboardEvent {
    let c = convert_code_x11(kpe.detail as u16);
    let k = convert_key(c);
    KeyboardEvent {
        code: c,
        state: KeyState::Down,
        key: if k.is_some() { k.unwrap() } else { Key::Character(c.to_string()) },
        location: convert_location(c),
        modifiers: convert_modifiers(kpe.state),
        repeat: false,
        is_composing: false,
    }
}

pub(crate) fn convert_keyboard_event_released_x11(kre: &KeyReleaseEvent) -> KeyboardEvent {
    let c = convert_code_x11(kre.detail as u16);
    let k = convert_key(c);
    KeyboardEvent {
        code: c,
        state: KeyState::Down,
        key: if k.is_some() { k.unwrap() } else { Key::Character(c.to_string()) },
        location: convert_location(c),
        modifiers: convert_modifiers(kre.state),
        repeat: false,
        is_composing: false,
    }
}
pub(crate) fn convert_mouse_button_pressed_x11(bpe: &ButtonPressEvent) -> PointerEvent {
    PointerEvent {
        viewport_pos: Point::new(bpe.event_x as f64, bpe.event_y as f64),
        wheel_delta: Vec2::default(),
        button: convert_mouse_button(bpe.detail),
        buttons: PointerButtons::empty().with(convert_mouse_button(bpe.detail)),
        mods: convert_modifiers(bpe.state),
        count: 1,
        did_focus_window: true,
        pressure: 1 as f32,
        tangential_pressure: 0 as f32,
        tilt: Vec2::default(),
        twist: 0 as f32,
        pointer_type: PointerType::Mouse,
    }
}
pub(crate) fn convert_mouse_button_released_x11(bre: &ButtonReleaseEvent) -> PointerEvent {
    PointerEvent {
        viewport_pos: Point::new(bre.event_x as f64, bre.event_y as f64),
        wheel_delta: Vec2::default(),
        button: convert_mouse_button(bre.detail),
        buttons: PointerButtons::empty(),
        mods: convert_modifiers(bre.state),
        count: 0,
        did_focus_window: true,
        pressure: 1 as f32,
        tangential_pressure: 0 as f32,
        tilt: Vec2::default(),
        twist: 0 as f32,
        pointer_type: PointerType::Mouse,
    }
}

pub(crate) fn convert_mouse_motion_x11(mm: &MotionNotifyEvent) -> PointerEvent {
    PointerEvent {
        viewport_pos: Point::new(mm.event_x as f64, mm.event_y as f64),
        wheel_delta: Vec2::default(),
        button: convert_mouse_button(mm.detail.into()),
        buttons: PointerButtons::empty(),
        mods: Modifiers::empty(),
        count: 0,
        did_focus_window: false,
        pressure: 0 as f32,
        tangential_pressure: 0 as f32,
        tilt: Vec2::default(),
        twist: 0 as f32,
        pointer_type: PointerType::Mouse,
    }
}

fn convert_modifiers(modifiers: KeyButMask) -> Modifiers {
    let mut retval = Modifiers::default();
    if modifiers.contains(KeyButMask::SHIFT) {
        retval = retval | Modifiers::SHIFT;
    }
    if modifiers.contains(KeyButMask::CONTROL) {
        retval = retval | Modifiers::CONTROL;
    }
    println!("{:?}", modifiers);
    return retval;
}
fn convert_mouse_button(btn: u8) -> PointerButton {
    PointerButton::from(btn as isize)
}

fn convert_key(code: Code) -> Option<Key> {
    Some(match code {
        Code::AltLeft | Code::AltRight => Key::Named(NamedKey::Alt),
        Code::ArrowDown => Key::Named(NamedKey::ArrowDown),
        Code::ArrowLeft => Key::Named(NamedKey::ArrowLeft),
        Code::ArrowRight => Key::Named(NamedKey::ArrowRight),
        Code::ArrowUp => Key::Named(NamedKey::ArrowUp),
        Code::Backspace => Key::Named(NamedKey::Backspace),
        Code::CapsLock => Key::Named(NamedKey::CapsLock),
        Code::ContextMenu => Key::Named(NamedKey::ContextMenu),
        Code::ControlLeft | Code::ControlRight => Key::Named(NamedKey::Control),
        Code::Delete => Key::Named(NamedKey::Delete),
        Code::End => Key::Named(NamedKey::End),
        Code::Enter => Key::Named(NamedKey::Enter),
        Code::Escape => Key::Named(NamedKey::Escape),
        Code::F1 => Key::Named(NamedKey::F1),
        Code::F2 => Key::Named(NamedKey::F2),
        Code::F3 => Key::Named(NamedKey::F3),
        Code::F4 => Key::Named(NamedKey::F4),
        Code::F5 => Key::Named(NamedKey::F5),
        Code::F6 => Key::Named(NamedKey::F6),
        Code::F7 => Key::Named(NamedKey::F7),
        Code::F8 => Key::Named(NamedKey::F8),
        Code::F9 => Key::Named(NamedKey::F9),
        Code::F10 => Key::Named(NamedKey::F10),
        Code::F11 => Key::Named(NamedKey::F11),
        Code::F12 => Key::Named(NamedKey::F12),
        Code::F13 => Key::Named(NamedKey::F13),
        Code::F14 => Key::Named(NamedKey::F14),
        Code::F15 => Key::Named(NamedKey::F15),
        Code::F16 => Key::Named(NamedKey::F16),
        Code::F17 => Key::Named(NamedKey::F17),
        Code::F18 => Key::Named(NamedKey::F18),
        Code::F19 => Key::Named(NamedKey::F19),
        Code::F20 => Key::Named(NamedKey::F20),
        Code::F21 => Key::Named(NamedKey::F21),
        Code::F22 => Key::Named(NamedKey::F22),
        Code::F23 => Key::Named(NamedKey::F23),
        Code::F24 => Key::Named(NamedKey::F24),
        Code::Fn => Key::Named(NamedKey::Fn),
        Code::Help => Key::Named(NamedKey::Help),
        Code::Home => Key::Named(NamedKey::Home),
        Code::Insert => Key::Named(NamedKey::Insert),
        Code::Lang1 => Key::Named(NamedKey::KanjiMode),
        Code::Lang2 => Key::Named(NamedKey::Eisu),
        Code::MetaLeft | Code::MetaRight => Key::Named(NamedKey::Meta),
        Code::NumLock => Key::Named(NamedKey::Clear),
        Code::NumpadEnter => Key::Named(NamedKey::Enter),
        Code::PageDown => Key::Named(NamedKey::PageDown),
        Code::PageUp => Key::Named(NamedKey::PageUp),
        Code::Pause => Key::Named(NamedKey::Pause),
        Code::PrintScreen => Key::Named(NamedKey::PrintScreen),
        Code::ScrollLock => Key::Named(NamedKey::ScrollLock),
        Code::ShiftLeft | Code::ShiftRight => Key::Named(NamedKey::Shift),
        Code::Tab => Key::Named(NamedKey::Tab),
        _ => return None,
    })
}

fn convert_location(code: Code) -> Location {
    match code {
        Code::MetaLeft | Code::ShiftLeft | Code::AltLeft | Code::ControlLeft => Location::Left,
        Code::MetaRight | Code::ShiftRight | Code::AltRight | Code::ControlRight => Location::Right,
        Code::Numpad0
        | Code::Numpad1
        | Code::Numpad2
        | Code::Numpad3
        | Code::Numpad4
        | Code::Numpad5
        | Code::Numpad6
        | Code::Numpad7
        | Code::Numpad8
        | Code::Numpad9
        | Code::NumpadAdd
        | Code::NumpadComma
        | Code::NumpadDecimal
        | Code::NumpadDivide
        | Code::NumpadEnter
        | Code::NumpadEqual
        | Code::NumpadMultiply
        | Code::NumpadSubtract => Location::Numpad,
        _ => Location::Standard,
    }
}

fn convert_code_x11(key_code: u16) -> Code {
    match key_code {
        77 => Code::NumLock,
        106 => Code::NumpadDivide,
        104 => Code::NumpadEnter,
        38 => Code::KeyA,
        39 => Code::KeyS,
        40 => Code::KeyD,
        41 => Code::KeyF,
        42 => Code::KeyG,
        43 => Code::KeyH,
        52 => Code::KeyZ,
        53 => Code::KeyX,
        54 => Code::KeyC,
        55 => Code::KeyV,
        51 => Code::Backslash,
        51 => Code::IntlBackslash,
        56 => Code::KeyB,
        24 => Code::KeyQ,
        25 => Code::KeyW,
        26 => Code::KeyE,
        27 => Code::KeyR,
        29 => Code::KeyY,
        28 => Code::KeyT,
        10 => Code::Digit1,
        11 => Code::Digit2,
        12 => Code::Digit3,
        13 => Code::Digit4,
        15 => Code::Digit6,
        14 => Code::Digit5,
        21 => Code::Equal,
        18 => Code::Digit9,
        16 => Code::Digit7,
        20 => Code::Minus,
        17 => Code::Digit8,
        19 => Code::Digit0,
        35 => Code::BracketRight,
        32 => Code::KeyO,
        30 => Code::KeyU,
        34 => Code::BracketLeft,
        31 => Code::KeyI,
        33 => Code::KeyP,
        36 => Code::Enter,
        46 => Code::KeyL,
        44 => Code::KeyJ,
        45 => Code::KeyK,
        47 => Code::Semicolon,
        59 => Code::Comma,
        61 => Code::Slash,
        57 => Code::KeyN,
        58 => Code::KeyM,
        60 => Code::Period,
        23 => Code::Tab,
        65 => Code::Space,
        49 => Code::Backquote,
        22 => Code::Backspace,
        9 => Code::Escape,
        133 => Code::MetaLeft,
        50 => Code::ShiftLeft,
        66 => Code::CapsLock,
        64 => Code::AltLeft,
        37 => Code::ControlLeft,
        62 => Code::ShiftRight,
        108 => Code::AltRight,
        105 => Code::ControlRight,
        71 => Code::F5,
        72 => Code::F6,
        73 => Code::F7,
        69 => Code::F3,
        74 => Code::F8,
        75 => Code::F9,
        95 => Code::F11,
        76 => Code::F10,
        96 => Code::F12,
        118 => Code::Insert,
        110 => Code::Home,
        112 => Code::PageUp,
        119 => Code::Delete,
        70 => Code::F4,
        115 => Code::End,
        68 => Code::F2,
        117 => Code::PageDown,
        67 => Code::F1,
        113 => Code::ArrowLeft,
        114 => Code::ArrowRight,
        116 => Code::ArrowDown,
        111 => Code::ArrowUp,
        _ => Code::Unidentified,
    }
}
