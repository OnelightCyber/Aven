//! Setups: the choices that make Aven look and feel a certain way, saved
//! under a name to switch between, and carried from one Aven to another as a
//! file.
//!
//! A setup holds appearance and layout choices only. Nothing in it names the
//! person, a site or a file on the machine, and no key it may hold lowers a
//! protection, so a setup file made by someone else is safe to take in. Each
//! value passes the same admission as a setting changed by hand.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The setting the saved setups are kept under.
pub const SETTING: &str = "ui.setups";
pub const MAX_SETUPS: usize = 12;
pub const MAX_NAME_CHARS: usize = 40;
/// A setup file is under two kibibytes; this leaves room for a later format.
pub const MAX_FILE_BYTES: usize = 64 * 1024;
/// Twelve setups at their largest stay well under this.
const MAX_SAVED_BYTES: usize = 48 * 1024;
const ID_CHARS: usize = 16;
const FORMAT: u64 = 1;

/// The settings a setup carries: how Aven looks, how its window is laid out
/// and which of its own tools show. Sites, addresses, the person's name,
/// their picture, the language, privacy, network, search and AI are left out.
pub const KEYS: &[&str] = &[
    "appearance",
    "ui.theme",
    "ui.theme-custom",
    "ui.theme-contrast",
    "ui.accent",
    "ui.material",
    "ui.window-tint",
    "ui.font",
    "ui.font-size",
    "ui.code-font",
    "ui.density",
    "ui.contrast",
    "ui.reduce-motion",
    "ui.force-dark",
    "ui.tab-layout",
    "sidebar.mode",
    "sidebar.apps",
    "tabs.close-button",
    "tabs.new-position",
    "tabs.after-close",
    "ui.newtab-greeting",
    "ui.newtab-clock",
    "ui.newtab-clock-format",
    "ui.newtab-tasks",
    "ui.newtab-dim",
    "ui.newtab-wordmark",
    "ui.newtab-trackers",
    "ui.newtab-focus",
    "notes.enabled",
    "tasks.enabled",
    "time.enabled",
];

/// One saved setup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub id: String,
    pub name: String,
    pub settings: BTreeMap<String, String>,
}

/// Sixteen lowercase hexadecimal characters, chosen by whoever saves it.
pub fn id_allowed(id: &str) -> bool {
    id.len() == ID_CHARS
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// A name as it is shown: some text, no longer than fits a row, with no
/// space around it and nothing unprintable in it.
pub fn name_allowed(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && name.trim() == name
        && !name.chars().any(char::is_control)
}

/// Whether a setup may hold this choice: a key it carries, with a value the
/// setting itself would take.
pub fn setting_allowed(key: &str, value: &str) -> bool {
    KEYS.contains(&key) && crate::preferences::value_allowed(key, value)
}

/// The saved setups as the setting stores them, or nothing when any part of
/// the list is not one a setup may hold. The empty string is no setups.
pub fn saved(value: &str) -> Option<Vec<Setup>> {
    if value.is_empty() {
        return Some(Vec::new());
    }
    if value.len() > MAX_SAVED_BYTES {
        return None;
    }
    let setups: Vec<Setup> = serde_json::from_str(value).ok()?;
    let sound = setups.len() <= MAX_SETUPS
        && setups.iter().enumerate().all(|(index, setup)| {
            id_allowed(&setup.id)
                && name_allowed(&setup.name)
                && !setup.settings.is_empty()
                && setup
                    .settings
                    .iter()
                    .all(|(key, value)| setting_allowed(key, value))
                && !setups[..index].iter().any(|earlier| earlier.id == setup.id)
        });
    sound.then_some(setups)
}

/// What the `ui.setups` setting accepts.
pub fn saved_allowed(value: &str) -> bool {
    saved(value).is_some()
}

#[derive(Serialize)]
struct FileOut<'a> {
    aven_setup: u64,
    name: &'a str,
    settings: &'a BTreeMap<String, String>,
}

/// A setup as a file: readable, and with nothing in it but its name and its
/// choices. The id stays behind; it means something only in one list.
pub fn to_file(setup: &Setup) -> String {
    let mut text = serde_json::to_string_pretty(&FileOut {
        aven_setup: FORMAT,
        name: &setup.name,
        settings: &setup.settings,
    })
    .unwrap_or_default();
    text.push('\n');
    text
}

