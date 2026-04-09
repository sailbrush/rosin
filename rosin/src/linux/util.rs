use crate::{dialog::FileDialogOptions, prelude::Modifiers};
use rosin_core::{
    keyboard_types::{Code, KeyboardEvent, Location},
    prelude::{Key, NamedKey},
};
use wayland_client::WEnum;
use wayland_client::protocol::wl_keyboard::KeyState;
use xkbcommon::xkb;
pub(crate) fn panic_and_print(msg: String) -> ! {
    println!("{}", msg);
    std::process::abort()
}

pub(crate) fn convert_wayland_key(key: u32, state: WEnum<KeyState>, mods: u32) -> KeyboardEvent {
    let xkb_key = convert_code(key + 8);
    let k = convert_key(xkb_key);
    let mut repeat = false;
    let s = match state {
        WEnum::Value(sta) => match sta {
            KeyState::Released => rosin_core::keyboard_types::KeyState::Up,
            KeyState::Pressed => rosin_core::keyboard_types::KeyState::Down,
            KeyState::Repeated => {
                repeat = true;
                rosin_core::keyboard_types::KeyState::Down
            }
            _ => rosin_core::keyboard_types::KeyState::Up,
        },
        _ => rosin_core::keyboard_types::KeyState::Up,
    };
    KeyboardEvent {
        code: xkb_key,
        key: if k.is_some() { k.unwrap() } else { Key::Character(xkb_key.to_string()) },
        is_composing: false,
        location: convert_location(xkb_key),
        modifiers: convert_modifiers(mods),
        repeat,
        state: s,
    }
}
fn convert_modifiers(mods: u32) -> Modifiers {
    let mut retval = Modifiers::default();
    if mods & 1 == 1 {
        retval |= Modifiers::SHIFT;
    }
    if mods & 8 == 8 {
        retval |= Modifiers::ALT;
    }
    println!("{:?}", mods);
    retval
}

