//! The key-down AppKit is handling: what tells a key equivalent from a
//! click. Safe calls only; the menus' delegate is in `opening`.

mod opening;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSEventType};

pub(crate) use opening::{Opening, watch};

use crate::KeyDown;

pub(crate) fn key_down() -> Option<KeyDown> {
    let mtm = MainThreadMarker::new()?;
    let event = NSApplication::sharedApplication(mtm).currentEvent()?;
    if event.r#type() != NSEventType::KeyDown {
        return None;
    }
    let flags = event.modifierFlags();
    Some(KeyDown {
        chars: event
            .charactersIgnoringModifiers()
            .map(|chars| chars.to_string())
            .unwrap_or_default(),
        cmd: flags.contains(NSEventModifierFlags::Command),
        ctrl: flags.contains(NSEventModifierFlags::Control),
        alt: flags.contains(NSEventModifierFlags::Option),
        shift: flags.contains(NSEventModifierFlags::Shift),
    })
}
