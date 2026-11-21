//! Sync between one person's devices, through a relay that cannot read it.
//!
//! A device holds a 160-bit secret, shown as a code to give to another
//! device. Three values are derived from it, each under its own label: the
//! id of a box on the relay, the token that proves knowing the secret, and
//! the key everything is encrypted with before it leaves. The secret is kept
//! by the operating system's credential store, never in a file, a setting or
//! a log.
//!
//! What crosses is decided by `aven_core::sync`: a closed list of settings,
//! merged so that a change made on either device survives. What comes back
//! is checked like any input before a single setting is applied, and each
//! one goes through the same admission as a setting changed by hand.

use super::*;
use aven_core::sync::{self as rules, PlainRefusal, Settings, CODE_BYTES};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::hkdf;
use ring::rand::{SecureRandom, SystemRandom};
use std::time::Duration;
use zeroize::Zeroizing;

/// The relay Aven syncs through. Until one is deployed, sync is not offered.
const RELAY: Option<&str> = None;
const CREDENTIAL: &str = "app.aven.sync";
/// Domain label: salts the derivation and is bound to every ciphertext.
const LABEL: &[u8] = b"aven-sync-v1";
const STATE_FILE: &str = "sync-state.json";
/// The relay's own limit on a box, which no honest answer exceeds.
const MAX_BLOB_BYTES: usize = 256 * 1024;
const FIRST_ROUND_AFTER: Duration = Duration::from_secs(45);
const ROUND_EVERY: Duration = Duration::from_secs(10 * 60);
/// Another device wrote between our read and our write: read again.
const ATTEMPTS: usize = 3;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SyncOutcome {
    Synced,
    /// This device has no sync code.
    Off,
    /// This build has no relay to sync through.
    Unavailable,
    /// The relay could not be reached.
    Offline,
    /// The relay turned the code away.
    Refused,
    /// What the relay holds is not what a device of this code wrote.
    Unreadable,
    /// Another device runs a later Aven that this one cannot read.
    Newer,
    /// A round is already under way, or other devices kept writing.
    Busy,
    Failed,
}

#[derive(Clone, Copy, Debug, Serialize, specta::Type, PartialEq, Eq)]
pub(crate) struct SyncStatusView {
    available: bool,
    on: bool,
}

#[derive(Clone, Debug, Serialize, specta::Type, PartialEq, Eq)]
pub(crate) struct SyncCodeView {
    outcome: SyncOutcome,
    /// The code to type on another device; empty unless `outcome` is synced.
    code: String,
}

/// One round at a time, whoever asks.
static ADMISSION: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

/// A development build may point at a relay under test; a release build
/// only ever speaks to the one compiled in.
fn relay() -> Option<String> {
    #[cfg(debug_assertions)]
    if let Ok(url) = std::env::var("AVEN_SYNC_RELAY") {
        if url.starts_with("https://") || url.starts_with("http://127.0.0.1:") {
            return Some(url.trim_end_matches('/').to_owned());
        }
    }
    RELAY.map(str::to_owned)
}

struct Keys {
    id: String,
    token: Zeroizing<String>,
    key: LessSafeKey,
}

struct Len32;
impl hkdf::KeyType for Len32 {
    fn len(&self) -> usize {
        32
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// The box's id, its token and the key, each under its own label so that
/// knowing one says nothing of another.
fn derive(secret: &[u8; CODE_BYTES]) -> Option<Keys> {
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, LABEL).extract(secret);
    let expand = |info: &'static [u8]| -> Option<Zeroizing<[u8; 32]>> {
        let mut out = Zeroizing::new([0u8; 32]);
        prk.expand(&[info], Len32).ok()?.fill(&mut *out).ok()?;
        Some(out)
    };
    let key = UnboundKey::new(&AES_256_GCM, &*expand(b"key")?).ok()?;
    Some(Keys {
        id: hex(&*expand(b"id")?),
        token: Zeroizing::new(hex(&*expand(b"token")?)),
        key: LessSafeKey::new(key),
    })
}

