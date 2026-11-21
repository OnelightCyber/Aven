//! The community's setups: a list anyone can read, kept as files in Aven's
//! public repository, with no server of Aven's own.
//!
//! The list and every setup in it are untrusted text from the network. The
//! list is bounded and cleaned here; a setup taken from it goes through
//! [`crate::setups::from_file`] like any file, so it can change appearance
//! and layout and nothing else, and it is saved, never applied by itself.

use serde::Deserialize;

/// Where the list and its setups are read from. Nothing else is fetched.
pub const BASE_URL: &str =
    "https://raw.githubusercontent.com/OnelightCyber/Aven/main/community/setups/";
pub const INDEX_FILE: &str = "index.json";
pub const MAX_INDEX_BYTES: usize = 256 * 1024;
pub const MAX_ENTRIES: usize = 200;
const MAX_ID_CHARS: usize = 40;
const MAX_AUTHOR_CHARS: usize = 40;
const MAX_ABOUT_CHARS: usize = 140;
const FORMAT: u64 = 1;

/// One setup the community shares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Names its file; lowercase letters, digits and dashes only.
    pub id: String,
    pub name: String,
    /// Who made it, as they wrote it; may be empty.
    pub author: String,
    /// One line about it; may be empty.
    pub about: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexRefusal {
    TooLarge,
    /// Not JSON, or JSON that is not a list of setups.
    NotAList,
    /// Written for a later Aven.
    Newer,
}

#[derive(Deserialize)]
struct RawEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    about: String,
}

/// An id that can only ever name one file beside the list: it starts with a
/// letter or digit and holds nothing a path could be built from.
pub fn id_allowed(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_CHARS
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !id.starts_with('-')
}

/// The address of one setup's file, or nothing for an id that is not one.
pub fn setup_url(id: &str) -> Option<String> {
    id_allowed(id).then(|| format!("{BASE_URL}{id}.aven-setup.json"))
}

pub fn index_url() -> String {
    format!("{BASE_URL}{INDEX_FILE}")
}

fn clean(text: &str, max: usize) -> String {
    let printable: String = text.chars().filter(|c| !c.is_control()).collect();
    let cut: String = printable.trim().chars().take(max).collect();
    cut.trim().to_owned()
}

