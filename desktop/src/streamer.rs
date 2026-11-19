//! Streamer mode on the native side: whether a recording or streaming
//! program is running, and whether Aven's own system notifications are held
//! back because the person is on air.
//!
//! Only the names of running programs are looked at, against a short fixed
//! list. Nothing about them is kept or logged.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{Manager, WebviewWindow};

use crate::{authorize, CallerPolicy};

static CAPTURING: AtomicBool = AtomicBool::new(false);

/// Programs that record or broadcast the screen, by their file name.
const CAPTURE_PROGRAMS: [&str; 10] = [
    "obs64.exe",
    "obs32.exe",
    "obs.exe",
    "streamlabs obs.exe",
    "streamlabs desktop.exe",
    "xsplit.core.exe",
    "twitchstudio.exe",
    "vmix64.exe",
    "action.exe",
    "bdcam.exe",
];

/// Whether a program of this file name records or broadcasts the screen.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn is_capture_program(name: &str) -> bool {
    CAPTURE_PROGRAMS
        .iter()
        .any(|known| name.eq_ignore_ascii_case(known))
}

fn setting(key: &str) -> Option<String> {
    crate::APP_STORE
        .get()
        .and_then(|store| aven_core::ports::store::Store::app_setting(&**store, key))
}

/// What the watcher saw; the interface is told when it changes.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn set_capturing(app: &tauri::AppHandle, capturing: bool) {
    if CAPTURING.swap(capturing, Ordering::Relaxed) == capturing {
        return;
    }
    if let Some(window) = app.get_webview_window(crate::MAIN_LABEL) {
        let _ = window.eval(format!(
            "window.dispatchEvent(new CustomEvent('aven:capture',{{detail:{capturing}}}))"
        ));
    }
}

/// Whether streamer mode is in force: switched on, or a capture program
/// runs and the person did not say to ignore that.
pub(crate) fn in_force() -> bool {
    setting("privacy.streamer-mode").as_deref() == Some("true")
        || (CAPTURING.load(Ordering::Relaxed)
            && setting("privacy.streamer-auto").as_deref() != Some("false"))
}

/// Whether a capture program runs, for an interface that came up after the
/// last change was announced.
#[tauri::command]
#[specta::specta]
pub(crate) fn streamer_capturing(caller: WebviewWindow) -> bool {
    authorize(&caller, CallerPolicy::Main, "streamer_capturing")
        && CAPTURING.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_programs_are_known_by_file_name_whatever_the_case() {
        assert!(is_capture_program("obs64.exe"));
        assert!(is_capture_program("OBS64.EXE"));
        assert!(is_capture_program("Streamlabs OBS.exe"));
        assert!(!is_capture_program("obs64.exe.bak"));
        assert!(!is_capture_program("notobs64.exe"));
        assert!(!is_capture_program(""));
    }
}
