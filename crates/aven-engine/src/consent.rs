//! Consent banners refused for the person. The shell sets the choice before
//! any view exists and again when it changes; a view takes whatever is
//! current as it is made.
//!
//! The script runs in a world of its own, registered through the view's
//! DevTools channel, so the page can neither read nor replace it. It holds
//! nothing worth taking: it only clicks a refusal the site itself offers.
//! Frames of another site, which Chromium runs apart, are not reached.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

static WANTED: AtomicBool = AtomicBool::new(false);
/// Sites whose banners are left for the person to answer.
static EXCEPT: RwLock<Vec<String>> = RwLock::new(Vec::new());

/// Returns whether the list changed.
pub(crate) fn set_exceptions(hosts: Vec<String>) -> bool {
    let mut except = EXCEPT
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if *except == hosts {
        return false;
    }
    *except = hosts;
    true
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
const SOURCE: &str = include_str!("platform/consent_refuse.js");
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
const WORLD: &str = "aven-consent";

/// Returns whether the choice changed.
pub(crate) fn set(on: bool) -> bool {
    WANTED.swap(on, Ordering::Relaxed) != on
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn wanted() -> bool {
    WANTED.load(Ordering::Relaxed)
}

/// What `Page.addScriptToEvaluateOnNewDocument` is sent: the script, its own
/// world, and the page already showing included.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn registration() -> String {
    // The list goes in front of the program as data, in the script's own
    // world: host names only, written as JSON.
    let except = EXCEPT
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let list = serde_json::to_string(&except).unwrap_or_else(|_| "[]".to_owned());
    serde_json::json!({
        "source": format!("globalThis.__avenConsentExcept = {list};\n{SOURCE}"),
        "worldName": WORLD,
        "includeCommandLineAPI": false,
        "runImmediately": true,
    })
    .to_string()
}

/// Registers the script on a view, or takes every registration away.
/// Leaving the Page domain drops the scripts it held for new documents.
#[cfg(target_os = "windows")]
pub(crate) fn apply(view: &wry::WebView, on: bool) {
    if on {
        crate::devtools::send(view, "Page.enable", "{}");
        crate::devtools::send(
            view,
            "Page.addScriptToEvaluateOnNewDocument",
            &registration(),
        );
    } else {
        crate::devtools::send(view, "Page.disable", "{}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_gets_a_world_of_its_own_and_the_page_already_showing() {
        let sent: serde_json::Value = serde_json::from_str(&registration()).expect("json");
        assert_eq!(sent["worldName"], WORLD);
        assert_eq!(sent["runImmediately"], true);
        assert_eq!(sent["includeCommandLineAPI"], false);
        assert!(sent["source"]
            .as_str()
            .is_some_and(|source| source.ends_with(SOURCE)));
    }

    #[test]
    fn the_sites_left_alone_go_in_as_data_in_front_of_the_program() {
        assert!(set_exceptions(vec!["example.com".to_owned()]));
        assert!(!set_exceptions(vec!["example.com".to_owned()]));
        let sent: serde_json::Value = serde_json::from_str(&registration()).expect("json");
        let source = sent["source"].as_str().expect("a script");
        assert!(source.starts_with("globalThis.__avenConsentExcept = [\"example.com\"];\n"));
        assert!(set_exceptions(Vec::new()));
        assert!(SOURCE.contains("__avenConsentExcept"));
    }

    #[test]
    fn the_script_stays_small_and_talks_to_nobody() {
        assert!(SOURCE.len() < 16 * 1024);
        for reach in [
            "fetch(",
            "XMLHttpRequest",
            "WebSocket",
            "sendBeacon",
            "postMessage",
            "eval(",
            "Function(",
            "localStorage",
            "document.cookie",
            "import(",
        ] {
            assert!(!SOURCE.contains(reach), "the script must not use {reach}");
        }
        // It refuses; it never accepts for the person.
        let lower = SOURCE.to_ascii_lowercase();
        for accepting in ["accept all", "accept-all", "tout accepter", "allow all"] {
            assert!(
                !lower.contains(accepting),
                "the script must not click {accepting}"
            );
        }
    }

    #[test]
    fn a_choice_said_twice_changes_once() {
        set(false);
        assert!(set(true));
        assert!(!set(true));
        assert!(wanted());
        assert!(set(false));
    }
}
