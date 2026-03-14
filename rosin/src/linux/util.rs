use rosin_core::{keyboard_types::{Code, KeyboardEvent, Location, Modifiers}, kurbo::{Point, Vec2}, prelude::{Key, KeyState, NamedKey, PointerButton, PointerButtons, PointerEvent, PointerType}};
use x11rb::protocol::{xproto::{ButtonPressEvent, ButtonReleaseEvent, KeyButMask, KeyPressEvent, KeyReleaseEvent, MotionNotifyEvent}};

pub(crate) fn panic_and_print(msg: String) -> ! {
    println!("{}", msg);
    std::process::abort()
}


pub(crate) fn convert_keyboard_event_pressed_x11(kpe: &KeyPressEvent) -> KeyboardEvent {
    let c = convert_code(kpe.detail as u16);
    let k = convert_key(c);
     KeyboardEvent {
        code: c,
        state: KeyState::Down,
        key: if k.is_some() {k.unwrap()} else {Key::Character(c.to_string())},
        location: convert_location(c),
        modifiers: convert_modifiers(kpe.state),
        repeat: false,
        is_composing: false,
    }
}

pub(crate) fn convert_keyboard_event_released_x11(kre: &KeyReleaseEvent) -> KeyboardEvent {
    let c = convert_code(kre.detail as u16);
    let k = convert_key(c);
     KeyboardEvent {
        code: c,
        state: KeyState::Down,
        key: if k.is_some() {k.unwrap()} else {Key::Character(c.to_string())},
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
        pointer_type: PointerType::Mouse
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
        pointer_type: PointerType::Mouse
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
        pointer_type: PointerType::Mouse
    }
}

fn convert_modifiers(modifiers: KeyButMask) -> Modifiers {
    let mut retval = Modifiers::default();
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

fn convert_code(key_code: u16) -> Code {
    match key_code {
        0x00 => Code::KeyA,
        0x01 => Code::KeyS,
        0x02 => Code::KeyD,
        0x03 => Code::KeyF,
        0x04 => Code::KeyH,
        0x05 => Code::KeyG,
        0x06 => Code::KeyZ,
        0x07 => Code::KeyX,
        0x08 => Code::KeyC,
        0x09 => Code::KeyV,
        0x0a => Code::IntlBackslash,
        0x0b => Code::KeyB,
        0x0c => Code::KeyQ,
        0x0d => Code::KeyW,
        0x0e => Code::KeyE,
        0x0f => Code::KeyR,
        0x10 => Code::KeyY,
        0x11 => Code::KeyT,
        0x12 => Code::Digit1,
        0x13 => Code::Digit2,
        0x14 => Code::Digit3,
        0x15 => Code::Digit4,
        0x16 => Code::Digit6,
        0x17 => Code::Digit5,
        0x18 => Code::Equal,
        0x19 => Code::Digit9,
        0x1a => Code::Digit7,
        0x1b => Code::Minus,
        0x1c => Code::Digit8,
        0x1d => Code::Digit0,
        0x1e => Code::BracketRight,
        0x1f => Code::KeyO,
        0x20 => Code::KeyU,
        0x21 => Code::BracketLeft,
        0x22 => Code::KeyI,
        0x23 => Code::KeyP,
        0x24 => Code::Enter,
        0x25 => Code::KeyL,
        0x26 => Code::KeyJ,
        0x27 => Code::Quote,
        0x28 => Code::KeyK,
        0x29 => Code::Semicolon,
        0x2a => Code::Backslash,
        0x2b => Code::Comma,
        0x2c => Code::Slash,
        0x2d => Code::KeyN,
        0x2e => Code::KeyM,
        0x2f => Code::Period,
        0x30 => Code::Tab,
        0x31 => Code::Space,
        0x32 => Code::Backquote,
        0x33 => Code::Backspace,
        0x34 => Code::NumpadEnter,
        0x35 => Code::Escape,
        0x36 => Code::MetaRight,
        0x37 => Code::MetaLeft,
        0x38 => Code::ShiftLeft,
        0x39 => Code::CapsLock,
        0x3a => Code::AltLeft,
        0x3b => Code::ControlLeft,
        0x3c => Code::ShiftRight,
        0x3d => Code::AltRight,
        0x3e => Code::ControlRight,
        0x3f => Code::Fn,
        0x40 => Code::F17,
        0x41 => Code::NumpadDecimal,
        0x43 => Code::NumpadMultiply,
        0x45 => Code::NumpadAdd,
        0x47 => Code::NumLock,
        0x48 => Code::AudioVolumeUp,
        0x49 => Code::AudioVolumeDown,
        0x4a => Code::AudioVolumeMute,
        0x4b => Code::NumpadDivide,
        0x4c => Code::NumpadEnter,
        0x4e => Code::NumpadSubtract,
        0x4f => Code::F18,
        0x50 => Code::F19,
        0x51 => Code::NumpadEqual,
        0x52 => Code::Numpad0,
        0x53 => Code::Numpad1,
        0x54 => Code::Numpad2,
        0x55 => Code::Numpad3,
        0x56 => Code::Numpad4,
        0x57 => Code::Numpad5,
        0x58 => Code::Numpad6,
        0x59 => Code::Numpad7,
        0x5a => Code::F20,
        0x5b => Code::Numpad8,
        0x5c => Code::Numpad9,
        0x5d => Code::IntlYen,
        0x5e => Code::IntlRo,
        0x5f => Code::NumpadComma,
        0x60 => Code::F5,
        0x61 => Code::F6,
        0x62 => Code::F7,
        0x63 => Code::F3,
        0x64 => Code::F8,
        0x65 => Code::F9,
        0x66 => Code::Lang2,
        0x67 => Code::F11,
        0x68 => Code::Lang1,
        0x69 => Code::F13,
        0x6a => Code::F16,
        0x6b => Code::F14,
        0x6d => Code::F10,
        0x6e => Code::ContextMenu,
        0x6f => Code::F12,
        0x71 => Code::F15,
        0x72 => Code::Insert,
        0x73 => Code::Home,
        0x74 => Code::PageUp,
        0x75 => Code::Delete,
        0x76 => Code::F4,
        0x77 => Code::End,
        0x78 => Code::F2,
        0x79 => Code::PageDown,
        0x7a => Code::F1,
        0x7b => Code::ArrowLeft,
        0x7c => Code::ArrowRight,
        0x7d => Code::ArrowDown,
        0x7e => Code::ArrowUp,
        _ => Code::Unidentified,
    }
}