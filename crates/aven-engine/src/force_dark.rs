//! Pages without a dark theme of their own, shown dark. The shell sets the
//! person's choice before any view exists and again when it changes; a view
//! takes whatever is current as it is made.
//!
//! The darkening is Chromium's own automatic dark theme, asked for through
//! the view's DevTools channel: nothing is evaluated in the page, and a site
//! that declares a dark colour scheme is left as its authors drew it.

use std::sync::atomic::{AtomicBool, Ordering};

static WANTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn set(on: bool) {
    WANTED.store(on, Ordering::Relaxed);
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn wanted() -> bool {
    WANTED.load(Ordering::Relaxed)
}

/// What `Emulation.setAutoDarkModeOverride` is sent: the override on, or no
/// override at all, which hands the page back to its own colours.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn parameters(on: bool) -> &'static str {
    if on {
        r#"{"enabled":true}"#
    } else {
        "{}"
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn apply(view: &wry::WebView, on: bool) {
    crate::devtools::send(view, "Emulation.setAutoDarkModeOverride", parameters(on));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_it_off_clears_the_override_instead_of_forcing_light() {
        let on: serde_json::Value = serde_json::from_str(parameters(true)).expect("json");
        assert_eq!(on, serde_json::json!({ "enabled": true }));
        let off: serde_json::Value = serde_json::from_str(parameters(false)).expect("json");
        assert_eq!(off, serde_json::json!({}));
    }

    #[test]
    fn a_new_view_takes_the_choice_in_force() {
        set(true);
        assert!(wanted());
        set(false);
        assert!(!wanted());
    }
}
