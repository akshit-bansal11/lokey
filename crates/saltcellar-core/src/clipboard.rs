//! Copying a secret to the clipboard, and clearing it afterwards.
//!
//! The copy carries the formats Windows reads to keep content out of Win+V
//! clipboard history and cloud clipboard sync. Clearing is conditional on the
//! clipboard sequence number: if anything else was copied in the meantime, the
//! user's newer copy is left alone.

use std::time::Duration;

use crate::Result;

pub const CLEAR_AFTER: Duration = Duration::from_secs(30);

/// Copies `text` and returns the clipboard sequence number afterwards, which
/// `clear_if_unchanged` compares against.
pub fn copy_secret(text: &str) -> Result<u32> {
    imp::copy_secret(text)
}

/// Empties the clipboard if nothing was copied since `sequence`. Returns
/// whether it cleared.
pub fn clear_if_unchanged(sequence: u32) -> Result<bool> {
    imp::clear_if_unchanged(sequence)
}

#[cfg(windows)]
mod imp {
    use std::{iter, ptr, thread, time::Duration};

    use windows_sys::Win32::{
        Foundation::GlobalFree,
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, GetClipboardSequenceNumber, OpenClipboard,
                RegisterClipboardFormatW, SetClipboardData,
            },
            Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock},
            Ole::CF_UNICODETEXT,
        },
    };
    use zeroize::Zeroizing;

    use crate::{Error, Result};

    /// An open clipboard; closed on drop so no path leaves it locked for
    /// every other program on the machine.
    struct Opened;

    impl Opened {
        fn new() -> Result<Self> {
            // Another program may hold the clipboard for a moment; retry briefly.
            for _ in 0..20 {
                // SAFETY: a null owner window is allowed; the call has no other inputs.
                if unsafe { OpenClipboard(ptr::null_mut()) } != 0 {
                    return Ok(Self);
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(Error::Io(std::io::Error::other(
                "the clipboard is busy in another program",
            )))
        }
    }

    impl Drop for Opened {
        fn drop(&mut self) {
            // SAFETY: only reachable after a successful OpenClipboard.
            unsafe { CloseClipboard() };
        }
    }

    fn register(name: &str) -> u32 {
        let wide: Vec<u16> = name.encode_utf16().chain(iter::once(0)).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 string alive for the call.
        unsafe { RegisterClipboardFormatW(wide.as_ptr()) }
    }

    /// Hands a copy of `bytes` to the clipboard under `format`. The clipboard
    /// must be open (the `_opened` witness).
    fn put(_opened: &Opened, format: u32, bytes: &[u8]) -> Result<()> {
        let failed = || Error::Io(std::io::Error::last_os_error());
        // SAFETY: standard clipboard hand-off. The allocation is filled while
        // locked, unlocked, then owned by the system once SetClipboardData
        // succeeds; on failure it is freed here instead.
        unsafe {
            let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
            if handle.is_null() {
                return Err(failed());
            }
            let target = GlobalLock(handle).cast::<u8>();
            if target.is_null() {
                GlobalFree(handle);
                return Err(failed());
            }
            ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
            GlobalUnlock(handle);
            if SetClipboardData(format, handle).is_null() {
                GlobalFree(handle);
                return Err(failed());
            }
        }
        Ok(())
    }

    pub fn copy_secret(text: &str) -> Result<u32> {
        let utf16: Zeroizing<Vec<u8>> = Zeroizing::new(
            text.encode_utf16()
                .chain(iter::once(0))
                .flat_map(u16::to_le_bytes)
                .collect(),
        );
        let opened = Opened::new()?;
        // SAFETY: the clipboard is open (held by `opened`).
        if unsafe { EmptyClipboard() } == 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }
        put(&opened, u32::from(CF_UNICODETEXT), &utf16)?;
        // Windows 10+: keep the value out of clipboard history, cloud sync and
        // clipboard-monitoring tools.
        put(
            &opened,
            register("ExcludeClipboardContentFromMonitorProcessing"),
            &[0],
        )?;
        put(
            &opened,
            register("CanIncludeInClipboardHistory"),
            &0u32.to_le_bytes(),
        )?;
        put(
            &opened,
            register("CanUploadToCloudClipboard"),
            &0u32.to_le_bytes(),
        )?;
        drop(opened);
        // SAFETY: no inputs; reads a counter.
        Ok(unsafe { GetClipboardSequenceNumber() })
    }

    pub fn clear_if_unchanged(sequence: u32) -> Result<bool> {
        // SAFETY: no inputs; reads a counter.
        if unsafe { GetClipboardSequenceNumber() } != sequence {
            return Ok(false);
        }
        let _opened = Opened::new()?;
        // SAFETY: the clipboard is open (held by `_opened`).
        Ok(unsafe { EmptyClipboard() } != 0)
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::{Error, Result};

    pub fn copy_secret(_text: &str) -> Result<u32> {
        Err(Error::Unsupported(
            "copying needs Windows; use --view to print the value",
        ))
    }

    pub fn clear_if_unchanged(_sequence: u32) -> Result<bool> {
        Ok(false)
    }
}
