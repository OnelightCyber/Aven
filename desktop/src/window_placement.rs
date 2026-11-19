//! Where the main window was when Aven closed: its frame and whether it
//! filled the screen, so the next launch opens the same way. Numbers only, in
//! one small file beside the app's data.
//!
//! Windows keeps a window's unmaximized frame itself, whatever state the
//! window is in when asked, so the frame is read once at close instead of
//! followed through moves: mid-maximize, a move arrives before the window
//! says it is maximized, and a followed frame took the maximized one for the
//! frame to come back to.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewWindow};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, SetWindowPlacement, SW_HIDE, SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED,
    WINDOWPLACEMENT, WPF_RESTORETOMAXIMIZED,
};

const FILE: &str = "window-placement.json";
// The file is five numbers; anything longer is not one of ours.
const MAX_FILE_BYTES: u64 = 512;
const MIN_SIDE: i32 = 200;
const MAX_SIDE: i32 = 16_384;
const MAX_OFFSET: u32 = 100_000;
// How much of the window's top edge a screen must show for a saved place to
// be taken: enough to see the window and to drag it by.
const REACH_WIDTH: i32 = 160;
const REACH_HEIGHT: i32 = 60;
// A window snapped to a screen's top or side sits a resize border outside it.
const BORDER: i32 = 16;

/// The window's unmaximized frame, as Windows places it, and whether the
/// window was maximized on top of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Placement {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    maximized: bool,
}

/// A screen's work area: left, top, width, height.
type Screen = (i32, i32, u32, u32);

// The saved window was maximized: it is maximized again as it is shown, not
// before, because maximizing a hidden window shows it.
static MAXIMIZE_ON_SHOW: AtomicBool = AtomicBool::new(false);

fn sane(placement: Placement) -> bool {
    let width = placement.right.checked_sub(placement.left);
    let height = placement.bottom.checked_sub(placement.top);
    width.is_some_and(|width| (MIN_SIDE..=MAX_SIDE).contains(&width))
        && height.is_some_and(|height| (MIN_SIDE..=MAX_SIDE).contains(&height))
        && placement.left.unsigned_abs() <= MAX_OFFSET
        && placement.top.unsigned_abs() <= MAX_OFFSET
}

fn parse(text: &str) -> Option<Placement> {
    let placement: Placement = serde_json::from_str(text).ok()?;
    sane(placement).then_some(placement)
}

/// Whether some screen shows enough of the window's top edge to reach it. A
/// screen unplugged since, or a changed layout, must not leave the window
/// where nobody can get at it.
fn reachable(placement: Placement, screens: &[Screen]) -> bool {
    screens.iter().any(|&(left, top, wide, high)| {
        let right = left.saturating_add(i32::try_from(wide).unwrap_or(i32::MAX));
        let bottom = top.saturating_add(i32::try_from(high).unwrap_or(i32::MAX));
        let shown = placement.right.min(right) - placement.left.max(left);
        shown >= REACH_WIDTH
            && placement.top >= top - BORDER
            && placement.top <= bottom - REACH_HEIGHT
    })
}

fn file(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|folder| folder.join(FILE))
}

fn read(app: &tauri::AppHandle) -> Option<Placement> {
    let path = file(app)?;
    if std::fs::metadata(&path).ok()?.len() > MAX_FILE_BYTES {
        return None;
    }
    parse(&std::fs::read_to_string(path).ok()?)
}

fn handle(window: &WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|hwnd| HWND(hwnd.0))
}

/// Gives the hidden main window the frame it had at the last close. Without
/// a saved frame, or with one no screen shows any more, the window keeps the
/// default the platform gave it.
pub(crate) fn restore(window: &WebviewWindow) {
    let Some(placement) = read(window.app_handle()) else {
        return;
    };
    let screens: Vec<Screen> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|monitor| {
            let area = monitor.work_area();
            (
                area.position.x,
                area.position.y,
                area.size.width,
                area.size.height,
            )
        })
        .collect();
    let Some(hwnd) = handle(window) else {
        return;
    };
    if !reachable(placement, &screens) {
        return;
    }
    let frame = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        // The window stays hidden: the startup gate shows it when it is ready.
        showCmd: SW_HIDE.0 as u32,
        ptMinPosition: POINT { x: -1, y: -1 },
        ptMaxPosition: POINT { x: -1, y: -1 },
        rcNormalPosition: RECT {
            left: placement.left,
            top: placement.top,
            right: placement.right,
            bottom: placement.bottom,
        },
        ..Default::default()
    };
    // SAFETY: a live window of this thread and an initialized placement.
    if unsafe { SetWindowPlacement(hwnd, &frame) }.is_ok() {
        MAXIMIZE_ON_SHOW.store(placement.maximized, Ordering::Release);
    }
}

