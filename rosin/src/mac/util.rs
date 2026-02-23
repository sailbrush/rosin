use std::{
    ffi::{CStr, OsString},
    ops::Range,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
    ptr::NonNull,
};

use objc2::{
    AnyThread, ClassType, MainThreadMarker, msg_send,
    rc::{Retained, autoreleasepool},
    runtime::{AnyObject, Sel},
    sel,
};
use objc2_app_kit::{NSAlert, NSAlertStyle, NSApp, NSCursor, NSEvent, NSEventModifierFlags, NSEventSubtype, NSEventType, NSImage, NSView};
use objc2_foundation::{NSAttributedString, NSData, NSNotFound, NSPoint, NSRange, NSString, NSURL};
use rosin_core::{
    keyboard_types::{Code, Key, KeyState, KeyboardEvent, Location, Modifiers, NamedKey},
    pointer::{PointerButton, PointerButtons, PointerEvent, PointerType},
    vello::kurbo::{Point, Vec2},
};

use crate::prelude::*;

pub(crate) fn fatal_alert_and_quit(mtm: MainThreadMarker, title: &str, details: &str) -> ! {
    let title = NSString::from_str(title);
    let details = NSString::from_str(details);

    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Critical);
    alert.setMessageText(&title);
    alert.setInformativeText(&details);

    let window = alert.window();
    window.center();
    window.makeKeyAndOrderFront(None);

    let _ = alert.runModal();

    NSApp(mtm).terminate(None);
    std::process::abort()
}

pub(crate) fn convert_pointer_event(ns_event: &NSEvent, ns_view: &NSView) -> PointerEvent {
    let event_type = ns_event.r#type();

    let pointer_pos = if let Some(ns_window) = ns_view.window() {
        let screen_pos = NSEvent::mouseLocation();
        let window_pos = ns_window.convertPointFromScreen(screen_pos);
        ns_view.convertPoint_fromView(window_pos, None)
    } else {
        ns_view.convertPoint_fromView(ns_event.locationInWindow(), None)
    };

    let wheel_delta = if event_type == NSEventType::ScrollWheel {
        let dx = -ns_event.scrollingDeltaX();
        let dy = -ns_event.scrollingDeltaY();
        if ns_event.hasPreciseScrollingDeltas() {
            Vec2::new(dx, dy)
        } else {
            Vec2::new(dx * 32.0, dy * 32.0)
        }
    } else {
        Vec2::ZERO
    };

    let button = match event_type {
        NSEventType::LeftMouseDown
        | NSEventType::RightMouseDown
        | NSEventType::OtherMouseDown
        | NSEventType::LeftMouseUp
        | NSEventType::RightMouseUp
        | NSEventType::OtherMouseUp
        | NSEventType::LeftMouseDragged
        | NSEventType::RightMouseDragged
        | NSEventType::OtherMouseDragged => (ns_event.buttonNumber() + 1).into(),
        _ => PointerButton::None,
    };

    let count = match event_type {
        NSEventType::LeftMouseDown
        | NSEventType::RightMouseDown
        | NSEventType::OtherMouseDown
        | NSEventType::LeftMouseUp
        | NSEventType::RightMouseUp
        | NSEventType::OtherMouseUp => ns_event.clickCount() as u8,
        _ => 0,
    };

    let pressure;
    let tangential_pressure;
    let tilt;
    let twist;
    let pointer_type;
    let subtype = ns_event.subtype();
    if matches!(subtype, NSEventSubtype::TabletPoint | NSEventSubtype::TabletProximity) && event_type != NSEventType::ScrollWheel {
        pressure = ns_event.pressure();
        tangential_pressure = ns_event.tangentialPressure();
        let ns_tilt = ns_event.tilt();
        tilt = Vec2::new(ns_tilt.x, ns_tilt.y);
        twist = ns_event.rotation();
        pointer_type = PointerType::Pen;
    } else {
        pressure = 0.;
        tangential_pressure = 0.;
        tilt = Vec2::ZERO;
        twist = 0.;
        pointer_type = PointerType::Mouse;
    };

    PointerEvent {
        viewport_pos: Point::new(pointer_pos.x, pointer_pos.y),
        wheel_delta,
        button,
        buttons: PointerButtons::from(NSEvent::pressedMouseButtons() as u8),
        mods: convert_modifiers(ns_event.modifierFlags()),
        count,
        did_focus_window: false,
        pressure,
        tangential_pressure,
        tilt,
        twist,
        pointer_type,
    }
}

