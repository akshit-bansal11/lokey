//! Copying a secret to the clipboard, and clearing it afterwards.
//!
//! The copy carries the formats Windows reads to keep content out of Win+V
//! clipboard history and cloud clipboard sync. Clearing is conditional on the
//! clipboard sequence number: if anything else was copied in the meantime, the
//! user's newer copy is left alone.
//!
//! On macOS and Linux the copy is marked for clipboard managers to skip (the
//! ConcealedType marker, or the KDE hint), and the clear compares the
//! clipboard's text with what was copied, since neither has a sequence number.
//! Neither can keep a copy out of macOS Universal Clipboard.

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
    use std::{
        hash::{DefaultHasher, Hash, Hasher},
        sync::{Mutex, PoisonError},
    };

    use arboard::Clipboard;
    #[cfg(target_os = "macos")]
    use arboard::SetExtApple;
    #[cfg(target_os = "linux")]
    use arboard::SetExtLinux;
    use zeroize::Zeroizing;

    use crate::{Error, Result};

    /// Kept for the life of the app: on Linux the copied text is served by this
    /// process, so dropping the clipboard would take the copy away with it.
    static CLIPBOARD: Mutex<Option<Clipboard>> = Mutex::new(None);

    fn failed(err: arboard::Error) -> Error {
        Error::Io(std::io::Error::other(err.to_string()))
    }

    /// macOS and Linux hand out no clipboard sequence number, so the "sequence"
    /// is a fingerprint of the copied text: the clear empties the clipboard only
    /// while it still holds that same text.
    fn fingerprint(text: &str) -> u32 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        // Only the low 32 bits fit the sequence slot; that is enough to tell copies apart.
        hasher.finish() as u32
    }

    fn with_clipboard<T>(
        work: impl FnOnce(&mut Clipboard) -> std::result::Result<T, arboard::Error>,
    ) -> Result<T> {
        let mut slot = CLIPBOARD.lock().unwrap_or_else(PoisonError::into_inner);
        let clipboard = match slot.take() {
            Some(clipboard) => slot.insert(clipboard),
            None => slot.insert(Clipboard::new().map_err(failed)?),
        };
        let result = work(clipboard);
        if result.is_err() {
            // A broken connection stays broken: open a fresh clipboard next time.
            *slot = None;
        }
        result.map_err(failed)
    }

    pub fn copy_secret(text: &str) -> Result<u32> {
        with_clipboard(|clipboard| {
            // Asks clipboard managers not to keep it: org.nspasteboard.ConcealedType on
            // macOS, the KDE password-manager hint on Linux.
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            let set = clipboard.set().exclude_from_history();
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            let set = clipboard.set();
            set.text(text)
        })?;
        Ok(fingerprint(text))
    }

    pub fn clear_if_unchanged(sequence: u32) -> Result<bool> {
        with_clipboard(|clipboard| match clipboard.get_text() {
            Ok(text) => {
                let text = Zeroizing::new(text);
                if fingerprint(&text) == sequence {
                    clipboard.clear().map(|()| true)
                } else {
                    Ok(false)
                }
            }
            // There is no text on the clipboard: not ours to clear.
            Err(_) => Ok(false),
        })
    }
}
