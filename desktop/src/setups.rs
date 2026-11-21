//! A saved setup taken out as a file, and a setup file taken in.
//!
//! The interface keeps the saved setups in a setting and applies one a
//! validated setting at a time. This only moves files: it writes the setup
//! the store holds, and it reads a file from anywhere as untrusted data,
//! handing back just what `aven_core::setups` admits.

use super::*;
use aven_core::setups::{self, FileRefusal};
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SetupExportView {
    Saved,
    /// The person closed the save dialog.
    Cancelled,
    /// No saved setup has this id.
    Missing,
    Failed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SetupImportOutcome {
    Read,
    /// The person closed the file dialog.
    Cancelled,
    TooLarge,
    NotASetup,
    Newer,
    Empty,
    Failed,
}

#[derive(Clone, Debug, Serialize, specta::Type, PartialEq, Eq)]
pub(crate) struct SetupImportView {
    outcome: SetupImportOutcome,
    /// Empty unless the file was read, or when it named nothing usable.
    name: String,
    settings: BTreeMap<String, String>,
    /// Entries of the file a setup may not hold, left out.
    dropped: u32,
}

impl SetupImportView {
    fn without(outcome: SetupImportOutcome) -> Self {
        Self {
            outcome,
            name: String::new(),
            settings: BTreeMap::new(),
            dropped: 0,
        }
    }
}

/// One file dialog at a time.
static ADMISSION: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
const EXTENSION: &str = "json";
const FILTER: &str = "Aven setup";

/// A file name from a setup's name: its letters, digits, spaces, dashes and
/// underscores, so no name can reach outside the chosen folder.
fn file_name(name: &str) -> String {
    let kept: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let kept = kept.trim();
    let stem = if kept.is_empty() { "Aven setup" } else { kept };
    format!("{stem}.aven-setup.{EXTENSION}")
}

fn refusal(refusal: FileRefusal) -> SetupImportOutcome {
    match refusal {
        FileRefusal::TooLarge => SetupImportOutcome::TooLarge,
        FileRefusal::NotASetup => SetupImportOutcome::NotASetup,
        FileRefusal::Newer => SetupImportOutcome::Newer,
        FileRefusal::Empty => SetupImportOutcome::Empty,
    }
}

/// Reads at most one byte past the limit, so a huge file is refused without
/// being loaded.
fn read_bounded(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(setups::MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn read_file(path: &std::path::Path) -> SetupImportView {
    let Ok(bytes) = read_bounded(path) else {
        return SetupImportView::without(SetupImportOutcome::Failed);
    };
    match setups::from_file(&bytes) {
        Ok(read) => SetupImportView {
            outcome: SetupImportOutcome::Read,
            name: read.name,
            settings: read.settings,
            dropped: u32::try_from(read.dropped).unwrap_or(u32::MAX),
        },
        Err(refused) => SetupImportView::without(refusal(refused)),
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn setups_export(
    caller: WebviewWindow,
    app: tauri::AppHandle,
    id: String,
) -> SetupExportView {
    use tauri_plugin_dialog::DialogExt;
    if !authorize(&caller, CallerPolicy::Main, "setups_export")
        || shutdown_started(&app)
        || !setups::id_allowed(&id)
    {
        return SetupExportView::Failed;
    }
    let Ok(_permit) = ADMISSION.try_acquire() else {
        return SetupExportView::Failed;
    };
    // The file is written from what the store holds, not from what the
    // caller says a setup contains.
    let stored = tauri::async_runtime::spawn_blocking(|| {
        APP_STORE
            .get()
            .and_then(|store| store.app_setting(setups::SETTING))
    })
    .await
    .ok()
    .flatten()
    .unwrap_or_default();
    let Some(setup) =
        setups::saved(&stored).and_then(|saved| saved.into_iter().find(|setup| setup.id == id))
    else {
        return SetupExportView::Missing;
    };
    let name = file_name(&setup.name);
    let picked = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(name)
            .add_filter(FILTER, &[EXTENSION])
            .blocking_save_file()
    })
    .await
    .ok()
    .flatten();
    let Some(path) = picked.and_then(|picked| picked.into_path().ok()) else {
        return SetupExportView::Cancelled;
    };
    let text = setups::to_file(&setup);
    match tokio::task::spawn_blocking(move || data_export::write_export(&path, text.as_bytes()))
        .await
    {
        Ok(Ok(())) => SetupExportView::Saved,
        _ => SetupExportView::Failed,
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn setups_import(caller: WebviewWindow, app: tauri::AppHandle) -> SetupImportView {
    use tauri_plugin_dialog::DialogExt;
    if !authorize(&caller, CallerPolicy::Main, "setups_import") || shutdown_started(&app) {
        return SetupImportView::without(SetupImportOutcome::Failed);
    }
    let Ok(_permit) = ADMISSION.try_acquire() else {
        return SetupImportView::without(SetupImportOutcome::Failed);
    };
    let picked = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter(FILTER, &[EXTENSION])
            .blocking_pick_file()
    })
    .await
    .ok()
    .flatten();
    let Some(path) = picked.and_then(|picked| picked.into_path().ok()) else {
        return SetupImportView::without(SetupImportOutcome::Cancelled);
    };
    tokio::task::spawn_blocking(move || read_file(&path))
        .await
        .unwrap_or_else(|_| SetupImportView::without(SetupImportOutcome::Failed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setup_name_cannot_steer_where_its_file_goes() {
        assert_eq!(file_name("Night"), "Night.aven-setup.json");
        assert_eq!(file_name("Été 2026"), "Été 2026.aven-setup.json");
        assert_eq!(
            file_name(r"..\..\Startup\run"),
            "______Startup_run.aven-setup.json"
        );
        assert_eq!(file_name("a/b:c*?"), "a_b_c__.aven-setup.json");
        assert_eq!(file_name("   "), "Aven setup.aven-setup.json");
        assert_eq!(file_name(""), "Aven setup.aven-setup.json");
    }

    #[test]
    fn a_file_is_read_as_untrusted_and_only_what_a_setup_holds_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("night.aven-setup.json");
        std::fs::write(
            &path,
            br#"{"aven_setup":1,"name":"Night","settings":{"appearance":"dark","privacy.smartscreen":"false"}}"#,
        )
        .unwrap();
        let read = read_file(&path);
        assert_eq!(read.outcome, SetupImportOutcome::Read);
        assert_eq!(read.name, "Night");
        assert_eq!(
            read.settings,
            BTreeMap::from([("appearance".to_owned(), "dark".to_owned())])
        );
        assert_eq!(read.dropped, 1);
    }

    #[test]
    fn a_file_that_is_missing_huge_or_not_a_setup_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            read_file(&dir.path().join("absent.json")).outcome,
            SetupImportOutcome::Failed
        );
        let huge = dir.path().join("huge.json");
        std::fs::write(&huge, vec![b' '; setups::MAX_FILE_BYTES + 4096]).unwrap();
        let refused = read_file(&huge);
        assert_eq!(refused.outcome, SetupImportOutcome::TooLarge);
        assert!(refused.settings.is_empty());
        // Only one byte past the limit is ever read.
        assert_eq!(
            read_bounded(&huge).unwrap().len(),
            setups::MAX_FILE_BYTES + 1
        );
        for (text, outcome) in [
            (&b"hello"[..], SetupImportOutcome::NotASetup),
            (
                br#"{"aven_setup":9,"settings":{"appearance":"dark"}}"#,
                SetupImportOutcome::Newer,
            ),
            (
                br#"{"aven_setup":1,"settings":{"ai.mcp-server":"true"}}"#,
                SetupImportOutcome::Empty,
            ),
        ] {
            let path = dir.path().join("other.json");
            std::fs::write(&path, text).unwrap();
            assert_eq!(read_file(&path).outcome, outcome);
        }
    }
}