pub(crate) fn convert_keyboard_event(event: &NSEvent, is_composing: bool) -> Option<KeyboardEvent> {
    let event_type = event.r#type();
    // We must extract the code early to determine which modifier changed
    let code = convert_code(event.keyCode());

    let state = match event_type {
        NSEventType::KeyDown => KeyState::Down,
        NSEventType::KeyUp => KeyState::Up,
        NSEventType::FlagsChanged => {
            let flags = event.modifierFlags();
            // We check if the specific flag for this key code is present.
            // If the flag is present, the key is Down (or Locked for Caps).
            // If the flag is absent, the key is Up.
            let is_down = match code {
                Code::CapsLock => flags.contains(NSEventModifierFlags::CapsLock),
                Code::ShiftLeft | Code::ShiftRight => flags.contains(NSEventModifierFlags::Shift),
                Code::ControlLeft | Code::ControlRight => flags.contains(NSEventModifierFlags::Control),
                Code::AltLeft | Code::AltRight => flags.contains(NSEventModifierFlags::Option),
                Code::MetaLeft | Code::MetaRight => flags.contains(NSEventModifierFlags::Command),
                Code::Fn => flags.contains(NSEventModifierFlags::Function),
                _ => false,
            };

            if is_down { KeyState::Down } else { KeyState::Up }
        }
        _ => return None,
    };

    let key = convert_key(code)
        .or_else(|| {
            // Important: FlagsChanged events do not usually contain valid characters.
            // Accessing .characters() on them can be unsafe or return unexpected empty strings.
            if event_type == NSEventType::FlagsChanged {
                return None;
            }

            let characters = event.characters().as_deref().map(NSString::to_string).unwrap_or_default();

            if is_valid_key(&characters) {
                Some(Key::Character(characters))
            } else {
                let characters_ignoring = event.charactersIgnoringModifiers().as_deref().map(NSString::to_string).unwrap_or_default();

                if is_valid_key(&characters_ignoring) {
                    Some(Key::Character(characters_ignoring))
                } else {
                    None
                }
            }
        })
        .unwrap_or(Key::Named(NamedKey::Unidentified));

    Some(KeyboardEvent {
        state,
        key,
        code,
        location: convert_location(code),
        modifiers: convert_modifiers(event.modifierFlags()),
        repeat: event_type == NSEventType::KeyDown && event.isARepeat(),
        is_composing,
    })
}

const MODIFIER_MAP: &[(NSEventModifierFlags, Modifiers)] = &[
    (NSEventModifierFlags::CapsLock, Modifiers::CAPS_LOCK),
    (NSEventModifierFlags::Shift, Modifiers::SHIFT),
    (NSEventModifierFlags::Control, Modifiers::CONTROL),
    (NSEventModifierFlags::Option, Modifiers::ALT),
    (NSEventModifierFlags::Command, Modifiers::META),
    (NSEventModifierFlags::NumericPad, Modifiers::NUM_LOCK),
    (NSEventModifierFlags::Function, Modifiers::FN),
];

fn convert_modifiers(raw_flags: NSEventModifierFlags) -> Modifiers {
    let mut mods = Modifiers::empty();
    MODIFIER_MAP.iter().for_each(|&(flag, modifier)| {
        if raw_flags.contains(flag) {
            mods.insert(modifier);
        }
    });
    mods
}

// Refer to https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/code/code_values
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

// When this returns None, the code can be considered printable.
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