/// Maximizes the window as it is first shown, when it was maximized at the
/// last close.
pub(crate) fn maximize_if_saved(window: &WebviewWindow) {
    if MAXIMIZE_ON_SHOW.swap(false, Ordering::AcqRel) {
        let _ = window.maximize();
    }
}

/// Writes the window's frame, when it is closing. A fullscreen window has no
/// frame of its own to keep, so the last saved one stays.
pub(crate) fn save(window: &WebviewWindow) {
    if window.is_fullscreen().unwrap_or(false) {
        return;
    }
    let Some(hwnd) = handle(window) else {
        return;
    };
    let mut frame = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    // SAFETY: a live window of this thread and a placement of the right size.
    if unsafe { GetWindowPlacement(hwnd, &mut frame) }.is_err() {
        return;
    }
    let placement = Placement {
        left: frame.rcNormalPosition.left,
        top: frame.rcNormalPosition.top,
        right: frame.rcNormalPosition.right,
        bottom: frame.rcNormalPosition.bottom,
        // Minimized at close, it comes back the way it would have been
        // restored.
        maximized: frame.showCmd == SW_SHOWMAXIMIZED.0 as u32
            || (frame.showCmd == SW_SHOWMINIMIZED.0 as u32
                && frame.flags.contains(WPF_RESTORETOMAXIMIZED)),
    };
    if !sane(placement) {
        return;
    }
    let (Some(path), Ok(text)) = (file(window.app_handle()), serde_json::to_string(&placement))
    else {
        return;
    };
    if let Some(folder) = path.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    // Written beside and renamed over, so a crash never leaves half a file.
    let temporary = path.with_extension("json.tmp");
    if std::fs::write(&temporary, text).is_ok() && std::fs::rename(&temporary, &path).is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESK: Screen = (0, 0, 1920, 1032);
    const SIDE: Screen = (-1920, 2, 1920, 1032);

    fn at(left: i32, top: i32) -> Placement {
        Placement {
            left,
            top,
            right: left + 1200,
            bottom: top + 800,
            maximized: false,
        }
    }

    #[test]
    fn a_saved_frame_round_trips_and_malformed_ones_are_refused() {
        let frame = Placement {
            maximized: true,
            ..at(-1880, 32)
        };
        assert_eq!(parse(&serde_json::to_string(&frame).unwrap()), Some(frame));
        for text in [
            "",
            "{}",
            "[1,2,3,4,false]",
            // No width, upside down, far too large, far away, wrong types.
            r#"{"left":0,"top":0,"right":0,"bottom":800,"maximized":false}"#,
            r#"{"left":500,"top":0,"right":100,"bottom":800,"maximized":false}"#,
            r#"{"left":0,"top":0,"right":1200,"bottom":99999,"maximized":false}"#,
            r#"{"left":9999999,"top":0,"right":10001199,"bottom":800,"maximized":false}"#,
            r#"{"left":-2147483648,"top":0,"right":2147483647,"bottom":800,"maximized":false}"#,
            r#"{"left":"0","top":0,"right":1200,"bottom":800,"maximized":false}"#,
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
    }

    #[test]
    fn a_place_is_taken_only_while_a_screen_still_shows_its_top_edge() {
        let both = [DESK, SIDE];
        assert!(reachable(at(100, 100), &both));
        assert!(reachable(at(-1880, 32), &both));
        // Snapped to a corner, a resize border outside the screen.
        assert!(reachable(at(-8, 0), &[DESK]));
        // The second screen is gone: its place is not taken.
        assert!(!reachable(at(-1880, 32), &[DESK]));
        // A sliver at the right edge, or the top band below the screen.
        assert!(!reachable(at(1900, 100), &[DESK]));
        assert!(!reachable(at(100, 1010), &[DESK]));
        // Above the screen, where the top band cannot be reached.
        assert!(!reachable(at(100, -200), &[DESK]));
        assert!(!reachable(at(100, 100), &[]));
    }
}