/// A fresh nonce, then the ciphertext and its tag.
fn seal(key: &LessSafeKey, plain: &[u8]) -> Option<Vec<u8>> {
    let mut nonce = [0u8; NONCE_LEN];
    SystemRandom::new().fill(&mut nonce).ok()?;
    let mut data = plain.to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::from(LABEL),
        &mut data,
    )
    .ok()?;
    let mut blob = nonce.to_vec();
    blob.append(&mut data);
    Some(blob)
}

/// The plain text, or nothing for anything this key did not seal whole.
fn open(key: &LessSafeKey, blob: &[u8]) -> Option<Vec<u8>> {
    if blob.len() < NONCE_LEN || blob.len() > MAX_BLOB_BYTES {
        return None;
    }
    let (nonce, sealed) = blob.split_at(NONCE_LEN);
    let mut data = sealed.to_vec();
    let plain = key
        .open_in_place(
            Nonce::try_assume_unique_for_key(nonce).ok()?,
            Aad::from(LABEL),
            &mut data,
        )
        .ok()?;
    Some(plain.to_vec())
}

fn read_secret() -> Result<Option<Zeroizing<[u8; CODE_BYTES]>>, ()> {
    let _turn = aven_credentials::turn().map_err(|_| ())?;
    match aven_credentials::read(CREDENTIAL) {
        Ok(bytes) => {
            let mut secret = Zeroizing::new([0u8; CODE_BYTES]);
            if bytes.len() != CODE_BYTES {
                return Err(());
            }
            secret.copy_from_slice(&bytes);
            Ok(Some(secret))
        }
        Err(aven_credentials::VaultError::Missing) => Ok(None),
        Err(_) => Err(()),
    }
}

fn write_secret(secret: &[u8; CODE_BYTES]) -> bool {
    let Ok(_turn) = aven_credentials::turn() else {
        return false;
    };
    aven_credentials::write(CREDENTIAL, secret).is_ok()
}

fn forget_secret() -> bool {
    let Ok(_turn) = aven_credentials::turn() else {
        return false;
    };
    matches!(
        aven_credentials::delete(CREDENTIAL),
        Ok(()) | Err(aven_credentials::VaultError::Missing)
    )
}

/// What this device and the relay last agreed on, and the count of writes
/// it has seen. Not secret: it holds only settings this device already has.
#[derive(Default, Serialize, Deserialize)]
struct State {
    base: Settings,
    seq: u64,
}

fn state_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    Some(app.path().app_data_dir().ok()?.join(STATE_FILE))
}

fn load_state(path: &std::path::Path) -> State {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<State>(&bytes).ok())
        .map(|state| State {
            base: rules::admitted(state.base),
            seq: state.seq,
        })
        .unwrap_or_default()
}

fn save_state(path: &std::path::Path, state: &State) -> bool {
    serde_json::to_vec(state)
        .ok()
        .is_some_and(|bytes| data_export::write_export(path, bytes.as_slice()).is_ok())
}

/// The synced settings as this device stores them now.
fn local_settings() -> Settings {
    let Some(store) = APP_STORE.get() else {
        return Settings::new();
    };
    rules::admitted(
        rules::keys()
            .filter_map(|key| Some((key.to_owned(), store.app_setting(key)?)))
            .collect(),
    )
}

enum Read {
    Box { blob: Vec<u8>, version: u64 },
    Empty,
    Offline,
    Refused,
}

enum Written {
    Stored,
    /// Another device wrote first.
    Conflict,
    Offline,
    Refused,
}

fn client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .pool_max_idle_per_host(1)
        .build()
        .ok()
}

fn version_of(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(reqwest::header::ETAG)?
        .to_str()
        .ok()?
        .strip_prefix('"')?
        .strip_suffix('"')?
        .parse()
        .ok()
}

