use rosin_core::{
    keyboard_types::{Code, KeyboardEvent, Location, Modifiers},
    kurbo::{Point, Vec2},
    prelude::{Key, KeyState, NamedKey, PointerButton, PointerButtons, PointerEvent, PointerType},
};
use x11rb::protocol::xproto::{ButtonPressEvent, ButtonReleaseEvent, KeyButMask, KeyPressEvent, KeyReleaseEvent, MotionNotifyEvent};
use xkbcommon::xkb;
use crate::linux::x11;

pub(crate) fn panic_and_print(msg: String) -> ! {
    println!("{}", msg);
    std::process::abort()
}

pub(crate) fn convert_keyboard_event_pressed_x11(kpe: &KeyPressEvent) -> KeyboardEvent {
    let c = convert_code_x11(kpe.detail as u16);
    let k = convert_key(c);
    println!("{:?}", c);
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
        state: KeyState::Up,
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
    if modifiers.contains(KeyButMask::MOD1) {
        retval = retval | Modifiers::ALT;
    }
    if modifiers.contains(KeyButMask::MOD2) {
        retval = retval | Modifiers::NUM_LOCK;
    }
    if modifiers.contains(KeyButMask::LOCK) {
        retval = retval | Modifiers::CAPS_LOCK;
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
// https://github.com/xkbcommon/libxkbcommon/blob/6e4f0fb9e7ee876f14aad07dda4d69a622c58a3b/include/xkbcommon/xkbcommon-keysyms.h
fn convert_code_x11(key_code: u16) -> Code {
    
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

    let keymap = xkb::Keymap::new_from_names(
        &context,
        "",                                          // rules
        "pc105",                                     // model
        "",                                        // layout
        "",                                    // variant
        Some("terminate:ctrl_alt_bksp".to_string()), // options
        xkb::COMPILE_NO_FLAGS,
    )
    .unwrap();

    let mut state = xkb::State::new(&keymap);
    let keysym = state.key_get_one_sym(key_code.into());
    match u32::from(keysym) {
        0x30 => Code::Digit0,
        0x31 => Code::Digit1,
        0x32 => Code::Digit2,
        0x33 => Code::Digit3,
        0x34 => Code::Digit4,
        0x35 => Code::Digit5,
        0x36 => Code::Digit6,
        0x37 => Code::Digit7,
        0x38 => Code::Digit8,
        0x39 => Code::Digit9,
        0x5b => Code::BracketLeft,
        0x5c => Code::Backslash,
        0x5d => Code::BracketRight,
        0x61 => Code::KeyA,
        0x62 => Code::KeyB,
        0x63 => Code::KeyC,
        0x64 => Code::KeyD,
        0x65 => Code::KeyE,
        0x66 => Code::KeyF,
        0x67 => Code::KeyG,
        0x68 => Code::KeyH,
        0x69 => Code::KeyI,
        0x6a => Code::KeyJ,
        0x6b => Code::KeyK,
        0x6c => Code::KeyL,
        0x6d => Code::KeyM,
        0x6e => Code::KeyN,
        0x6f => Code::KeyO,
        0x70 => Code::KeyP,
        0x71 => Code::KeyQ,
        0x72 => Code::KeyR,
        0x73 => Code::KeyS,
        0x74 => Code::KeyT,
        0x75 => Code::KeyU,
        0x76 => Code::KeyV,
        0x77 => Code::KeyW,
        0x78 => Code::KeyY,
        0x79 => Code::KeyX,
        0x7a => Code::KeyZ,
        0x3d => Code::Equal,
        0x2d => Code::Minus,
        0xff8d => Code::Enter,
        0x22 => Code::Quote,
        0x3b => Code::Semicolon,
        0x2c => Code::Comma,
        0x2f => Code::Slash,
        0x2e => Code::Period,
        0xff09 => Code::Tab,
        0xffbd => Code::NumpadEqual,
        0xffb0 => Code::Numpad0,
        0xffb1 => Code::Numpad1,
        0xffb2 => Code::Numpad2,
        0xffb3 => Code::Numpad3,
        0xffb4 => Code::Numpad4,
        0xffb5 => Code::Numpad5,
        0xffb6 => Code::Numpad6,
        0xffb7 => Code::Numpad7,
        0xffb8 => Code::Numpad8,
        0xffb9 => Code::Numpad9,
        0xffae => Code::NumpadDecimal,
        0xffaa => Code::NumpadMultiply,
        0xffab => Code::NumpadAdd,
        0x31 => Code::Space,
        0x32 => Code::Backquote,
        0xff08 => Code::Backspace,
        0xff8d => Code::NumpadEnter,
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
        0x47 => Code::NumLock,
        0x48 => Code::AudioVolumeUp,
        0x49 => Code::AudioVolumeDown,
        0x4a => Code::AudioVolumeMute,
        0xffaf => Code::NumpadDivide,
        0xffad => Code::NumpadSubtract,
        0x4f => Code::F18,
        0x50 => Code::F19,
        0x5a => Code::F20,
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
