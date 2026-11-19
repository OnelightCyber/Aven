//! Copies a secret so that it stays out of the clipboard history (Win+V)
//! and of the cloud clipboard, and takes it off the clipboard again a little
//! later if nothing else was copied meanwhile.
//!
//! The text is the person's own, copied because they asked: it is never
//! logged, and only the main window may ask.

use tauri::WebviewWindow;

use crate::{authorize, CallerPolicy};

/// The longest text copied this way.
const MAX_BYTES: usize = 64 * 1024;
/// How long a copied secret stays on the clipboard.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
const STAYS: std::time::Duration = std::time::Duration::from_secs(45);

#[cfg(target_os = "windows")]
mod native {
    use windows::core::w;
    use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardSequenceNumber, OpenClipboard,
        RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    const CF_UNICODETEXT: u32 = 13;

    /// Hands `bytes` to the open clipboard under `format`.
    ///
    /// # Safety
    /// The clipboard must be open by this thread. Once set, the memory is
    /// the clipboard's.
    unsafe fn put(format: u32, bytes: &[u8]) -> bool {
        if format == 0 {
            return false;
        }
        let Ok(memory) = GlobalAlloc(GMEM_MOVEABLE, bytes.len()) else {
            return false;
        };
        let at = GlobalLock(memory);
        if at.is_null() {
            let _ = GlobalFree(Some(memory));
            return false;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), at.cast::<u8>(), bytes.len());
        let _ = GlobalUnlock(memory);
        if SetClipboardData(format, Some(HANDLE(memory.0))).is_ok() {
            true
        } else {
            let _ = GlobalFree(Some(memory));
            false
        }
    }

    /// Puts `text` on the clipboard, marked to be left out of the history
    /// and the cloud. Answers the clipboard's sequence number afterwards.
    pub(super) fn copy(owner: Option<HWND>, text: &str) -> Option<u32> {
        let wide = zeroize::Zeroizing::new(
            text.encode_utf16()
                .chain([0])
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<u8>>(),
        );
        // SAFETY: the clipboard is opened, written and closed here.
        unsafe {
            OpenClipboard(owner).ok()?;
            let done = EmptyClipboard().is_ok() && put(CF_UNICODETEXT, &wide);
            if done {
                // The three marks password managers set; each is one zero.
                let none = 0u32.to_le_bytes();
                put(
                    RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing")),
                    &none,
                );
                put(
                    RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory")),
                    &none,
                );
                put(
                    RegisterClipboardFormatW(w!("CanUploadToCloudClipboard")),
                    &none,
                );
            }
            // Read while the clipboard is still held: once it is closed,
            // another program's copy could take this number.
            let sequence = done.then(|| GetClipboardSequenceNumber());
            let _ = CloseClipboard();
            sequence
        }
    }

    /// Empties the clipboard if it still holds what copy `sequence` put.
    pub(super) fn clear_if_unchanged(sequence: u32) {
        // SAFETY: as above.
        unsafe {
            if GetClipboardSequenceNumber() != sequence || OpenClipboard(None).is_err() {
                return;
            }
            let _ = EmptyClipboard();
            let _ = CloseClipboard();
        }
    }
}

/// Copies `text` out of the clipboard history, to be taken off again later.
/// False when it could not be copied this way: the caller may then copy it
/// the ordinary way.
#[tauri::command]
#[specta::specta]
pub(crate) fn clipboard_copy_private(caller: WebviewWindow, text: String) -> bool {
    if !authorize(&caller, CallerPolicy::Main, "clipboard_copy_private")
        || text.is_empty()
        || text.len() > MAX_BYTES
    {
        return false;
    }
    let text = zeroize::Zeroizing::new(text);
    #[cfg(target_os = "windows")]
    {
        let owner = caller.hwnd().ok();
        let Some(sequence) = native::copy(owner, &text) else {
            return false;
        };
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(STAYS).await;
            native::clear_if_unchanged(sequence);
        });
        true
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (caller, text);
        false
    }
}