/// Reads the list. Entries with an id or a name that cannot be used are left
/// out, as are repeats and anything past the limit; text is cleaned for
/// display and stays text.
pub fn parse_index(bytes: &[u8]) -> Result<Vec<Entry>, IndexRefusal> {
    if bytes.len() > MAX_INDEX_BYTES {
        return Err(IndexRefusal::TooLarge);
    }
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let json: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| IndexRefusal::NotAList)?;
    let file = json.as_object().ok_or(IndexRefusal::NotAList)?;
    let format = file
        .get("aven_setups")
        .and_then(serde_json::Value::as_u64)
        .filter(|format| *format >= 1)
        .ok_or(IndexRefusal::NotAList)?;
    if format > FORMAT {
        return Err(IndexRefusal::Newer);
    }
    let listed = file
        .get("setups")
        .and_then(serde_json::Value::as_array)
        .ok_or(IndexRefusal::NotAList)?;
    let mut entries: Vec<Entry> = Vec::new();
    for raw in listed {
        if entries.len() == MAX_ENTRIES {
            break;
        }
        let Ok(raw) = serde_json::from_value::<RawEntry>(raw.clone()) else {
            continue;
        };
        let name = clean(&raw.name, crate::setups::MAX_NAME_CHARS);
        if !id_allowed(&raw.id) || name.is_empty() || entries.iter().any(|e| e.id == raw.id) {
            continue;
        }
        entries.push(Entry {
            id: raw.id,
            name,
            author: clean(&raw.author, MAX_AUTHOR_CHARS),
            about: clean(&raw.about, MAX_ABOUT_CHARS),
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_names_one_file_beside_the_list_and_nothing_else() {
        assert_eq!(
            setup_url("midnight-2").as_deref(),
            Some("https://raw.githubusercontent.com/OnelightCyber/Aven/main/community/setups/midnight-2.aven-setup.json")
        );
        for refused in [
            "",
            "-night",
            "Night",
            "a/b",
            "..",
            "a.b",
            "a b",
            "a%2fb",
            "a?x=1",
            "a#b",
            "é",
            &"a".repeat(MAX_ID_CHARS + 1),
        ] {
            assert_eq!(setup_url(refused), None, "{refused}");
        }
        assert!(index_url().starts_with("https://raw.githubusercontent.com/"));
        assert!(index_url().ends_with("/community/setups/index.json"));
    }

    /// What Aven's own repository publishes must read whole: every listed
    /// setup has its file, and nothing in a file is left out on reading.
    #[test]
    fn the_setups_this_repository_publishes_all_read_whole() {
        let folder =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../community/setups");
        let index = std::fs::read(folder.join(INDEX_FILE)).unwrap();
        let listed: serde_json::Value = serde_json::from_slice(&index).unwrap();
        let entries = parse_index(&index).unwrap();
        assert!(!entries.is_empty());
        assert_eq!(entries.len(), listed["setups"].as_array().unwrap().len());
        for entry in &entries {
            let file = std::fs::read(folder.join(format!("{}.aven-setup.json", entry.id)))
                .unwrap_or_else(|_| panic!("{} has no file", entry.id));
            let read = crate::setups::from_file(&file).unwrap();
            assert_eq!(read.dropped, 0, "{}", entry.id);
            assert_eq!(read.name, entry.name, "{}", entry.id);
        }
        let files = std::fs::read_dir(&folder)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|file| {
                file.file_name()
                    .to_string_lossy()
                    .ends_with(".aven-setup.json")
            })
            .count();
        assert_eq!(files, entries.len(), "a setup file is not in the list");
    }

    #[test]
    fn the_list_keeps_usable_entries_cleaned_and_drops_the_rest() {
        let list = br#"{
            "aven_setups": 1,
            "setups": [
                {"id": "midnight", "name": "  Midnight\u0007 ", "author": "Ada", "about": "Dark and compact."},
                {"id": "midnight", "name": "Twice"},
                {"id": "../etc", "name": "Path"},
                {"id": "unnamed", "name": "   "},
                {"id": "paper", "name": "Paper", "extra": true},
                "not an entry",
                {"id": 7, "name": "Number"}
            ]
        }"#;
        let entries = parse_index(list).unwrap();
        assert_eq!(
            entries,
            vec![
                Entry {
                    id: "midnight".into(),
                    name: "Midnight".into(),
                    author: "Ada".into(),
                    about: "Dark and compact.".into(),
                },
                Entry {
                    id: "paper".into(),
                    name: "Paper".into(),
                    author: String::new(),
                    about: String::new(),
                },
            ]
        );
        let long = format!(
            r#"{{"aven_setups":1,"setups":[{{"id":"long","name":"{}","author":"{}","about":"{}"}}]}}"#,
            "n".repeat(200),
            "a".repeat(200),
            "b".repeat(400)
        );
        let entry = &parse_index(long.as_bytes()).unwrap()[0];
        assert_eq!(entry.name.chars().count(), crate::setups::MAX_NAME_CHARS);
        assert_eq!(entry.author.chars().count(), MAX_AUTHOR_CHARS);
        assert_eq!(entry.about.chars().count(), MAX_ABOUT_CHARS);
    }

    #[test]
    fn the_list_stops_at_its_limit_and_refuses_what_is_not_a_list() {
        let many: Vec<String> = (0..MAX_ENTRIES + 20)
            .map(|n| format!(r#"{{"id":"s{n}","name":"Setup {n}"}}"#))
            .collect();
        let list = format!(r#"{{"aven_setups":1,"setups":[{}]}}"#, many.join(","));
        assert_eq!(parse_index(list.as_bytes()).unwrap().len(), MAX_ENTRIES);
        assert_eq!(
            parse_index(br#"{"aven_setups":1,"setups":[]}"#),
            Ok(Vec::new())
        );
        assert_eq!(
            parse_index(&vec![b' '; MAX_INDEX_BYTES + 1]),
            Err(IndexRefusal::TooLarge)
        );
        for not_a_list in [
            &b"<html>404</html>"[..],
            b"[]",
            b"{}",
            br#"{"aven_setups":1}"#,
            br#"{"aven_setups":"1","setups":[]}"#,
            br#"{"aven_setups":1,"setups":{}}"#,
        ] {
            assert_eq!(parse_index(not_a_list), Err(IndexRefusal::NotAList));
        }
        assert_eq!(
            parse_index(br#"{"aven_setups":2,"setups":[]}"#),
            Err(IndexRefusal::Newer)
        );
    }
}