async fn read_box(client: &reqwest::Client, relay: &str, keys: &Keys) -> Read {
    let response = client
        .get(format!("{relay}/v1/sync/{}", keys.id))
        .bearer_auth(keys.token.as_str())
        .send()
        .await;
    let Ok(mut response) = response else {
        return Read::Offline;
    };
    match response.status().as_u16() {
        200 => {}
        404 => return Read::Empty,
        401 | 403 => return Read::Refused,
        _ => return Read::Offline,
    }
    let Some(version) = version_of(&response) else {
        return Read::Offline;
    };
    let mut blob = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) if blob.len() + chunk.len() <= MAX_BLOB_BYTES => {
                blob.extend_from_slice(&chunk);
            }
            Ok(None) => return Read::Box { blob, version },
            _ => return Read::Offline,
        }
    }
}

async fn write_box(
    client: &reqwest::Client,
    relay: &str,
    keys: &Keys,
    on_top_of: Option<u64>,
    blob: Vec<u8>,
) -> Written {
    let request = client
        .put(format!("{relay}/v1/sync/{}", keys.id))
        .bearer_auth(keys.token.as_str())
        .body(blob);
    let request = match on_top_of {
        Some(version) => request.header(reqwest::header::IF_MATCH, format!("\"{version}\"")),
        None => request.header(reqwest::header::IF_NONE_MATCH, "*"),
    };
    match request
        .send()
        .await
        .map(|response| response.status().as_u16())
    {
        Ok(200 | 201) => Written::Stored,
        Ok(412) => Written::Conflict,
        Ok(401 | 403) => Written::Refused,
        _ => Written::Offline,
    }
}

/// One round: read the relay's copy, merge it with this device's settings,
/// apply what the other devices changed, and write back what this one did.
async fn round(app: &tauri::AppHandle) -> SyncOutcome {
    let Some(relay) = relay() else {
        return SyncOutcome::Unavailable;
    };
    let Ok(_permit) = ADMISSION.try_acquire() else {
        return SyncOutcome::Busy;
    };
    let secret = match tokio::task::spawn_blocking(read_secret).await {
        Ok(Ok(Some(secret))) => secret,
        Ok(Ok(None)) => return SyncOutcome::Off,
        _ => return SyncOutcome::Failed,
    };
    let (Some(keys), Some(client), Some(path)) = (derive(&secret), client(), state_path(app))
    else {
        return SyncOutcome::Failed;
    };
    let Some(shell) = app.try_state::<Handle>().map(|state| state.inner().clone()) else {
        return SyncOutcome::Failed;
    };
    let state = {
        let path = path.clone();
        tokio::task::spawn_blocking(move || load_state(&path))
            .await
            .unwrap_or_default()
    };
    for _ in 0..ATTEMPTS {
        let (remote, version, seen) = match read_box(&client, &relay, &keys).await {
            Read::Empty => (Settings::new(), None, 0),
            Read::Box { blob, version } => {
                let Some(plain) = open(&keys.key, &blob) else {
                    return SyncOutcome::Unreadable;
                };
                match rules::decode(&plain) {
                    Ok((settings, seq)) => (settings, Some(version), seq),
                    Err(PlainRefusal::Newer) => return SyncOutcome::Newer,
                    Err(_) => return SyncOutcome::Unreadable,
                }
            }
            Read::Offline => return SyncOutcome::Offline,
            Read::Refused => return SyncOutcome::Refused,
        };
        // A relay handing back a state older than one this device has seen.
        if seen < state.seq {
            return SyncOutcome::Unreadable;
        }
        let local = tokio::task::spawn_blocking(local_settings)
            .await
            .unwrap_or_default();
        let merge = rules::merge(&state.base, &local, &remote);
        for (key, value) in &merge.apply {
            if !setting_value_allowed(key, value)
                || !shell.dispatch(Command::SetAppSetting {
                    key: key.clone(),
                    value: value.clone(),
                })
            {
                return SyncOutcome::Failed;
            }
        }
        let seq = if merge.push { seen + 1 } else { seen };
        if merge.push {
            let Some(blob) = seal(&keys.key, &rules::encode(&merge.merged, seq)) else {
                return SyncOutcome::Failed;
            };
            match write_box(&client, &relay, &keys, version, blob).await {
                Written::Stored => {}
                Written::Conflict => continue,
                Written::Offline => return SyncOutcome::Offline,
                Written::Refused => return SyncOutcome::Refused,
            }
        }
        let agreed = State {
            base: merge.merged,
            seq,
        };
        return match tokio::task::spawn_blocking(move || save_state(&path, &agreed)).await {
            Ok(true) => SyncOutcome::Synced,
            _ => SyncOutcome::Failed,
        };
    }
    SyncOutcome::Busy
}

