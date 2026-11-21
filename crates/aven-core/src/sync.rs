//! Sync between one person's devices, with no account.
//!
//! One device makes a code; the others are given it. Everything a device
//! sends is encrypted with a key derived from that code, so the relay holds
//! what it cannot read. This module is the part with no network and no
//! cipher in it: the code as people read and type it, what is synced, and how
//! two devices that both changed something are brought back together.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The secret a sync code carries: 160 bits.
pub const CODE_BYTES: usize = 20;
/// The plain text of what is synced stays far under the relay's limit.
pub const MAX_PLAIN_BYTES: usize = 128 * 1024;
const FORMAT: u64 = 1;
/// Crockford's base 32: no I, L, O or U, so a code read aloud or copied by
/// hand has no letter that looks like a digit.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The settings devices share: everything a setup carries, the saved setups
/// themselves, the language and the search engine. Nothing that belongs to
/// one machine or guards it: no sizes, proxy, privacy or AI switches, and no
/// site lists.
pub fn keys() -> impl Iterator<Item = &'static str> {
    crate::setups::KEYS.iter().copied().chain([
        crate::setups::SETTING,
        "ui.language",
        "search.engine",
    ])
}

pub fn key_synced(key: &str) -> bool {
    keys().any(|synced| synced == key)
}

/// A code as shown: eight groups of four, `7K2M-…`.
pub fn format_code(secret: &[u8; CODE_BYTES]) -> String {
    let mut text = String::with_capacity(39);
    let (mut buffer, mut bits, mut written) = (0u32, 0u32, 0usize);
    for byte in secret {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            if written > 0 && written % 4 == 0 {
                text.push('-');
            }
            text.push(char::from(ALPHABET[((buffer >> bits) & 31) as usize]));
            written += 1;
        }
    }
    text
}

/// A code as typed: any case, with or without dashes and spaces, and with
/// the letters people mistake for digits read as those digits. Nothing for
/// text that is not a whole code.
pub fn parse_code(text: &str) -> Option<[u8; CODE_BYTES]> {
    let mut secret = [0u8; CODE_BYTES];
    let (mut buffer, mut bits, mut filled, mut symbols) = (0u32, 0u32, 0usize, 0usize);
    for c in text.chars() {
        if c == '-' || c.is_whitespace() {
            continue;
        }
        let c = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        };
        let value = ALPHABET
            .iter()
            .position(|symbol| char::from(*symbol) == c)?;
        symbols += 1;
        if symbols > CODE_BYTES * 8 / 5 {
            return None;
        }
        buffer = (buffer << 5) | value as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            secret[filled] = ((buffer >> bits) & 0xff) as u8;
            filled += 1;
        }
    }
    (symbols == CODE_BYTES * 8 / 5 && filled == CODE_BYTES).then_some(secret)
}

/// Synced settings by key. A key that is absent has never been set.
pub type Settings = BTreeMap<String, String>;

/// Only what may be synced, with values the settings themselves would take.
pub fn admitted(settings: Settings) -> Settings {
    settings
        .into_iter()
        .filter(|(key, value)| key_synced(key) && crate::preferences::value_allowed(key, value))
        .collect()
}

#[derive(Serialize, Deserialize)]
struct Plain {
    aven_sync: u64,
    /// Counts the writes to the box. A device remembers the last it saw and
    /// refuses anything older, so a relay cannot hand back a past state.
    seq: u64,
    settings: Settings,
}