fn to_char(s: &str) -> char {
    match s {
        "KeyA" => 'a',
        "KeyB" => 'b',
        "KeyC" => 'c',
        "KeyD" => 'd',
        "KeyE" => 'e',
        "KeyF" => 'f',
        "KeyG" => 'g',
        "KeyH" => 'h',
        "KeyI" => 'i',
        "KeyJ" => 'j',
        "KeyK" => 'k',
        "KeyL" => 'l',
        "KeyM" => 'm',
        "KeyN" => 'n',
        "KeyO" => 'o',
        "KeyP" => 'p',
        "KeyQ" => 'q',
        "KeyR" => 'r',
        "KeyS" => 's',
        "KeyT" => 't',
        "KeyU" => 'u',
        "KeyV" => 'v',
        "KeyW" => 'w',
        "KeyX" => 'x',
        "KeyY" => 'y',
        "KeyZ" => 'z',
        "Digit0" => '0',
        "Digit1" => '1',
        "Digit2" => '2',
        "Digit3" => '3',
        "Digit4" => '4',
        "Digit5" => '5',
        "Digit6" => '6',
        "Digit7" => '7',
        "Digit8" => '8',
        "Digit9" => '9',
        "Comma" => ',',
        "Semicolon" => ';',
        "Period" => '.',
        "Slash" => '/',
        "Backslash" => '\\',
        "BracketRight" => ']',
        "BracketLeft" => '[',
        "Minus" => '-',
        "Equal" => '=',
        _ => {
            println!("{:?}", s);
            ' '
        }
    }
}
pub fn valid_char(c: char) -> bool {
    (c as u32) >= 32 && (c as u32) < 127
}
pub fn kb_event_to_str(kbe: &KeyboardEvent) -> String {
    let mut retval = String::new();
    let mut c = char::from_u32(match kbe.key {
        // See: https://w3c.github.io/uievents/#fixed-virtual-key-codes
        Key::Named(NamedKey::Backspace) => 8,
        Key::Named(NamedKey::Tab) => 9,
        Key::Named(NamedKey::Enter) => 13,
        Key::Named(NamedKey::Shift) => 16,
        Key::Named(NamedKey::Control) => 17,
        Key::Named(NamedKey::Alt) => 18,
        Key::Named(NamedKey::CapsLock) => 20,
        Key::Named(NamedKey::Escape) => 27,
        Key::Named(NamedKey::PageUp) => 33,
        Key::Named(NamedKey::PageDown) => 34,
        Key::Named(NamedKey::End) => 35,
        Key::Named(NamedKey::Home) => 36,
        Key::Named(NamedKey::ArrowLeft) => 37,
        Key::Named(NamedKey::ArrowUp) => 38,
        Key::Named(NamedKey::ArrowRight) => 39,
        Key::Named(NamedKey::ArrowDown) => 40,
        Key::Named(NamedKey::Delete) => 46,
        Key::Character(ref c) => to_char(c) as u32,
        _ => 0,
    })
    .unwrap();
    if kbe.modifiers.shift() {
        c.make_ascii_uppercase();
    } else {
        c.make_ascii_lowercase();
    }
    retval.push(c);
    println!("{:?}", c);
    retval
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
fn convert_code(key_code: u32) -> Code {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

    //empty strings indicates system default
    let keymap = xkb::Keymap::new_from_names(
        &context,
        "",                                          // rules
        "",                                          // model
        "",                                          // layout
        "",                                          // variant
        Some("terminate:ctrl_alt_bksp".to_string()), // options
        xkb::COMPILE_NO_FLAGS,
    )
    .unwrap();

    let state = xkb::State::new(&keymap);
    let keysym = state.key_get_one_sym(key_code.into());
    match u32::from(keysym) {
        // function keys
        0x8f6 => Code::Fn,
        0xffbe => Code::F1,
        0xffbf => Code::F2,
        0xffc0 => Code::F3,
        0xffc1 => Code::F4,
        0xffc2 => Code::F5,
        0xffc3 => Code::F6,
        0xffc4 => Code::F7,
        0xffc5 => Code::F8,
        0xffc6 => Code::F9,
        0xffc7 => Code::F10,
        0xffc8 => Code::F11,
        0xffc9 => Code::F12,
        0xffca => Code::F13,
        0xffcb => Code::F14,
        0xffcc => Code::F15,
        0xffcd => Code::F16,
        0xffce => Code::F17,
        0xffcf => Code::F18,
        0xffd0 => Code::F19,
        0xffd1 => Code::F20,
        // digits
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
        // alphabet
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
        // punctuation
        0x22 => Code::Quote,
        0x3b => Code::Semicolon,
        0x2c => Code::Comma,
        0x2d => Code::Minus,
        0x2e => Code::Period,
        0x2f => Code::Slash,
        0x3d => Code::Equal,
        // Numpad
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
        0xff08 => Code::Backspace,
        0xff8d => Code::NumpadEnter,
        0xffaf => Code::NumpadDivide,
        0xffad => Code::NumpadSubtract,
        0x5f => Code::NumpadComma,
        //control characters
        0x20 => Code::Space,
        0xff09 => Code::Tab,
        0x60 => Code::Backquote,
        0xff1b => Code::Escape,
        0xff0d => Code::Enter,
        // modifiers
        0xffe1 => Code::ShiftLeft,
        0xffe2 => Code::ShiftRight,
        0xffe3 => Code::ControlLeft,
        0xffe4 => Code::ControlRight,
        0xffe5 => Code::CapsLock,
        0xffe7 => Code::MetaLeft,
        0xffe8 => Code::MetaRight,
        0xffe9 => Code::AltLeft,
        0xffea => Code::AltRight,
        0xff7f => Code::NumLock,
        0x1008ff13 => Code::AudioVolumeUp,
        0x1008ff11 => Code::AudioVolumeDown,
        0x1008ff12 => Code::AudioVolumeMute,
        0x00a5 => Code::IntlYen,
        //0x5e => Code::IntlRo,
        //0x66 => Code::Lang2,
        //0x68 => Code::Lang1,
        0xff67 => Code::ContextMenu,
        0xff9e => Code::Insert,
        0xff95 => Code::Home,
        0xff9a => Code::PageUp,
        0xffff => Code::Delete,
        0xff9c => Code::End,
        // arrows
        0xff9b => Code::PageDown,
        0xff51 => Code::ArrowLeft,
        0xff53 => Code::ArrowRight,
        0xff54 => Code::ArrowDown,
        0xff52 => Code::ArrowUp,
        _ => Code::Unidentified,
    }
}

use crate::prelude::PointerButton;
pub fn linux_mouse_btn_convert(btn: u16) -> PointerButton {
    if btn == 0x110 {
        return PointerButton::from(1);
    }

    if btn == 0x111 {
        return PointerButton::from(2);
    }

    if btn == 0x112 {
        return PointerButton::from(3);
    }

    if btn == 0x116 {
        return PointerButton::from(4);
    }

    if btn == 0x117 {
        return PointerButton::from(5);
    }
    PointerButton::from(0)
}
use wayland_protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;
pub fn csd_resize_to_wayland(edge: wayland_csd_frame::ResizeEdge) -> wayland_protocols::xdg::shell::client::xdg_toplevel::ResizeEdge {
    match edge {
        wayland_csd_frame::ResizeEdge::None => ResizeEdge::None,
        wayland_csd_frame::ResizeEdge::Top => ResizeEdge::Top,
        wayland_csd_frame::ResizeEdge::Bottom => ResizeEdge::Bottom,
        wayland_csd_frame::ResizeEdge::Left => ResizeEdge::Left,
        wayland_csd_frame::ResizeEdge::TopLeft => ResizeEdge::TopLeft,
        wayland_csd_frame::ResizeEdge::BottomLeft => ResizeEdge::BottomLeft,
        wayland_csd_frame::ResizeEdge::Right => ResizeEdge::Right,
        wayland_csd_frame::ResizeEdge::TopRight => ResizeEdge::TopRight,
        wayland_csd_frame::ResizeEdge::BottomRight => ResizeEdge::BottomRight,
        _ => ResizeEdge::None,
    }
}
use crate::prelude::CursorType;
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;
pub(crate) fn cursor_icon_to_shape(cursor_icon: CursorType) -> Shape {
    match cursor_icon {
        CursorType::Default => Shape::Default,
        CursorType::ContextMenu => Shape::ContextMenu,
        CursorType::Help => Shape::Help,
        CursorType::Pointer => Shape::Pointer,
        CursorType::Cell => Shape::Cell,
        CursorType::Crosshair => Shape::Crosshair,
        CursorType::Text => Shape::Text,
        CursorType::VerticalText => Shape::VerticalText,
        CursorType::Alias => Shape::Alias,
        CursorType::Copy => Shape::Copy,
        CursorType::Move => Shape::Move,
        CursorType::NotAllowed => Shape::NotAllowed,
        CursorType::Grab => Shape::Grab,
        CursorType::Grabbing => Shape::Grabbing,
        CursorType::EResize => Shape::EResize,
        CursorType::NResize => Shape::NResize,
        CursorType::NEResize => Shape::NeResize,
        CursorType::NWResize => Shape::NwResize,
        CursorType::SResize => Shape::SResize,
        CursorType::SEResize => Shape::SeResize,
        CursorType::SWResize => Shape::SwResize,
        CursorType::WResize => Shape::WResize,
        CursorType::EWResize => Shape::EwResize,
        CursorType::NSResize => Shape::NsResize,
        CursorType::NESWResize => Shape::NeswResize,
        CursorType::NWSEResize => Shape::NwseResize,
        CursorType::ColResize => Shape::ColResize,
        CursorType::RowResize => Shape::RowResize,
        CursorType::ZoomIn => Shape::ZoomIn,
        CursorType::ZoomOut => Shape::ZoomOut,
        _ => Shape::Default,
    }
}
use serde::*;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::ffi::{CStr, CString};
use std::fmt;
use std::fmt::Debug;
use zbus::zvariant::{DeserializeDict, SerializeDict};
use std::ffi::c_uint;
use std::ffi::c_void;
use zbus::zvariant::{
    Optional, Type,
    as_value::{self, optional},
};
unsafe extern "C" {
    pub unsafe fn getrandom(buf: *mut c_void, buflen: usize, flags: c_uint) -> usize;
}

#[derive(Serialize, Type, PartialEq, Eq, Hash, Clone)]
pub struct HandleToken(zbus::names::OwnedMemberName);
impl Default for HandleToken {
    fn default() -> Self {
        const ALPHANUMERIC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

        let mut token = String::with_capacity(16); // "rosin_" + 10 chars
        token.push_str("rosin_");

        let mut rnd_bytes = [0u8; 10];
        unsafe {
            getrandom(rnd_bytes.as_mut_ptr() as *mut c_void, rnd_bytes.len(), 0);
        }
        for byte in rnd_bytes.iter() {
            let idx = (*byte as usize) % ALPHANUMERIC.len();
            token.push(ALPHANUMERIC[idx] as char);
        }

        println!("{:?}", token);
        Self(zbus::names::OwnedMemberName::try_from(token).unwrap())
    }
}
impl Debug for HandleToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("HandleToken").field(&self.0.as_str()).finish()
    }
}
#[derive(Clone, Serialize, Deserialize, Type, Debug)]
pub struct Choice(String, String, Vec<(String, String)>, String);

