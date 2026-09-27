//! The Windows Save and Open dialogs, called directly: choosing where a backup
//! goes needs no plugin, and the page keeps no filesystem access at all. The
//! path chosen stays on this side; the page only ever sees it as text.

use std::path::PathBuf;

use tauri::WebviewWindow;

/// Asks where to save a backup. `None` when the person cancels.
pub fn save(window: &WebviewWindow, suggested: &str) -> Option<PathBuf> {
    imp::run(window, suggested, true)
}

/// Asks which backup to restore. `None` when the person cancels.
pub fn open(window: &WebviewWindow) -> Option<PathBuf> {
    imp::run(window, "", false)
}

#[cfg(windows)]
mod imp {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf, ptr};

    use tauri::WebviewWindow;
    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR,
        OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };

    const FILTER: &str = "lokey backup (*.lokey)\0*.lokey\0All files\0*.*\0\0";
    /// Long paths are allowed on Windows 10 and later.
    const MAX_PATH_UNITS: usize = 32 * 1024;

    pub fn run(window: &WebviewWindow, initial: &str, save: bool) -> Option<PathBuf> {
        let filter: Vec<u16> = FILTER.encode_utf16().collect();
        let extension: Vec<u16> = "lokey\0".encode_utf16().collect();
        let mut buffer = vec![0u16; MAX_PATH_UNITS];
        for (slot, unit) in buffer.iter_mut().zip(initial.encode_utf16()) {
            *slot = unit;
        }
        // Owned by the app window, so the dialog is modal to it.
        let owner = window.hwnd().map_or(ptr::null_mut(), |hwnd| hwnd.0);

        // SAFETY: every field of OPENFILENAMEW is an integer, a pointer or an
        // optional function pointer, for all of which zero is valid.
        let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        dialog.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        dialog.hwndOwner = owner;
        dialog.lpstrFilter = filter.as_ptr();
        dialog.lpstrFile = buffer.as_mut_ptr();
        dialog.nMaxFile = MAX_PATH_UNITS as u32;
        dialog.lpstrDefExt = extension.as_ptr();
        dialog.Flags = OFN_EXPLORER
            | OFN_NOCHANGEDIR
            | OFN_PATHMUSTEXIST
            | if save {
                OFN_OVERWRITEPROMPT
            } else {
                OFN_FILEMUSTEXIST
            };

        // SAFETY: `dialog` points at buffers that outlive the call, and
        // `nMaxFile` is the length of `buffer`.
        let chosen = unsafe {
            if save {
                GetSaveFileNameW(&mut dialog)
            } else {
                GetOpenFileNameW(&mut dialog)
            }
        };
        if chosen == 0 {
            return None;
        }
        let len = buffer.iter().position(|&unit| unit == 0)?;
        Some(PathBuf::from(OsString::from_wide(&buffer[..len])))
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::PathBuf;

    use tauri::WebviewWindow;

    /// lokey ships for Windows only; elsewhere there is no dialog to show.
    pub fn run(_window: &WebviewWindow, _initial: &str, _save: bool) -> Option<PathBuf> {
        None
    }
}