/// The plain text a device encrypts and sends.
pub fn encode(settings: &Settings, seq: u64) -> Vec<u8> {
    serde_json::to_vec(&Plain {
        aven_sync: FORMAT,
        seq,
        settings: settings.clone(),
    })
    .unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlainRefusal {
    TooLarge,
    /// Decrypted, but not what a device sends.
    NotSync,
    /// Sent by a later Aven in a format this one does not read.
    Newer,
}

/// Reads what another device sent, once decrypted. It was written by a
/// device holding the code, but it is still checked like any input: bounded,
/// and kept only where a setting may be synced and takes the value.
pub fn decode(bytes: &[u8]) -> Result<(Settings, u64), PlainRefusal> {
    if bytes.len() > MAX_PLAIN_BYTES {
        return Err(PlainRefusal::TooLarge);
    }
    let json: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| PlainRefusal::NotSync)?;
    let format = json
        .get("aven_sync")
        .and_then(serde_json::Value::as_u64)
        .filter(|format| *format >= 1)
        .ok_or(PlainRefusal::NotSync)?;
    if format > FORMAT {
        return Err(PlainRefusal::Newer);
    }
    let entries = json
        .get("settings")
        .and_then(serde_json::Value::as_object)
        .ok_or(PlainRefusal::NotSync)?;
    let seq = json
        .get("seq")
        .and_then(serde_json::Value::as_u64)
        .ok_or(PlainRefusal::NotSync)?;
    let settings = admitted(
        entries
            .iter()
            .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_owned())))
            .collect(),
    );
    Ok((settings, seq))
}

/// What one round of sync decides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Merge {
    /// What every device should hold once this round is done.
    pub merged: Settings,
    /// Settings to change on this device, in key order.
    pub apply: Vec<(String, String)>,
    /// Whether the relay's copy must be replaced by `merged`.
    pub push: bool,
}

/// The saved setups of both sides, joined rather than one list replacing
/// the other: a setup saved on either device is kept, and one deleted on
/// either is gone. A list one side already holds is returned as that side
/// wrote it, so two devices that agree never rewrite the setting.
///
/// A side that does not hold the setting at all has deleted nothing: it is
/// a device that never saved a setup, or a box that expired on the relay.
/// Only a list that is there, even empty, can say a setup is gone.
fn merge_setups(
    base: Option<&String>,
    here: Option<&String>,
    there: Option<&String>,
) -> Option<String> {
    use crate::setups::{saved, Setup, MAX_SETUPS};
    if here.is_none() || there.is_none() {
        return here.or(there).cloned();
    }
    let list = |value: Option<&String>| -> Vec<Setup> {
        value.and_then(|value| saved(value)).unwrap_or_default()
    };
    let (was, mine, theirs) = (list(base), list(here), list(there));
    let has = |list: &[Setup], id: &str| list.iter().any(|setup| setup.id == id);
    // Mine, less what the other side deleted since we agreed.
    let mut joined: Vec<Setup> = mine
        .iter()
        .filter(|setup| !has(&was, &setup.id) || has(&theirs, &setup.id))
        .cloned()
        .collect();
    // Then what the other side saved since; what it still has only because
    // I deleted it here stays deleted.
    for setup in &theirs {
        if !has(&mine, &setup.id) && !has(&was, &setup.id) && joined.len() < MAX_SETUPS {
            joined.push(setup.clone());
        }
    }
    if let (true, Some(here)) = (joined == mine, here) {
        return Some(here.clone());
    }
    if let (true, Some(there)) = (joined == theirs, there) {
        return Some(there.clone());
    }
    Some(if joined.is_empty() {
        String::new()
    } else {
        serde_json::to_string(&joined).unwrap_or_default()
    })
}

