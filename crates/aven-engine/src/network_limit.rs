//! How fast each page may download and upload. The shell sets the limit in
//! force before any view exists and again when it changes; a view takes
//! whatever is current as it is made.
//!
//! The limit is Chromium's own network throttling, asked for through the
//! view's DevTools channel. It holds for the page and the frames of its own
//! site; a frame of another site, which Chromium runs apart, is not held.

use std::sync::atomic::{AtomicU32, Ordering};

static KILOBITS: AtomicU32 = AtomicU32::new(0);

pub(crate) fn set(kilobits_per_second: u32) {
    KILOBITS.store(kilobits_per_second, Ordering::Relaxed);
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn current() -> u32 {
    KILOBITS.load(Ordering::Relaxed)
}

/// What `Network.emulateNetworkConditions` is sent. Chromium counts bytes
/// per second and takes -1 for no limit.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn parameters(kilobits_per_second: u32) -> String {
    let rate = if kilobits_per_second == 0 {
        -1
    } else {
        i64::from(kilobits_per_second) * 125
    };
    format!(
        r#"{{"offline":false,"latency":0,"downloadThroughput":{rate},"uploadThroughput":{rate}}}"#
    )
}

/// Holds a view to the limit, or lifts it. Chromium keeps requests to a
/// limit only while the view's Network domain is on, so it is turned on for
/// the time a limit holds and off again with it: a page nobody limits pays
/// nothing.
#[cfg(target_os = "windows")]
pub(crate) fn apply(view: &wry::WebView, kilobits_per_second: u32) {
    if kilobits_per_second > 0 {
        crate::devtools::send(view, "Network.enable", "{}");
    }
    crate::devtools::send(
        view,
        "Network.emulateNetworkConditions",
        &parameters(kilobits_per_second),
    );
    if kilobits_per_second == 0 {
        crate::devtools::send(view, "Network.disable", "{}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_limit_is_sent_in_bytes_and_none_lifts_it() {
        let limited: serde_json::Value = serde_json::from_str(&parameters(8000)).expect("json");
        assert_eq!(limited["downloadThroughput"], 1_000_000);
        assert_eq!(limited["uploadThroughput"], 1_000_000);
        assert_eq!(limited["offline"], false);
        let lifted: serde_json::Value = serde_json::from_str(&parameters(0)).expect("json");
        assert_eq!(lifted["downloadThroughput"], -1);
        assert_eq!(lifted["uploadThroughput"], -1);
        // The largest value a setting could ever hold still fits.
        let most: serde_json::Value = serde_json::from_str(&parameters(u32::MAX)).expect("json");
        assert_eq!(most["downloadThroughput"], i64::from(u32::MAX) * 125);
    }
}