#[derive(Clone, Serialize, Deserialize, Type, Debug, PartialEq)]
pub struct FileFilter(String, Vec<(FilterType, String)>);

#[derive(Clone, Serialize_repr, Deserialize_repr, Debug, Type, PartialEq)]
#[repr(u32)]
enum FilterType {
    GlobPattern = 0,
    MimeType = 1,
}

#[derive(Type, Debug, Default, PartialEq)]
#[zvariant(signature = "ay")]
pub struct FilePath(CString);

impl Serialize for FilePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.0.as_bytes_with_nul())
    }
}

impl<'de> Deserialize<'de> for FilePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = <Vec<u8>>::deserialize(deserializer)?;
        let c_string = CString::from_vec_with_nul(bytes)
            .map_err(|_| serde::de::Error::custom("Bytes are not nul-terminated"))?;

        Ok(Self(c_string))
    }
}

#[derive(Serialize, Deserialize, Type, Debug, Default)]
#[zvariant(signature = "dict")]
pub struct OpenFileOptions {
    #[serde(with = "as_value", skip_deserializing)]
    handle_token: HandleToken,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    accept_label: Option<String>,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    modal: Option<bool>,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    multiple: Option<bool>,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    directory: Option<bool>,
    #[serde(default, with = "as_value", skip_serializing_if = "Vec::is_empty")]
    filters: Vec<FileFilter>,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    current_filter: Option<FileFilter>,
    #[serde(default, with = "as_value", skip_serializing_if = "Vec::is_empty")]
    choices: Vec<Choice>,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    current_folder: Option<FilePath>,
}

