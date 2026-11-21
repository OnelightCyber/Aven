//! The community's setups, read from Aven's public repository when the
//! person asks to see them, never before.
//!
//! Two addresses are ever fetched, both fixed in `aven_core::setup_gallery`:
//! the list, and one setup's file named by an id from that list. Both come
//! back as untrusted bytes: bounded here, then read by the core, which keeps
//! only what a list or a setup may hold. A setup taken is handed to the
//! interface to be saved; nothing is applied from here.

use super::*;
use aven_core::setup_gallery::{self as gallery, IndexRefusal};
use aven_core::setups;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, specta::Type, PartialEq, Eq)]
pub(crate) struct SetupGalleryEntryView {
    id: String,
    name: String,
    author: String,
    about: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SetupGalleryOutcome {
    Listed,
    /// The list could not be reached.
    Offline,
    /// What came back is not a list this Aven reads.
    Unreadable,
    /// The list is written for a later Aven.
    Newer,
}

#[derive(Clone, Debug, Serialize, specta::Type, PartialEq, Eq)]
pub(crate) struct SetupGalleryView {
    outcome: SetupGalleryOutcome,
    entries: Vec<SetupGalleryEntryView>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SetupGalleryTakeOutcome {
    Read,
    Offline,
    /// The file is not a setup this Aven can use.
    Unreadable,
    Newer,
}

#[derive(Clone, Debug, Serialize, specta::Type, PartialEq, Eq)]
pub(crate) struct SetupGalleryTakeView {
    outcome: SetupGalleryTakeOutcome,
    name: String,
    settings: BTreeMap<String, String>,
    dropped: u32,
}

static CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
/// One request to the repository at a time.
static ADMISSION: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

fn client() -> Option<&'static reqwest::Client> {
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .https_only(true)
                // The addresses are fixed; a redirect elsewhere is not followed.
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(12))
                .pool_max_idle_per_host(1)
                .build()
                .ok()
        })
        .as_ref()
}

/// The bytes at one of the two fixed addresses, or nothing when the answer
/// is not a plain success or is larger than `limit`.
async fn fetch(url: &str, limit: usize) -> Option<Vec<u8>> {
    let mut response = client()?.get(url).send().await.ok()?;
    if response.status() != reqwest::StatusCode::OK
        || response
            .content_length()
            .is_some_and(|length| length > limit as u64)
    {
        return None;
    }
    let mut bytes = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) if bytes.len() + chunk.len() <= limit => {
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => return Some(bytes),
            _ => return None,
        }
    }
}

fn listed(bytes: &[u8]) -> SetupGalleryView {
    match gallery::parse_index(bytes) {
        Ok(entries) => SetupGalleryView {
            outcome: SetupGalleryOutcome::Listed,
            entries: entries
                .into_iter()
                .map(|entry| SetupGalleryEntryView {
                    id: entry.id,
                    name: entry.name,
                    author: entry.author,
                    about: entry.about,
                })
                .collect(),
        },
        Err(refusal) => SetupGalleryView {
            outcome: match refusal {
                IndexRefusal::Newer => SetupGalleryOutcome::Newer,
                IndexRefusal::TooLarge | IndexRefusal::NotAList => SetupGalleryOutcome::Unreadable,
            },
            entries: Vec::new(),
        },
    }
}

fn taken(bytes: &[u8]) -> SetupGalleryTakeView {
    match setups::from_file(bytes) {
        Ok(read) => SetupGalleryTakeView {
            outcome: SetupGalleryTakeOutcome::Read,
            name: read.name,
            settings: read.settings,
            dropped: u32::try_from(read.dropped).unwrap_or(u32::MAX),
        },
        Err(refusal) => SetupGalleryTakeView {
            outcome: match refusal {
                setups::FileRefusal::Newer => SetupGalleryTakeOutcome::Newer,
                _ => SetupGalleryTakeOutcome::Unreadable,
            },
            name: String::new(),
            settings: BTreeMap::new(),
            dropped: 0,
        },
    }
}