/// What a setup file held that a setup may hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Imported {
    /// Cleaned for display; empty when the file named nothing usable.
    pub name: String,
    pub settings: BTreeMap<String, String>,
    /// Entries left out: keys a setup does not carry, or values the setting
    /// would not take.
    pub dropped: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileRefusal {
    TooLarge,
    /// Not JSON, or JSON that is not a setup file.
    NotASetup,
    /// Written by a later Aven in a format this one does not read.
    Newer,
    /// A setup file with nothing in it this Aven can apply.
    Empty,
}

fn clean_name(name: &str) -> String {
    let printable: String = name.chars().filter(|c| !c.is_control()).collect();
    let cut: String = printable.trim().chars().take(MAX_NAME_CHARS).collect();
    cut.trim().to_owned()
}

/// Reads a setup file from anywhere. The file is untrusted: it is bounded,
/// only the keys a setup carries are kept, and each value must be one the
/// setting takes. What is left out is counted, never applied.
pub fn from_file(bytes: &[u8]) -> Result<Imported, FileRefusal> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(FileRefusal::TooLarge);
    }
    // Editors on Windows may put a byte order mark first.
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let json: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| FileRefusal::NotASetup)?;
    let file = json.as_object().ok_or(FileRefusal::NotASetup)?;
    let format = file
        .get("aven_setup")
        .and_then(serde_json::Value::as_u64)
        .filter(|format| *format >= 1)
        .ok_or(FileRefusal::NotASetup)?;
    if format > FORMAT {
        return Err(FileRefusal::Newer);
    }
    let entries = file
        .get("settings")
        .and_then(serde_json::Value::as_object)
        .ok_or(FileRefusal::NotASetup)?;
    let mut settings = BTreeMap::new();
    let mut dropped = 0;
    for (key, value) in entries {
        match value.as_str() {
            Some(value) if setting_allowed(key, value) => {
                settings.insert(key.clone(), value.to_owned());
            }
            _ => dropped += 1,
        }
    }
    if settings.is_empty() {
        return Err(FileRefusal::Empty);
    }
    let name = file
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(clean_name)
        .unwrap_or_default();
    Ok(Imported {
        name,
        settings,
        dropped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(id: &str, name: &str, settings: &[(&str, &str)]) -> Setup {
        Setup {
            id: id.to_owned(),
            name: name.to_owned(),
            settings: settings
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }

    fn stored(setups: &[Setup]) -> String {
        serde_json::to_string(setups).unwrap()
    }

    #[test]
    fn a_setup_carries_settings_only_and_none_that_guards_or_names_anything() {
        for key in KEYS {
            assert!(crate::preferences::KEYS.contains(key), "{key}");
            for guarded in [
                "privacy.",
                "network.",
                "ai.",
                "work.",
                "search.",
                "updates.",
                "performance.",
                "focus.",
                "notice.",
                "zoom.",
                "tabs.startup",
                "tabs.home",
            ] {
                assert!(!key.starts_with(guarded), "{key}");
            }
        }
        for personal in [
            "ui.newtab-name",
            "ui.newtab-image",
            "ui.newtab-background",
            "ui.language",
            "ui.force-dark-except",
            "onboarding",
            SETTING,
        ] {
            assert!(!KEYS.contains(&personal), "{personal}");
        }
        assert!(crate::preferences::KEYS.contains(&SETTING));
    }

    #[test]
    fn saved_setups_are_admitted_whole_or_not_at_all() {
        let night = setup(
            "00000000000000aa",
            "Night",
            &[("appearance", "dark"), ("ui.font", "mono")],
        );
        let day = setup("00000000000000ab", "Day", &[("appearance", "light")]);
        assert_eq!(saved(""), Some(Vec::new()));
        assert_eq!(saved("[]"), Some(Vec::new()));
        assert_eq!(
            saved(&stored(&[night.clone(), day.clone()])),
            Some(vec![night.clone(), day.clone()])
        );
        assert!(crate::preferences::value_allowed(
            SETTING,
            &stored(std::slice::from_ref(&night))
        ));

        // The same id twice, or a list past the limit.
        assert!(!saved_allowed(&stored(&[night.clone(), night.clone()])));
        let many: Vec<Setup> = (0..=MAX_SETUPS)
            .map(|index| setup(&format!("{index:016x}"), "Many", &[("appearance", "dark")]))
            .collect();
        assert!(saved_allowed(&stored(&many[..MAX_SETUPS])));
        assert!(!saved_allowed(&stored(&many)));

        // An id, a name, a key or a value a setup may not hold.
        for refused in [
            setup("short", "Night", &[("appearance", "dark")]),
            setup("00000000000000AA", "Night", &[("appearance", "dark")]),
            setup("00000000000000aa", "", &[("appearance", "dark")]),
            setup("00000000000000aa", " Night", &[("appearance", "dark")]),
            setup("00000000000000aa", "Ni\nght", &[("appearance", "dark")]),
            setup(
                "00000000000000aa",
                &"n".repeat(MAX_NAME_CHARS + 1),
                &[("appearance", "dark")],
            ),
            setup("00000000000000aa", "Night", &[]),
            setup("00000000000000aa", "Night", &[("appearance", "purple")]),
            setup(
                "00000000000000aa",
                "Night",
                &[("privacy.smartscreen", "false")],
            ),
            setup("00000000000000aa", "Night", &[("ui.newtab-name", "true")]),
        ] {
            assert!(
                !saved_allowed(&stored(std::slice::from_ref(&refused))),
                "{refused:?}"
            );
        }
        assert!(!saved_allowed("{}"));
        assert!(!saved_allowed("not json"));
        assert!(!saved_allowed(
            r#"[{"id":"00000000000000aa","name":"Night","settings":{"appearance":"dark"},"extra":1}]"#
        ));
        assert!(!saved_allowed(&" ".repeat(MAX_SAVED_BYTES + 1)));
    }

    #[test]
    fn a_setup_leaves_as_a_file_and_comes_back_the_same_without_its_id() {
        let night = setup(
            "00000000000000aa",
            "Night",
            &[("appearance", "dark"), ("ui.font", "mono")],
        );
        let file = to_file(&night);
        assert!(!file.contains("00000000000000aa"));
        assert!(file.ends_with("}\n"));
        let back = from_file(file.as_bytes()).unwrap();
        assert_eq!(back.name, "Night");
        assert_eq!(back.settings, night.settings);
        assert_eq!(back.dropped, 0);
        // The same file saved by an editor that marks its byte order.
        let marked = [b"\xEF\xBB\xBF".as_slice(), file.as_bytes()].concat();
        assert_eq!(from_file(&marked).unwrap(), back);
    }

    #[test]
    fn a_file_from_elsewhere_keeps_only_what_a_setup_may_hold() {
        let file = br#"{
            "aven_setup": 1,
            "name": "  Loud\u0007 one  ",
            "settings": {
                "appearance": "dark",
                "ui.font": "comic",
                "privacy.smartscreen": "false",
                "ai.mcp-server": "true",
                "network.proxy-address": "evil.test:8080",
                "ui.newtab-clock": true,
                "not.a.setting": "x"
            },
            "note": "ignored"
        }"#;
        let read = from_file(file).unwrap();
        assert_eq!(read.name, "Loud one");
        assert_eq!(
            read.settings,
            BTreeMap::from([("appearance".to_owned(), "dark".to_owned())])
        );
        assert_eq!(read.dropped, 6);

        let long = format!(
            r#"{{"aven_setup":1,"name":"{}","settings":{{"appearance":"light"}}}}"#,
            "n".repeat(MAX_NAME_CHARS * 3)
        );
        assert_eq!(
            from_file(long.as_bytes()).unwrap().name.chars().count(),
            MAX_NAME_CHARS
        );
        let unnamed = br#"{"aven_setup":1,"settings":{"appearance":"light"}}"#;
        assert_eq!(from_file(unnamed).unwrap().name, "");
    }

    #[test]
    fn a_file_that_is_not_a_setup_is_refused_and_says_why() {
        assert_eq!(
            from_file(&vec![b' '; MAX_FILE_BYTES + 1]),
            Err(FileRefusal::TooLarge)
        );
        for not_a_setup in [
            &b"not json"[..],
            b"[]",
            b"{}",
            br#"{"aven_setup":"1","settings":{"appearance":"dark"}}"#,
            br#"{"aven_setup":0,"settings":{"appearance":"dark"}}"#,
            br#"{"aven_setup":1}"#,
            br#"{"aven_setup":1,"settings":["appearance"]}"#,
            b"\xff\xfe",
        ] {
            assert_eq!(from_file(not_a_setup), Err(FileRefusal::NotASetup));
        }
        assert_eq!(
            from_file(br#"{"aven_setup":2,"settings":{"appearance":"dark"}}"#),
            Err(FileRefusal::Newer)
        );
        assert_eq!(
            from_file(br#"{"aven_setup":1,"settings":{}}"#),
            Err(FileRefusal::Empty)
        );
        assert_eq!(
            from_file(br#"{"aven_setup":1,"settings":{"privacy.smartscreen":"false"}}"#),
            Err(FileRefusal::Empty)
        );
    }
}