/// Syncs shortly after start and then every ten minutes, while a code is set.
pub(crate) fn install(app: &tauri::AppHandle) {
    if relay().is_none() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_ROUND_AFTER).await;
        loop {
            if shutdown_started(&app) {
                return;
            }
            let _ = round(&app).await;
            tokio::time::sleep(ROUND_EVERY).await;
        }
    });
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sync_status(caller: WebviewWindow) -> SyncStatusView {
    let off = SyncStatusView {
        available: relay().is_some(),
        on: false,
    };
    if !authorize(&caller, CallerPolicy::Main, "sync_status") {
        return off;
    }
    let on = matches!(
        tokio::task::spawn_blocking(read_secret).await,
        Ok(Ok(Some(_)))
    );
    SyncStatusView { on, ..off }
}

/// This device's code, made on first asking. Showing it is how another
/// device is added, so only the main window may ask.
#[tauri::command]
#[specta::specta]
pub(crate) async fn sync_code(caller: WebviewWindow, app: tauri::AppHandle) -> SyncCodeView {
    let refused = |outcome| SyncCodeView {
        outcome,
        code: String::new(),
    };
    if !authorize(&caller, CallerPolicy::Main, "sync_code") || shutdown_started(&app) {
        return refused(SyncOutcome::Failed);
    }
    if relay().is_none() {
        return refused(SyncOutcome::Unavailable);
    }
    let secret = tokio::task::spawn_blocking(|| match read_secret() {
        Ok(Some(secret)) => Some(secret),
        Ok(None) => {
            let mut fresh = Zeroizing::new([0u8; CODE_BYTES]);
            SystemRandom::new().fill(&mut *fresh).ok()?;
            write_secret(&fresh).then_some(fresh)
        }
        Err(()) => None,
    })
    .await
    .ok()
    .flatten();
    match secret {
        Some(secret) => SyncCodeView {
            outcome: SyncOutcome::Synced,
            code: rules::format_code(&secret),
        },
        None => refused(SyncOutcome::Failed),
    }
}