pub fn file_dialog_to_open(opt: FileDialogOptions) -> OpenFileOptions {
    OpenFileOptions {
        handle_token: Default::default(),
        accept_label: opt.submit_label,
        modal: None,
        multiple: Some(opt.allow_multiple),
        directory: Some(opt.pick_folders),
        filters: vec![],
        current_filter: None,
        choices: vec![],
        current_folder: Some(
            FilePath(CString::new(if opt.initial_path.is_some() {
                opt.initial_path.as_ref().unwrap().to_str().unwrap()
            } else {
                ""
            })
            .unwrap(),
        )),
    }
}
#[derive(Serialize, Deserialize, Type, Debug, Default)]
#[zvariant(signature = "dict")]
pub struct SaveFileOptions {
    #[serde(with = "as_value", skip_deserializing)]
    handle_token: HandleToken,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    accept_label: Option<String>,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    modal: Option<bool>,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    current_name: Option<String>,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    current_folder: Option<FilePath>,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    current_file: Option<FilePath>,
    #[serde(default, with = "as_value", skip_serializing_if = "Vec::is_empty")]
    filters: Vec<FileFilter>,
    #[serde(default, with = "optional", skip_serializing_if = "Option::is_none")]
    current_filter: Option<FileFilter>,
    #[serde(default, with = "as_value", skip_serializing_if = "Vec::is_empty")]
    choices: Vec<Choice>,
}
pub fn file_dialog_to_save(opt: FileDialogOptions) -> SaveFileOptions {
    SaveFileOptions {
        handle_token: Default::default(),
        accept_label: opt.submit_label,
        modal: None,
        filters: vec![],
        current_filter: None,
        choices: vec![],
        current_folder: Some(
            FilePath(CString::new(if opt.initial_path.is_some() {
                opt.initial_path.as_ref().unwrap().to_str().unwrap()
            } else {
                ""
            })
            .unwrap(),
        )),
        current_file: Some(
            FilePath(CString::new(if opt.initial_path.is_some() {
                opt.initial_path.as_ref().unwrap().to_str().unwrap()
            } else {
                ""
            })
            .unwrap(),
        )),
        current_name: opt.filename_label
    }
}