fn nothing_taken(outcome: SetupGalleryTakeOutcome) -> SetupGalleryTakeView {
    SetupGalleryTakeView {
        outcome,
        name: String::new(),
        settings: BTreeMap::new(),
        dropped: 0,
    }
}

/// The community's list, fetched because the person opened it.
#[tauri::command]
#[specta::specta]
pub(crate) async fn setups_gallery_list(
    caller: WebviewWindow,
    app: tauri::AppHandle,
) -> SetupGalleryView {
    let offline = SetupGalleryView {
        outcome: SetupGalleryOutcome::Offline,
        entries: Vec::new(),
    };
    if !authorize(&caller, CallerPolicy::Main, "setups_gallery_list") || shutdown_started(&app) {
        return offline;
    }
    let Ok(_permit) = ADMISSION.try_acquire() else {
        return offline;
    };
    match fetch(&gallery::index_url(), gallery::MAX_INDEX_BYTES).await {
        Some(bytes) => listed(&bytes),
        None => offline,
    }
}

/// One setup from the list, read as a file from anywhere would be.
#[tauri::command]
#[specta::specta]
pub(crate) async fn setups_gallery_take(
    caller: WebviewWindow,
    app: tauri::AppHandle,
    id: String,
) -> SetupGalleryTakeView {
    if !authorize(&caller, CallerPolicy::Main, "setups_gallery_take") || shutdown_started(&app) {
        return nothing_taken(SetupGalleryTakeOutcome::Offline);
    }
    // An id that is not one never becomes part of an address.
    let Some(url) = gallery::setup_url(&id) else {
        return nothing_taken(SetupGalleryTakeOutcome::Unreadable);
    };
    let Ok(_permit) = ADMISSION.try_acquire() else {
        return nothing_taken(SetupGalleryTakeOutcome::Offline);
    };
    match fetch(&url, setups::MAX_FILE_BYTES).await {
        Some(bytes) => taken(&bytes),
        None => nothing_taken(SetupGalleryTakeOutcome::Offline),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_from_the_network_is_shown_cleaned_or_not_at_all() {
        let view = listed(
            br#"{"aven_setups":1,"setups":[
                {"id":"midnight","name":"Midnight","author":"Ada","about":"Dark."},
                {"id":"../x","name":"Path"}
            ]}"#,
        );
        assert_eq!(view.outcome, SetupGalleryOutcome::Listed);
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.entries[0].id, "midnight");
        for (bytes, outcome) in [
            (
                &b"<html>Not found</html>"[..],
                SetupGalleryOutcome::Unreadable,
            ),
            (
                br#"{"aven_setups":3,"setups":[]}"#,
                SetupGalleryOutcome::Newer,
            ),
        ] {
            let refused = listed(bytes);
            assert_eq!(refused.outcome, outcome);
            assert!(refused.entries.is_empty());
        }
    }

    #[test]
    fn a_setup_from_the_network_keeps_only_what_a_setup_may_hold() {
        let read = taken(
            br#"{"aven_setup":1,"name":"Loud","settings":{"appearance":"dark","privacy.smartscreen":"false","network.proxy":"http"}}"#,
        );
        assert_eq!(read.outcome, SetupGalleryTakeOutcome::Read);
        assert_eq!(
            read.settings,
            BTreeMap::from([("appearance".to_owned(), "dark".to_owned())])
        );
        assert_eq!(read.dropped, 2);
        let refused = taken(b"404: Not Found");
        assert_eq!(refused.outcome, SetupGalleryTakeOutcome::Unreadable);
        assert!(refused.settings.is_empty());
        assert_eq!(
            taken(br#"{"aven_setup":7,"settings":{"appearance":"dark"}}"#).outcome,
            SetupGalleryTakeOutcome::Newer
        );
    }

    #[test]
    fn only_the_repository_s_two_kinds_of_address_are_ever_built() {
        assert!(gallery::index_url().starts_with(gallery::BASE_URL));
        assert!(gallery::setup_url("midnight")
            .unwrap()
            .starts_with(gallery::BASE_URL));
        for id in ["../../other", "x/y", "https://evil.test/x", "a b", ""] {
            assert_eq!(gallery::setup_url(id), None);
        }
    }
}