/// Brings this device and the relay's copy together, given what both held
/// the last time they agreed (`base`).
///
/// A setting changed on one side only keeps that change. A setting changed
/// on both sides to different values keeps this device's value: it is the
/// one the person is looking at. Settings cannot be unset, so a key missing
/// from one side is simply taken from the other. The saved setups are a
/// list, and are joined setup by setup instead.
pub fn merge(base: &Settings, local: &Settings, remote: &Settings) -> Merge {
    let mut merged = Settings::new();
    let mut apply = Vec::new();
    let keys: std::collections::BTreeSet<&String> = base
        .keys()
        .chain(local.keys())
        .chain(remote.keys())
        .collect();
    for key in keys {
        let (was, here, there) = (base.get(key), local.get(key), remote.get(key));
        let joined;
        let chosen = if key == crate::setups::SETTING {
            joined = merge_setups(was, here, there);
            match &joined {
                Some(list) => list,
                None => continue,
            }
        } else {
            match (here, there) {
                (Some(here), Some(there)) if here == there => here,
                (Some(here), Some(there)) => {
                    if was == Some(here) {
                        there
                    } else {
                        here
                    }
                }
                (Some(here), None) => here,
                (None, Some(there)) => there,
                (None, None) => continue,
            }
        };
        if here != Some(chosen) {
            apply.push((key.clone(), chosen.clone()));
        }
        merged.insert(key.clone(), chosen.clone());
    }
    let push = &merged != remote;
    Merge {
        merged,
        apply,
        push,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(entries: &[(&str, &str)]) -> Settings {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn what_is_synced_is_a_setting_and_never_one_that_guards_or_belongs_to_a_machine() {
        for key in keys() {
            assert!(crate::preferences::KEYS.contains(&key), "{key}");
            for guarded in [
                "privacy.",
                "network.",
                "ai.",
                "work.",
                "performance.",
                "notice.",
            ] {
                assert!(!key.starts_with(guarded), "{key}");
            }
        }
        for local in [
            "sidebar.width",
            "onboarding",
            "search.custom-url",
            "tabs.startup-pages",
            "tabs.home",
            "focus.blocked",
            "ui.newtab-image",
            "ui.newtab-name",
        ] {
            assert!(!key_synced(local), "{local}");
        }
        assert!(key_synced("appearance") && key_synced("ui.setups") && key_synced("ui.language"));
    }

    #[test]
    fn a_code_reads_back_however_it_was_typed() {
        let secret: [u8; CODE_BYTES] = core::array::from_fn(|index| (index * 13 + 7) as u8);
        let code = format_code(&secret);
        assert_eq!(code.len(), 39);
        assert_eq!(code.split('-').count(), 8);
        assert!(code.split('-').all(|group| group.len() == 4));
        assert!(code
            .chars()
            .all(|c| c == '-' || ALPHABET.contains(&(c as u8))));
        assert_eq!(parse_code(&code), Some(secret));
        assert_eq!(
            parse_code(&code.to_lowercase().replace('-', " ")),
            Some(secret)
        );
        assert_eq!(parse_code(&code.replace('-', "")), Some(secret));
        // The letters taken for digits are read as those digits.
        let zeros = format_code(&[0; CODE_BYTES]);
        assert_eq!(parse_code(&zeros.replace('0', "O")), Some([0; CODE_BYTES]));
        assert_eq!(
            format_code(&[0xff; CODE_BYTES]),
            "ZZZZ-ZZZZ-ZZZZ-ZZZZ-ZZZZ-ZZZZ-ZZZZ-ZZZZ"
        );
    }

    #[test]
    fn text_that_is_not_a_whole_code_is_refused() {
        let code = format_code(&[9; CODE_BYTES]);
        for refused in [
            "",
            "1234",
            &code[..code.len() - 1],
            &format!("{code}0"),
            &code.replace('1', "U"),
            &code.replace('1', "!"),
            "ÉÉÉÉ-ÉÉÉÉ-ÉÉÉÉ-ÉÉÉÉ-ÉÉÉÉ-ÉÉÉÉ-ÉÉÉÉ-ÉÉÉÉ",
        ] {
            assert_eq!(parse_code(refused), None, "{refused}");
        }
    }

    #[test]
    fn what_a_device_sends_reads_back_and_only_what_may_be_synced_is_kept() {
        let sent = settings(&[("appearance", "dark"), ("ui.language", "system")]);
        assert_eq!(decode(&encode(&sent, 7)), Ok((sent, 7)));
        let mixed = br#"{"aven_sync":1,"seq":3,"settings":{
            "appearance":"dark",
            "ui.font":"comic",
            "privacy.smartscreen":"false",
            "network.proxy":"http",
            "sidebar.width":"300",
            "ui.newtab-clock":true
        }}"#;
        assert_eq!(decode(mixed), Ok((settings(&[("appearance", "dark")]), 3)));
        assert_eq!(
            decode(&vec![b' '; MAX_PLAIN_BYTES + 1]),
            Err(PlainRefusal::TooLarge)
        );
        for not_sync in [
            &b"garbage"[..],
            b"[]",
            b"{}",
            br#"{"aven_sync":1,"seq":1}"#,
            br#"{"aven_sync":1,"settings":{}}"#,
            br#"{"aven_sync":0,"seq":1,"settings":{}}"#,
        ] {
            assert_eq!(decode(not_sync), Err(PlainRefusal::NotSync));
        }
        assert_eq!(
            decode(br#"{"aven_sync":2,"seq":1,"settings":{}}"#),
            Err(PlainRefusal::Newer)
        );
    }

    #[test]
    fn a_change_on_one_side_only_is_kept_whichever_side_made_it() {
        let base = settings(&[("appearance", "light"), ("ui.font", "system")]);
        // Changed here: pushed, nothing to apply.
        let here = merge(
            &base,
            &settings(&[("appearance", "dark"), ("ui.font", "system")]),
            &base,
        );
        assert_eq!(
            here.merged.get("appearance").map(String::as_str),
            Some("dark")
        );
        assert!(here.apply.is_empty());
        assert!(here.push);
        // Changed there: applied, nothing to push.
        let there = merge(
            &base,
            &base,
            &settings(&[("appearance", "light"), ("ui.font", "mono")]),
        );
        assert_eq!(there.apply, vec![("ui.font".to_owned(), "mono".to_owned())]);
        assert!(!there.push);
        // One change on each side: both survive.
        let both = merge(
            &base,
            &settings(&[("appearance", "dark"), ("ui.font", "system")]),
            &settings(&[("appearance", "light"), ("ui.font", "mono")]),
        );
        assert_eq!(
            both.merged,
            settings(&[("appearance", "dark"), ("ui.font", "mono")])
        );
        assert_eq!(both.apply, vec![("ui.font".to_owned(), "mono".to_owned())]);
        assert!(both.push);
    }

    #[test]
    fn the_same_setting_changed_on_both_sides_keeps_this_device_s_value() {
        let base = settings(&[("appearance", "system")]);
        let merged = merge(
            &base,
            &settings(&[("appearance", "dark")]),
            &settings(&[("appearance", "light")]),
        );
        assert_eq!(merged.merged, settings(&[("appearance", "dark")]));
        assert!(merged.apply.is_empty());
        assert!(merged.push);
        // Both made the same change: nothing to do.
        let agreed = merge(
            &base,
            &settings(&[("appearance", "dark")]),
            &settings(&[("appearance", "dark")]),
        );
        assert!(agreed.apply.is_empty() && !agreed.push);
    }

    #[test]
    fn a_first_sync_takes_what_each_side_has_and_agreeing_twice_changes_nothing() {
        // A new device with a setting of its own meets a relay with another.
        let first = merge(
            &Settings::new(),
            &settings(&[("ui.font", "mono")]),
            &settings(&[("appearance", "dark")]),
        );
        assert_eq!(
            first.merged,
            settings(&[("appearance", "dark"), ("ui.font", "mono")])
        );
        assert_eq!(
            first.apply,
            vec![("appearance".to_owned(), "dark".to_owned())]
        );
        assert!(first.push);
        // An empty relay takes everything; an empty device receives everything.
        let seed = merge(&Settings::new(), &first.merged, &Settings::new());
        assert!(seed.apply.is_empty() && seed.push);
        let join = merge(&Settings::new(), &Settings::new(), &first.merged);
        assert_eq!(join.apply.len(), 2);
        assert!(!join.push);
        // Once both hold the merged settings, a round is a no-op.
        let again = merge(&first.merged, &first.merged, &first.merged);
        assert!(again.apply.is_empty() && !again.push);
        assert_eq!(again.merged, first.merged);
    }

    fn setup_list(ids: &[&str]) -> String {
        let list: Vec<crate::setups::Setup> = ids
            .iter()
            .map(|id| crate::setups::Setup {
                id: format!("{id:0>16}"),
                name: format!("Setup {id}"),
                settings: [("appearance".to_owned(), "dark".to_owned())].into(),
            })
            .collect();
        if list.is_empty() {
            String::new()
        } else {
            serde_json::to_string(&list).unwrap()
        }
    }

    fn ids_of(settings: &Settings) -> Vec<String> {
        crate::setups::saved(&settings[crate::setups::SETTING])
            .unwrap()
            .into_iter()
            .map(|setup| setup.id.trim_start_matches('0').to_owned())
            .collect()
    }

    fn holding(list: &str) -> Settings {
        settings(&[(crate::setups::SETTING, list)])
    }

    fn lists(base: &[&str], local: &[&str], remote: &[&str]) -> Merge {
        let of = |ids: &[&str]| holding(&setup_list(ids));
        merge(&of(base), &of(local), &of(remote))
    }

    #[test]
    fn setups_saved_on_two_devices_at_once_are_both_kept() {
        let both = lists(&[], &["a"], &["b"]);
        assert_eq!(ids_of(&both.merged), ["a", "b"]);
        assert_eq!(both.apply.len(), 1);
        assert!(both.push);
        assert!(crate::preferences::value_allowed(
            crate::setups::SETTING,
            &both.merged[crate::setups::SETTING]
        ));
        // On top of setups they already shared.
        let more = lists(&["a"], &["a", "b"], &["a", "c"]);
        assert_eq!(ids_of(&more.merged), ["a", "b", "c"]);
    }

    #[test]
    fn a_setup_deleted_on_either_device_is_gone_and_does_not_come_back() {
        // Deleted there: gone here, nothing to push.
        let there = lists(&["a", "b"], &["a", "b"], &["b"]);
        assert_eq!(ids_of(&there.merged), ["b"]);
        assert_eq!(there.apply.len(), 1);
        assert!(!there.push);
        // Deleted here while the other device saved one: both happen.
        let here = lists(&["a", "b"], &["b"], &["a", "b", "c"]);
        assert_eq!(ids_of(&here.merged), ["b", "c"]);
        assert!(here.push);
        // Deleted on both, down to none.
        let none = lists(&["a"], &[], &[]);
        assert_eq!(none.merged[crate::setups::SETTING], "");
        assert!(none.apply.is_empty() && !none.push);
    }

    #[test]
    fn a_box_that_vanished_takes_no_setup_with_it() {
        // The relay's box expired: it holds nothing, which is not a list
        // saying every setup was deleted.
        let kept = merge(
            &holding(&setup_list(&["a", "b"])),
            &holding(&setup_list(&["a", "b"])),
            &Settings::new(),
        );
        assert_eq!(ids_of(&kept.merged), ["a", "b"]);
        assert!(kept.apply.is_empty());
        assert!(kept.push);
        // An empty list that is there does say so.
        let emptied = lists(&["a", "b"], &["a", "b"], &[]);
        assert_eq!(emptied.merged[crate::setups::SETTING], "");
        assert_eq!(emptied.apply.len(), 1);
        // And a device that holds no such setting deletes nothing either.
        let joining = merge(
            &holding(&setup_list(&["a"])),
            &Settings::new(),
            &holding(&setup_list(&["a", "b"])),
        );
        assert_eq!(ids_of(&joining.merged), ["a", "b"]);
    }

    #[test]
    fn agreeing_devices_never_rewrite_the_setups_and_the_list_stays_bounded() {
        let same = lists(&["a", "b"], &["a", "b"], &["a", "b"]);
        assert!(same.apply.is_empty() && !same.push);
        // The same setups written with other spacing are not a change here.
        let spaced = setup_list(&["a"]).replace(",\"name\"", ", \"name\"");
        assert!(crate::setups::saved_allowed(&spaced));
        let reworded = merge(
            &holding(&setup_list(&["a"])),
            &holding(&setup_list(&["a"])),
            &holding(&spaced),
        );
        assert!(reworded.apply.is_empty());
        // A full list takes no more from the other side.
        let full: Vec<String> = (0..crate::setups::MAX_SETUPS)
            .map(|n| format!("{n:x}"))
            .collect();
        let full: Vec<&str> = full.iter().map(String::as_str).collect();
        let bounded = lists(&[], &full, &["ff"]);
        assert_eq!(ids_of(&bounded.merged).len(), crate::setups::MAX_SETUPS);
        assert!(!ids_of(&bounded.merged).contains(&"ff".to_owned()));
        // A device that never saved any takes the other's whole list as is.
        let fresh = merge(
            &Settings::new(),
            &Settings::new(),
            &holding(&setup_list(&["a", "b"])),
        );
        assert_eq!(
            fresh.merged[crate::setups::SETTING],
            setup_list(&["a", "b"])
        );
        assert!(!fresh.push);
    }
}
