//! How loud one tab plays, as the person set it. The engine keeps each
//! tab's level so a view made again for the same tab, after it slept, plays
//! at the level it had.
//!
//! WebView2 can mute a page and nothing more, so the level is applied by a
//! small script in the page's own world: see `platform/tab_volume.js` for
//! what that reaches and what it cannot. A level of 100 installs nothing.

use std::collections::HashMap;
use std::sync::Mutex;

use aven_core::ids::ItemId;

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
const SOURCE: &str = include_str!("platform/tab_volume.js");

/// The level a tab plays at when nothing was chosen for it.
pub(crate) const FULL: u8 = 100;
/// Levels kept at once.
const MAX_KEPT: usize = 2_048;

static LEVELS: Mutex<Option<HashMap<ItemId, u8>>> = Mutex::new(None);

fn with_levels<T>(apply: impl FnOnce(&mut HashMap<ItemId, u8>) -> T) -> T {
    let mut levels = LEVELS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    apply(levels.get_or_insert_with(HashMap::new))
}

/// Keeps `percent` for `tab`; more than full is full.
pub(crate) fn set(tab: ItemId, percent: u8) -> u8 {
    let percent = percent.min(FULL);
    with_levels(|levels| {
        if percent == FULL {
            levels.remove(&tab);
        } else {
            // Closed tabs leave their level behind; the table starts over
            // long before that could weigh anything.
            if levels.len() >= MAX_KEPT {
                levels.clear();
            }
            levels.insert(tab, percent);
        }
    });
    percent
}

/// The level kept for `tab`.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn level(tab: ItemId) -> u8 {
    with_levels(|levels| levels.get(&tab).copied().unwrap_or(FULL))
}

/// The script that plays a page at `percent`: the level, then the program,
/// in a block of their own so that it can be run again in the same page when
/// the level changes.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn script(percent: u8) -> String {
    format!(
        "{{ const AVEN_TAB_VOLUME = {};\n{SOURCE}\n}}",
        percent.min(FULL)
    )
}

#[cfg(target_os = "windows")]
mod native {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use aven_core::ids::ItemId;
    use webview2_com::AddScriptToExecuteOnDocumentCreatedCompletedHandler;
    use windows::core::HSTRING;
    use wry::WebViewExtWindows;

    thread_local! {
        /// The script each view runs in its new documents, by the name
        /// WebView2 gave it. Views live on this thread only.
        static REGISTERED: RefCell<HashMap<ItemId, String>> = RefCell::new(HashMap::new());
    }

    /// Plays the view's page at `percent` from now on: the document showing
    /// and those that follow.
    pub(crate) fn apply(tab: ItemId, view: &wry::WebView, percent: u8) {
        let core = view.webview();
        if let Some(old) = REGISTERED.with(|registered| registered.borrow_mut().remove(&tab)) {
            // SAFETY: a live view's own interface, on its thread.
            let _ = unsafe { core.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(old)) };
        }
        let script = super::script(percent);
        if percent < super::FULL {
            let handler = AddScriptToExecuteOnDocumentCreatedCompletedHandler::create(Box::new(
                move |status, name| {
                    // A level set again meanwhile registered its own.
                    if status.is_ok() && super::level(tab) == percent {
                        REGISTERED.with(|registered| registered.borrow_mut().insert(tab, name));
                    }
                    Ok(())
                },
            ));
            // SAFETY: as above; the handler is kept by WebView2 until it ran.
            let _ = unsafe {
                core.AddScriptToExecuteOnDocumentCreated(&HSTRING::from(script.as_str()), &handler)
            };
        }
        let parameters = serde_json::json!({
            "expression": script,
            "includeCommandLineAPI": false,
        })
        .to_string();
        crate::devtools::send(view, "Runtime.evaluate", &parameters);
    }

    /// The view is gone: its registration went with it.
    pub(crate) fn forget(tab: ItemId) {
        REGISTERED.with(|registered| registered.borrow_mut().remove(&tab));
    }
}

#[cfg(target_os = "windows")]
pub(crate) use native::{apply, forget as forget_view};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_is_kept_per_tab_and_full_keeps_nothing() {
        let (quiet, other) = (ItemId::generate(), ItemId::generate());
        assert_eq!(level(quiet), FULL);
        assert_eq!(set(quiet, 40), 40);
        assert_eq!(level(quiet), 40);
        assert_eq!(level(other), FULL);
        assert_eq!(set(quiet, 250), FULL);
        assert_eq!(level(quiet), FULL);
        set(other, 0);
        assert_eq!(level(other), 0);
    }

    #[test]
    fn the_script_carries_the_level_and_talks_to_nobody() {
        let script = script(35);
        // A block: a second run in the same page declares nothing twice.
        assert!(script.starts_with("{ const AVEN_TAB_VOLUME = 35;\n"));
        assert!(script.ends_with("\n}"));
        assert!(script.len() < 16 * 1024);
        for reach in [
            "fetch(",
            "XMLHttpRequest",
            "WebSocket",
            "sendBeacon",
            "postMessage",
            "eval(",
        ] {
            assert!(!SOURCE.contains(reach), "{reach}");
        }
    }
}
