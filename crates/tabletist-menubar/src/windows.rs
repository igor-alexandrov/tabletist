//! The bar on the window: muda subclasses it, which it marks unsafe.

#![allow(unsafe_code)]

use raw_window_handle::RawWindowHandle;

fn hwnd(window: RawWindowHandle) -> Option<isize> {
    match window {
        RawWindowHandle::Win32(win32) => Some(win32.hwnd.get()),
        _ => None,
    }
}

pub(crate) fn attach(menu: &muda::Menu, window: RawWindowHandle) -> Option<()> {
    let hwnd = hwnd(window)?;
    // SAFETY: the handle is the live window eframe created, used on its
    // thread (the creation closure and the frames run on the event
    // loop's), and the window outlives the bar, as `MenuBar::attach`
    // asks: eframe drops the app, and with it the bar, before the window.
    unsafe { menu.init_for_hwnd(hwnd) }
        .map_err(|error| log::warn!("could not attach the menu bar: {error}"))
        .ok()
}

pub(crate) fn detach(menu: &muda::Menu, window: RawWindowHandle) {
    let Some(hwnd) = hwnd(window) else {
        return;
    };
    // SAFETY: as `attach`'s; this menu was attached to this window.
    if let Err(error) = unsafe { menu.remove_for_hwnd(hwnd) } {
        log::warn!("could not take the menu bar off: {error}");
    }
}