/// Joins the devices of a code typed here, then syncs once.
#[tauri::command]
#[specta::specta]
pub(crate) async fn sync_join(
    caller: WebviewWindow,
    app: tauri::AppHandle,
    code: String,
) -> SyncOutcome {
    if !authorize(&caller, CallerPolicy::Main, "sync_join") || shutdown_started(&app) {
        return SyncOutcome::Failed;
    }
    if relay().is_none() {
        return SyncOutcome::Unavailable;
    }
    // A code is 39 characters; anything far longer is not one.
    if code.len() > 128 {
        return SyncOutcome::Refused;
    }
    let Some(secret) = rules::parse_code(&code).map(Zeroizing::new) else {
        return SyncOutcome::Refused;
    };
    let stored = tokio::task::spawn_blocking(move || match read_secret() {
        // Already on another code: stop that one first, on purpose.
        Ok(Some(current)) => *current == *secret,
        Ok(None) => write_secret(&secret),
        Err(()) => false,
    })
    .await
    .unwrap_or(false);
    if !stored {
        return SyncOutcome::Failed;
    }
    // What this device agreed with an earlier code says nothing of this one.
    if let Some(path) = state_path(&app) {
        let _ = tokio::task::spawn_blocking(move || std::fs::remove_file(path)).await;
    }
    round(&app).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sync_now(caller: WebviewWindow, app: tauri::AppHandle) -> SyncOutcome {
    if !authorize(&caller, CallerPolicy::Main, "sync_now") || shutdown_started(&app) {
        return SyncOutcome::Failed;
    }
    round(&app).await
}

/// Stops syncing on this device. The settings stay as they are; the code is
/// forgotten, and what the relay holds stays for the other devices.
#[tauri::command]
#[specta::specta]
pub(crate) async fn sync_stop(caller: WebviewWindow, app: tauri::AppHandle) -> bool {
    if !authorize(&caller, CallerPolicy::Main, "sync_stop") || shutdown_started(&app) {
        return false;
    }
    let Ok(_permit) = ADMISSION.acquire().await else {
        return false;
    };
    let path = state_path(&app);
    tokio::task::spawn_blocking(move || {
        if let Some(path) = path {
            let _ = std::fs::remove_file(path);
        }
        forget_secret()
    })
    .await
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: [u8; CODE_BYTES] = [7; CODE_BYTES];

    #[test]
    fn the_id_the_token_and_the_key_differ_and_follow_from_the_secret_alone() {
        let keys = derive(&SECRET).unwrap();
        let again = derive(&SECRET).unwrap();
        assert_eq!(keys.id, again.id);
        assert_eq!(*keys.token, *again.token);
        assert_ne!(keys.id, *keys.token);
        for value in [&keys.id, &*keys.token] {
            assert_eq!(value.len(), 64);
            assert!(value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        }
        let other = derive(&[8; CODE_BYTES]).unwrap();
        assert_ne!(keys.id, other.id);
        assert_ne!(*keys.token, *other.token);
        // Neither value sent to the relay is the secret, nor contains it.
        assert!(!keys.id.contains(&hex(&SECRET)));
        assert!(!keys.token.contains(&hex(&SECRET)));
    }

    #[test]
    fn what_one_device_seals_another_of_the_same_code_opens_and_nobody_else() {
        let keys = derive(&SECRET).unwrap();
        let plain = b"{\"aven_sync\":1}".to_vec();
        let blob = seal(&keys.key, &plain).unwrap();
        assert_ne!(blob[NONCE_LEN..], plain[..]);
        assert_eq!(
            open(&derive(&SECRET).unwrap().key, &blob).as_deref(),
            Some(&plain[..])
        );
        // Sealed twice, the same text never looks the same.
        assert_ne!(seal(&keys.key, &plain).unwrap(), blob);
        // Another code, a changed byte, a cut blob, or too much.
        assert_eq!(open(&derive(&[8; CODE_BYTES]).unwrap().key, &blob), None);
        let mut changed = blob.clone();
        *changed.last_mut().unwrap() ^= 1;
        assert_eq!(open(&keys.key, &changed), None);
        assert_eq!(open(&keys.key, &blob[..blob.len() - 1]), None);
        assert_eq!(open(&keys.key, &blob[..NONCE_LEN - 1]), None);
        assert_eq!(open(&keys.key, &vec![0; MAX_BLOB_BYTES + 1]), None);
    }

    #[test]
    fn the_agreed_state_survives_a_restart_and_a_damaged_file_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        assert_eq!(load_state(&path).seq, 0);
        let state = State {
            base: Settings::from([("appearance".to_owned(), "dark".to_owned())]),
            seq: 4,
        };
        assert!(save_state(&path, &state));
        let back = load_state(&path);
        assert_eq!(back.seq, 4);
        assert_eq!(back.base, state.base);
        // A hand-edited file cannot smuggle in a setting that is not synced.
        std::fs::write(
            &path,
            br#"{"base":{"appearance":"dark","privacy.smartscreen":"false"},"seq":9}"#,
        )
        .unwrap();
        let edited = load_state(&path);
        assert_eq!(edited.base, state.base);
        std::fs::write(&path, b"not json").unwrap();
        assert_eq!(load_state(&path).seq, 0);
        assert!(load_state(&path).base.is_empty());
    }

    #[test]
    fn a_release_build_speaks_only_to_the_relay_compiled_in() {
        // No relay is deployed yet: sync is off the menu, whatever the
        // environment says outside a development build.
        if !cfg!(debug_assertions) {
            assert_eq!(relay(), RELAY.map(str::to_owned));
        }
        assert_eq!(RELAY, None);
    }
}