fn is_valid_key(s: &str) -> bool {
    match s.chars().next() {
        None => false,
        Some(c) => c >= ' ' && c != '\x7f' && !('\u{e000}'..'\u{f900}').contains(&c),
    }
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

pub(crate) fn load_cursor_from_pdf(path: &str, hotspot_x: f64, hotspot_y: f64) -> Option<Retained<NSCursor>> {
    autoreleasepool(|_| {
        let data = NSData::dataWithContentsOfFile(&NSString::from_str(path))?;
        let ns_image: Retained<NSImage> = NSImage::initWithData(NSImage::alloc(), &data)?;
        let hotspot = NSPoint::new(hotspot_x, hotspot_y);
        Some(NSCursor::initWithImage_hotSpot(NSCursor::alloc(), &ns_image, hotspot))
    })
}

pub(crate) fn set_private_cursor(sel: Sel) {
    unsafe {
        let cls = NSCursor::class();

        let responds: bool = msg_send![cls, respondsToSelector: sel];
        if !responds {
            NSCursor::arrowCursor().set();
            return;
        }

        let cur_ptr: *mut NSCursor = msg_send![cls, performSelector: sel];
        let cur = Retained::retain_autoreleased(cur_ptr);

        if let Some(cur) = cur {
            cur.set();
        } else {
            NSCursor::arrowCursor().set();
        }
    }
}

pub(crate) fn nsurl_to_pathbuf(url: &NSURL) -> Option<PathBuf> {
    unsafe {
        let c_path: *const std::ffi::c_char = msg_send![url, fileSystemRepresentation];
        if c_path.is_null() {
            return None;
        }
        let bytes = CStr::from_ptr(c_path).to_bytes();
        Some(PathBuf::from(OsString::from_vec(bytes.to_vec())))
    }
}

pub(crate) fn path_to_nsurl(path: &std::path::Path, is_dir: bool) -> Option<Retained<NSURL>> {
    let ptr = path.as_os_str().as_bytes().as_ptr() as *mut i8;

    unsafe {
        let bytes = NonNull::new_unchecked(ptr);
        Some(NSURL::initFileURLWithFileSystemRepresentation_isDirectory_relativeToURL(NSURL::alloc(), bytes, is_dir, None))
    }
}

pub(crate) fn range_from_ns(ns: NSRange) -> std::ops::Range<usize> {
    if ns.location == NSNotFound as usize {
        return 0..0;
    }
    ns.location..(ns.location.saturating_add(ns.length))
}

pub(crate) fn extract_string(string: &AnyObject) -> String {
    if let Some(str_val) = string.downcast_ref::<NSString>() {
        str_val.to_string()
    } else if let Some(attr_str) = string.downcast_ref::<NSAttributedString>() {
        attr_str.string().to_string()
    } else {
        String::new()
    }
}

// Needed because `InputHandler`'s utf16 conversion methods require the text to be in the document.
// In `set_marked_text`, the text is a standalone string not yet in the document.
pub(crate) fn local_utf16_to_utf8(text: &str, range: Range<usize>) -> Option<Range<usize>> {
    if range.start > range.end {
        return None;
    }

    let start_t = range.start;
    let end_t = range.end;

    #[inline(always)]
    fn snap_between(t: usize, prev_u16: usize, prev_b: usize, cur_u16: usize, cur_b: usize) -> (usize, usize) {
        let dist_prev = t - prev_u16;
        let dist_cur = cur_u16 - t;
        if dist_cur < dist_prev { (cur_u16, cur_b) } else { (prev_u16, prev_b) }
    }

    let mut start_b: Option<usize> = if start_t == 0 { Some(0) } else { None };
    let mut end_b: Option<usize> = if end_t == 0 { Some(0) } else { None };

    let mut prev_u16 = 0usize;
    let mut prev_b = 0usize;

    for (byte_idx, ch) in text.char_indices() {
        debug_assert_eq!(byte_idx, prev_b);

        let u16_len = if (ch as u32) <= 0xFFFF { 1 } else { 2 };
        let b_len = ch.len_utf8();

        let cur_u16 = prev_u16 + u16_len;
        let cur_b = prev_b + b_len;

        if start_b.is_none() && start_t <= cur_u16 {
            start_b = Some(snap_between(start_t, prev_u16, prev_b, cur_u16, cur_b).1);
        }
        if end_b.is_none() && end_t <= cur_u16 {
            end_b = Some(snap_between(end_t, prev_u16, prev_b, cur_u16, cur_b).1);
        }

        if let (Some(start_b), Some(end_b)) = (start_b, end_b) {
            return Some(start_b..end_b);
        }

        prev_u16 = cur_u16;
        prev_b = cur_b;
    }

    let total_u16 = prev_u16;
    let total_b = text.len();

    if start_t > total_u16 || end_t > total_u16 {
        return None;
    }

    let s = start_b.unwrap_or(total_b);
    let e = end_b.unwrap_or(total_b);
    Some(s..e)
}

pub(crate) fn selector_to_action(selector: Sel) -> Option<Action> {
    if selector == sel!(insertNewline:) || selector == sel!(insertNewlineIgnoringFieldEditor:) {
        Some(Action::InsertNewLine)
    } else if selector == sel!(insertTab:) {
        Some(Action::InsertTab)
    } else if selector == sel!(insertBacktab:) {
        Some(Action::InsertBacktab)
    } else if selector == sel!(deleteBackward:) {
        Some(Action::Delete(Movement::Grapheme(HorizontalDirection::Left)))
    } else if selector == sel!(deleteForward:) {
        Some(Action::Delete(Movement::Grapheme(HorizontalDirection::Right)))
    } else if selector == sel!(deleteWordBackward:) {
        Some(Action::Delete(Movement::Word(HorizontalDirection::Left)))
    } else if selector == sel!(deleteWordForward:) {
        Some(Action::Delete(Movement::Word(HorizontalDirection::Right)))
    } else if selector == sel!(deleteToBeginningOfLine:) {
        Some(Action::Delete(Movement::Line(HorizontalDirection::Left)))
    } else if selector == sel!(deleteToEndOfLine:) {
        Some(Action::Delete(Movement::Line(HorizontalDirection::Right)))
    } else if selector == sel!(deleteToBeginningOfParagraph:) {
        Some(Action::Delete(Movement::Paragraph(HorizontalDirection::Left)))
    } else if selector == sel!(deleteToEndOfParagraph:) {
        Some(Action::Delete(Movement::Paragraph(HorizontalDirection::Right)))
    } else if selector == sel!(moveBackward:) || selector == sel!(moveLeft:) {
        Some(Action::Move(Movement::Grapheme(HorizontalDirection::Left)))
    } else if selector == sel!(moveForward:) || selector == sel!(moveRight:) {
        Some(Action::Move(Movement::Grapheme(HorizontalDirection::Right)))
    } else if selector == sel!(moveUp:) {
        Some(Action::Move(Movement::Vertical(VerticalDirection::Up)))
    } else if selector == sel!(moveDown:) {
        Some(Action::Move(Movement::Vertical(VerticalDirection::Down)))
    } else if selector == sel!(scrollPageUp:) {
        Some(Action::Move(Movement::Vertical(VerticalDirection::PageUp)))
    } else if selector == sel!(scrollPageDown:) {
        Some(Action::Move(Movement::Vertical(VerticalDirection::PageDown)))
    } else if selector == sel!(moveWordBackward:) || selector == sel!(moveWordLeft:) {
        Some(Action::Move(Movement::Word(HorizontalDirection::Left)))
    } else if selector == sel!(moveWordForward:) || selector == sel!(moveWordRight:) {
        Some(Action::Move(Movement::Word(HorizontalDirection::Right)))
    } else if selector == sel!(moveToLeftEndOfLine:) {
        Some(Action::Move(Movement::Line(HorizontalDirection::Left)))
    } else if selector == sel!(moveToRightEndOfLine:) {
        Some(Action::Move(Movement::Line(HorizontalDirection::Right)))
    } else if selector == sel!(moveToLeftEndOfLineAndModifySelection:) {
        Some(Action::MoveSelecting(Movement::Line(HorizontalDirection::Left)))
    } else if selector == sel!(moveToRightEndOfLineAndModifySelection:) {
        Some(Action::MoveSelecting(Movement::Line(HorizontalDirection::Right)))
    } else if selector == sel!(moveToBeginningOfDocument:) {
        Some(Action::Move(Movement::Document(HorizontalDirection::Left)))
    } else if selector == sel!(moveToEndOfDocument:) {
        Some(Action::Move(Movement::Document(HorizontalDirection::Right)))
    } else if selector == sel!(moveBackwardAndModifySelection:) || selector == sel!(moveLeftAndModifySelection:) {
        Some(Action::MoveSelecting(Movement::Grapheme(HorizontalDirection::Left)))
    } else if selector == sel!(moveForwardAndModifySelection:) || selector == sel!(moveRightAndModifySelection:) {
        Some(Action::MoveSelecting(Movement::Grapheme(HorizontalDirection::Right)))
    } else if selector == sel!(moveUpAndModifySelection:) {
        Some(Action::MoveSelecting(Movement::Vertical(VerticalDirection::Up)))
    } else if selector == sel!(moveDownAndModifySelection:) {
        Some(Action::MoveSelecting(Movement::Vertical(VerticalDirection::Down)))
    } else if selector == sel!(moveWordBackwardAndModifySelection:) || selector == sel!(moveWordLeftAndModifySelection:) {
        Some(Action::MoveSelecting(Movement::Word(HorizontalDirection::Left)))
    } else if selector == sel!(moveWordForwardAndModifySelection:) || selector == sel!(moveWordRightAndModifySelection:) {
        Some(Action::MoveSelecting(Movement::Word(HorizontalDirection::Right)))
    } else if selector == sel!(cancelOperation:) {
        Some(Action::Cancel)
    } else {
        None
    }
}
