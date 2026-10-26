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
    // SAFETY: the handle names a live window, used on its thread (the
    // creation closure and the frames run on the event loop's): the
    // window eframe lends `MenuBar::attach`, or at a rebuild one that
    // `detach` just took the old bar off, which muda would have
    // forgotten had it been destroyed.
    unsafe { menu.init_for_hwnd(hwnd) }
        .map_err(|error| log::warn!("could not attach the menu bar: {error}"))
        .ok()
}

/// Takes `menu` off `window`. False if muda does not have it there: muda
/// forgets a window as it is destroyed, and then makes no call with it.
pub(crate) fn detach(menu: &muda::Menu, window: RawWindowHandle) -> bool {
    let Some(hwnd) = hwnd(window) else {
        return false;
    };
    // SAFETY: as `attach`'s. muda looks the window up first, and calls
    // Windows with it only if it still has it, alive.
    match unsafe { menu.remove_for_hwnd(hwnd) } {
        Ok(()) => true,
        Err(error) => {
            log::warn!("could not take the menu bar off: {error}");
            false
        }
    }
}
