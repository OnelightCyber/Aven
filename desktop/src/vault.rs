//! Aven's vault: sign-ins, secure notes, API keys and two-factor codes,
//! kept on this computer only.
//!
//! Each profile has its own vault file under the app's data folder, sealed
//! with AES-256-GCM. Its key is 32 random bytes kept in the Windows
//! Credential Manager for this Windows user, never in the file and never
//! sent anywhere. Only the main window may ask, a secret leaves the vault
//! only when the person asks to see or copy it, and nothing here is logged.
//!
//! Passwords come in through a CSV file the person exports from their old
//! browser themselves: Aven never opens another browser's password store.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewWindow};
use zeroize::Zeroizing;

use crate::{authorize, CallerPolicy};

const MAGIC: &[u8; 8] = b"AVENVLT1";
const MAX_ITEMS: usize = 20_000;
const MAX_FIELD: usize = 64 * 1024;
const MAX_FILE: u64 = 64 * 1024 * 1024;
const MAX_CSV: u64 = 16 * 1024 * 1024;
/// Codes asked for at once: what one screen of the vault can show.
const MAX_CODES: usize = 64;

/// One vault file at a time is read or written.
static LOCK: Mutex<()> = Mutex::new(());
/// Profiles the person asked to delete, and when. A vault goes only with
/// a profile they deleted, once the shell's own list no longer has it.
static LEAVING: Mutex<Vec<(String, std::time::Instant)>> = Mutex::new(Vec::new());
/// How long a deletion is given to show in the shell's list.
const LEAVING_FOR: std::time::Duration = std::time::Duration::from_secs(120);
/// The persistent profile the window browses as, and whether its private
/// scope is shown instead, as the shell last said. The vault opens for that
/// profile only: never another one, and never in a private scope, which
/// keeps nothing on this computer.
static FOCUSED: Mutex<Option<(String, bool)>> = Mutex::new(None);

#[derive(Clone, Copy, Debug, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VaultKindV1 {
    /// A site's sign-in: address, username, password.
    Login,
    /// A secure note.
    Note,
    /// An API key, token or any other secret string.
    Key,
    /// A two-factor secret Aven turns into six-digit codes.
    Totp,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Item {
    id: String,
    kind: VaultKindV1,
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    username: String,
    /// The password, the note's text, the key, or the two-factor secret.
    #[serde(default)]
    secret: String,
    #[serde(default)]
    notes: String,
    /// A sign-in's own two-factor secret, if it has one.
    #[serde(default)]
    totp: String,
    created: f64,
    updated: f64,
}

/// What an item holds is wiped from memory when it is let go: a deleted
/// item, or every item once a look at the vault is over.
impl Drop for Item {
    fn drop(&mut self) {
        use zeroize::Zeroize as _;
        self.username.zeroize();
        self.secret.zeroize();
        self.notes.zeroize();
        self.totp.zeroize();
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Store {
    version: u32,
    items: Vec<Item>,
}

/// An item as the vault page lists it: no secret, only whether there is one.
#[derive(Clone, Debug, Serialize, specta::Type)]
pub(crate) struct VaultItemV1 {
    id: String,
    kind: VaultKindV1,
    title: String,
    url: String,
    username: String,
    notes: String,
    has_secret: bool,
    has_totp: bool,
    updated: f64,
}

/// What the page sends to add or change an item. A secret left out keeps
/// the one already saved.
#[derive(Clone, Debug, Deserialize, specta::Type)]
pub(crate) struct VaultInputV1 {
    id: Option<String>,
    kind: VaultKindV1,
    title: String,
    url: String,
    username: String,
    notes: String,
    secret: Option<String>,
    totp: Option<String>,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
pub(crate) struct VaultImportV1 {
    added: u32,
    updated: u32,
    skipped: u32,
    /// Whether the person closed the file picker without choosing a file.
    cancelled: bool,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
pub(crate) struct VaultCodeV1 {
    id: String,
    code: String,
    /// Seconds before the code changes.
    remaining: u32,
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |elapsed| elapsed.as_secs_f64().floor())
}

fn valid_profile(profile: &str) -> bool {
    !profile.is_empty()
        && profile.len() <= 64
        && profile
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Whether the vault may be opened for `profile`: the persistent profile
/// the window browses as, while its regular pages are shown.
fn focused(focus: Option<&(String, bool)>, profile: &str) -> bool {
    focus.is_some_and(|(current, private)| current == profile && !private)
}

fn bounded(text: &str) -> String {
    let mut text = text.trim_matches(|c: char| c == '\0').to_owned();
    if text.len() > MAX_FIELD {
        let mut cut = MAX_FIELD;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
    }
    text
}

fn file_of(app: &tauri::AppHandle, profile: &str) -> Result<PathBuf, String> {
    let folder = app
        .path()
        .app_data_dir()
        .map_err(|_| "no data folder".to_owned())?
        .join("vault");
    std::fs::create_dir_all(&folder).map_err(|_| "the vault folder cannot be made".to_owned())?;
    Ok(folder.join(format!("{profile}.aven-vault")))
}

/// The profile's key from the Credential Manager, made on first use.
fn key_of(profile: &str, vault_exists: bool) -> Result<LessSafeKey, String> {
    let target = format!("app.aven.vault/{profile}");
    let _turn = aven_credentials::turn().map_err(|_| "the key store is busy".to_owned())?;
    let bytes: Zeroizing<Vec<u8>> = match aven_credentials::read(&target) {
        Ok(bytes) if bytes.len() == 32 => bytes,
        Ok(_) => return Err("the vault key is damaged".to_owned()),
        Err(aven_credentials::VaultError::Missing) if vault_exists => {
            return Err("the vault's key is no longer in this Windows account".to_owned());
        }
        Err(aven_credentials::VaultError::Missing) => {
            let mut fresh = Zeroizing::new(vec![0u8; 32]);
            SystemRandom::new()
                .fill(&mut fresh)
                .map_err(|_| "no randomness".to_owned())?;
            aven_credentials::write(&target, &fresh)
                .map_err(|_| "the vault key cannot be stored".to_owned())?;
            fresh
        }
        Err(_) => return Err("the vault key cannot be read".to_owned()),
    };
    let key =
        UnboundKey::new(&AES_256_GCM, &bytes).map_err(|_| "the vault key is damaged".to_owned())?;
    Ok(LessSafeKey::new(key))
}

fn load(path: &Path, key: &LessSafeKey) -> Result<Store, String> {
    let Ok(meta) = std::fs::metadata(path) else {
        return Ok(Store {
            version: 1,
            items: Vec::new(),
        });
    };
    if meta.len() > MAX_FILE {
        return Err("the vault file is too large".to_owned());
    }
    let raw = std::fs::read(path).map_err(|_| "the vault cannot be read".to_owned())?;
    if raw.len() < MAGIC.len() + NONCE_LEN || &raw[..MAGIC.len()] != MAGIC {
        return Err("the vault file is not one Aven made".to_owned());
    }
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&raw[MAGIC.len()..MAGIC.len() + NONCE_LEN]);
    let mut sealed = Zeroizing::new(raw[MAGIC.len() + NONCE_LEN..].to_vec());
    let open = key
        .open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(MAGIC),
            sealed.as_mut_slice(),
        )
        .map_err(|_| "the vault cannot be opened with this Windows account".to_owned())?;
    serde_json::from_slice(open).map_err(|_| "the vault contents are damaged".to_owned())
}

fn save(path: &Path, key: &LessSafeKey, store: &Store) -> Result<(), String> {
    let mut plain = Zeroizing::new(
        serde_json::to_vec(store).map_err(|_| "the vault cannot be written".to_owned())?,
    );
    let mut nonce = [0u8; NONCE_LEN];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| "no randomness".to_owned())?;
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::from(MAGIC),
        &mut *plain,
    )
    .map_err(|_| "the vault cannot be sealed".to_owned())?;
    let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + plain.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&plain);
    if out.len() as u64 > MAX_FILE {
        return Err("the vault is too large to save".to_owned());
    }
    // Written beside and flushed to the disk, then moved over: neither a
    // crash nor a power cut leaves half a vault. The vault it replaces stays
    // beside it, one save back.
    let written = || -> std::io::Result<()> {
        use std::io::Write;
        let temporary = path.with_extension("aven-vault.new");
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&out)?;
        file.sync_all()?;
        drop(file);
        if path.exists() {
            std::fs::copy(path, path.with_extension("aven-vault.bak"))?;
        }
        std::fs::rename(&temporary, path)
    };
    written().map_err(|_| "the vault cannot be written".to_owned())
}

/// Opens the profile's vault, applies `change`, and saves it if asked.
fn with_store<T>(
    app: &tauri::AppHandle,
    profile: &str,
    change: impl FnOnce(&mut Store) -> Result<(T, bool), String>,
) -> Result<T, String> {
    let open = focused(
        FOCUSED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref(),
        profile,
    );
    if !valid_profile(profile) || !open {
        return Err("unknown profile".to_owned());
    }
    let _guard = LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = file_of(app, profile)?;
    if !path.exists() {
        // Nothing kept yet: a look needs no key, and only a first save
        // makes one in the Credential Manager.
        let mut store = Store {
            version: 1,
            items: Vec::new(),
        };
        let (value, dirty) = change(&mut store)?;
        if dirty {
            save(&path, &key_of(profile, false)?, &store)?;
        }
        return Ok(value);
    }
    let key = key_of(profile, true)?;
    let mut store = load(&path, &key)?;
    let (value, dirty) = change(&mut store)?;
    if dirty {
        save(&path, &key, &store)?;
    }
    Ok(value)
}

fn view(item: &Item) -> VaultItemV1 {
    VaultItemV1 {
        id: item.id.clone(),
        kind: item.kind,
        title: item.title.clone(),
        url: item.url.clone(),
        username: item.username.clone(),
        notes: if item.kind == VaultKindV1::Login {
            item.notes.clone()
        } else {
            String::new()
        },
        has_secret: !item.secret.is_empty(),
        has_totp: !item.totp.is_empty() || item.kind == VaultKindV1::Totp,
        updated: item.updated,
    }
}

fn new_id() -> Result<String, String> {
    let mut bytes = [0u8; 12];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "no randomness".to_owned())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn host_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| {
            parsed
                .host_str()
                .map(|host| host.trim_start_matches("www.").to_ascii_lowercase())
        })
        .unwrap_or_default()
}

// ---- A vault goes with its profile ------------------------------------------

/// The person asked for this profile to be deleted.
pub(crate) fn profile_leaving(profile: &str) {
    if !valid_profile(profile) {
        return;
    }
    let mut leaving = LEAVING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    leaving.retain(|(known, since)| known != profile && since.elapsed() < LEAVING_FOR);
    leaving.push((profile.to_owned(), std::time::Instant::now()));
}

/// The profiles that left: asked to be deleted, recently, and absent from
/// `listed`. An empty list says nothing: it is never read as every
/// profile being gone.
fn gone_profiles(leaving: &mut Vec<(String, std::time::Instant)>, listed: &[&str]) -> Vec<String> {
    leaving.retain(|(_, since)| since.elapsed() < LEAVING_FOR);
    if listed.is_empty() {
        return Vec::new();
    }
    let gone: Vec<String> = leaving
        .iter()
        .filter(|(profile, _)| !listed.contains(&profile.as_str()))
        .map(|(profile, _)| profile.clone())
        .collect();
    leaving.retain(|(profile, _)| listed.contains(&profile.as_str()));
    gone
}

/// Follows the shell's list of profiles: a profile the person just
/// deleted, and that the list no longer has, takes its vault and its key
/// with it.
pub(crate) fn follow_profiles(app: &tauri::AppHandle, view: &aven_ipc::ProfilesView) {
    *FOCUSED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = view
        .current
        .clone()
        .filter(|current| view.profiles.iter().any(|entry| &entry.id == current))
        .map(|current| (current, view.private));
    let listed: Vec<&str> = view
        .profiles
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    let gone = gone_profiles(
        &mut LEAVING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
        &listed,
    );
    for profile in gone {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || forget(&app, &profile));
    }
}

/// Removes a deleted profile's vault, its backup and its key.
fn forget(app: &tauri::AppHandle, profile: &str) {
    if !valid_profile(profile) {
        return;
    }
    let _guard = LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Ok(path) = file_of(app, profile) {
        forget_files(&path);
    }
    if let Ok(_turn) = aven_credentials::turn() {
        let _ = aven_credentials::delete(&format!("app.aven.vault/{profile}"));
    }
}

fn forget_files(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("aven-vault.bak"));
}

// ---- Commands -------------------------------------------------------------

/// Runs vault work off the UI thread. It reads a file, waits for the
/// Credential Manager and decrypts, and that thread also decides every
/// page's requests.
async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| "the vault could not be reached".to_owned())?
}

/// Every item of the profile's vault, without any secret.
#[tauri::command]
#[specta::specta]
pub(crate) async fn vault_list(
    caller: WebviewWindow,
    profile: String,
) -> Result<Vec<VaultItemV1>, String> {
    if !authorize(&caller, CallerPolicy::Main, "vault_list") {
        return Err("denied".to_owned());
    }
    let app = caller.app_handle().clone();
    off_thread(move || {
        with_store(&app, &profile, |store| {
            let mut items: Vec<VaultItemV1> = store.items.iter().map(view).collect();
            items.sort_by_cached_key(|item| item.title.to_lowercase());
            Ok((items, false))
        })
    })
    .await
}

/// One item's secret, when the person asks to see or copy it.
#[tauri::command]
#[specta::specta]
pub(crate) async fn vault_reveal(
    caller: WebviewWindow,
    profile: String,
    id: String,
) -> Result<String, String> {
    if !authorize(&caller, CallerPolicy::Main, "vault_reveal") {
        return Err("denied".to_owned());
    }
    let app = caller.app_handle().clone();
    off_thread(move || {
        with_store(&app, &profile, |store| {
            let item = store
                .items
                .iter()
                .find(|item| item.id == id)
                .ok_or("not found")?;
            Ok((item.secret.clone(), false))
        })
    })
    .await
}

/// Adds an item, or changes one. Returns its id.
#[tauri::command]
#[specta::specta]
pub(crate) async fn vault_save(
    caller: WebviewWindow,
    profile: String,
    item: VaultInputV1,
) -> Result<String, String> {
    if !authorize(&caller, CallerPolicy::Main, "vault_save") {
        return Err("denied".to_owned());
    }
    let app = caller.app_handle().clone();
    off_thread(move || {
        with_store(&app, &profile, |store| {
            let at = now();
            let existing = item
                .id
                .as_ref()
                .and_then(|id| store.items.iter().position(|known| &known.id == id));
            let id = match existing {
                Some(index) => {
                    let known = &mut store.items[index];
                    known.kind = item.kind;
                    known.title = bounded(&item.title);
                    known.url = bounded(&item.url);
                    known.username = bounded(&item.username);
                    known.notes = bounded(&item.notes);
                    if let Some(secret) = &item.secret {
                        known.secret = bounded(secret);
                    }
                    if let Some(totp) = &item.totp {
                        known.totp = bounded(totp);
                    }
                    known.updated = at;
                    known.id.clone()
                }
                None => {
                    if store.items.len() >= MAX_ITEMS {
                        return Err("the vault is full".to_owned());
                    }
                    let id = new_id()?;
                    store.items.push(Item {
                        id: id.clone(),
                        kind: item.kind,
                        title: bounded(&item.title),
                        url: bounded(&item.url),
                        username: bounded(&item.username),
                        secret: bounded(item.secret.as_deref().unwrap_or_default()),
                        notes: bounded(&item.notes),
                        totp: bounded(item.totp.as_deref().unwrap_or_default()),
                        created: at,
                        updated: at,
                    });
                    id
                }
            };
            Ok((id, true))
        })
    })
    .await
}

/// Removes an item for good.
#[tauri::command]
#[specta::specta]
pub(crate) async fn vault_delete(
    caller: WebviewWindow,
    profile: String,
    id: String,
) -> Result<(), String> {
    if !authorize(&caller, CallerPolicy::Main, "vault_delete") {
        return Err("denied".to_owned());
    }
    let app = caller.app_handle().clone();
    off_thread(move || {
        with_store(&app, &profile, |store| {
            let before = store.items.len();
            store.items.retain(|item| item.id != id);
            Ok(((), store.items.len() != before))
        })
    })
    .await
}

/// The current codes of the two-factor secrets asked for, with the seconds
/// each has left: one look at the vault for all of them. An item without a
/// valid secret is left out.
#[tauri::command]
#[specta::specta]
pub(crate) async fn vault_codes(
    caller: WebviewWindow,
    profile: String,
    ids: Vec<String>,
) -> Result<Vec<VaultCodeV1>, String> {
    if !authorize(&caller, CallerPolicy::Main, "vault_codes") {
        return Err("denied".to_owned());
    }
    if ids.len() > MAX_CODES || ids.iter().any(|id| id.len() > 64) {
        return Err("too many codes asked for".to_owned());
    }
    let app = caller.app_handle().clone();
    off_thread(move || {
        with_store(&app, &profile, |store| {
            let codes = store
                .items
                .iter()
                .filter(|item| ids.contains(&item.id))
                .filter_map(|item| {
                    let secret = if item.kind == VaultKindV1::Totp {
                        &item.secret
                    } else {
                        &item.totp
                    };
                    let (code, remaining) = totp::now(secret)?;
                    Some(VaultCodeV1 {
                        id: item.id.clone(),
                        code,
                        remaining,
                    })
                })
                .collect();
            Ok((codes, false))
        })
    })
    .await
}

/// Fills a saved sign-in into the page in front. It goes only to the
/// encrypted site it was saved for: the answer is false, and nothing is
/// sent, when the page in front is any other.
#[tauri::command]
#[specta::specta]
pub(crate) async fn vault_fill(
    caller: WebviewWindow,
    profile: String,
    id: String,
) -> Result<bool, String> {
    if !authorize(&caller, CallerPolicy::Main, "vault_fill") {
        return Err("denied".to_owned());
    }
    let app = caller.app_handle().clone();
    off_thread(move || {
        let fill = with_store(&app, &profile, |store| {
            let item = store
                .items
                .iter()
                .find(|item| item.id == id && item.kind == VaultKindV1::Login)
                .ok_or("not found")?;
            let fill = aven_core::login_fill::LoginFill::new(
                &item.url,
                item.username.clone(),
                item.secret.clone(),
            )
            .ok_or("this sign-in has no secure address to be filled on")?;
            Ok((fill, false))
        })?;
        let shell = app
            .try_state::<aven_app::Handle>()
            .ok_or("the browser is not ready")?;
        let (reply, answer) = std::sync::mpsc::sync_channel(1);
        if !shell.dispatch(aven_app::Command::FillLogin { fill, reply }) {
            return Err("the browser is busy".to_owned());
        }
        answer
            .recv_timeout(std::time::Duration::from_secs(3))
            .map_err(|_| "the browser did not answer".to_owned())
    })
    .await
}

/// What saving a typed sign-in did.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VaultTypedV1 {
    /// A new item was added.
    Saved,
    /// The item kept for that name on that site now holds this password.
    Updated,
    /// The page in front has no sign-in typed into it, or is not encrypted.
    Nothing,
}

/// What a page says is typed in its sign-in form.
#[derive(Deserialize)]
struct Typed {
    #[serde(default)]
    name: String,
    secret: String,
}

/// The sign-in a page reported, checked: an encrypted address, a
/// password that is there, both within bounds. Where the page says it is
/// counts for nothing: the address is the one the browser committed.
fn typed_sign_in(address: &str, typed: &str) -> Option<(String, String, Zeroizing<String>)> {
    let page = url::Url::parse(address)
        .ok()
        .filter(|page| page.scheme() == "https" && page.host_str().is_some())?;
    let typed: Typed = serde_json::from_str(typed).ok()?;
    let secret = Zeroizing::new(typed.secret);
    if secret.is_empty() || secret.len() > 4096 || typed.name.len() > 1024 {
        return None;
    }
    let origin = format!("{}/", page.origin().ascii_serialization());
    Some((origin, typed.name, secret))
}

/// Saves the sign-in typed into the page in front as an item for that
/// page's site, or puts its password in the item already kept for the
/// same name there. The person asks from the site menu; the page is
/// read once, then, and what it holds is never logged.
#[tauri::command]
#[specta::specta]
pub(crate) async fn vault_save_typed(
    caller: WebviewWindow,
    profile: String,
) -> Result<VaultTypedV1, String> {
    if !authorize(&caller, CallerPolicy::Main, "vault_save_typed") {
        return Err("denied".to_owned());
    }
    let app = caller.app_handle().clone();
    let front = {
        let app = app.clone();
        off_thread(move || {
            let shell = app
                .try_state::<aven_app::Handle>()
                .ok_or("the browser is not ready")?;
            let (reply, answer) = std::sync::mpsc::sync_channel(1);
            if !shell.dispatch(aven_app::Command::PageInFront { reply }) {
                return Err("the browser is busy".to_owned());
            }
            answer
                .recv_timeout(std::time::Duration::from_secs(3))
                .map_err(|_| "the browser did not answer".to_owned())
        })
        .await?
    };
    let Some((item, _)) = front else {
        return Ok(VaultTypedV1::Nothing);
    };
    let Some((address, typed)) = crate::page_assist::typed_login(item).await else {
        return Ok(VaultTypedV1::Nothing);
    };
    let typed = Zeroizing::new(typed);
    off_thread(move || {
        let Some((origin, name, secret)) = typed_sign_in(&address, &typed) else {
            return Ok(VaultTypedV1::Nothing);
        };
        with_store(&app, &profile, |store| {
            let at = now();
            let site = host_of(&origin);
            let name = bounded(&name);
            if let Some(known) = store.items.iter_mut().find(|item| {
                item.kind == VaultKindV1::Login
                    && host_of(&item.url) == site
                    && item.username == name
            }) {
                if known.secret == *secret {
                    return Ok((VaultTypedV1::Updated, false));
                }
                known.secret = bounded(&secret);
                known.updated = at;
                return Ok((VaultTypedV1::Updated, true));
            }
            if store.items.len() >= MAX_ITEMS {
                return Err("the vault is full".to_owned());
            }
            store.items.push(Item {
                id: new_id()?,
                kind: VaultKindV1::Login,
                title: site,
                url: origin,
                username: name,
                secret: bounded(&secret),
                notes: String::new(),
                totp: String::new(),
                created: at,
                updated: at,
            });
            Ok((VaultTypedV1::Saved, true))
        })
    })
    .await
}

/// A strong random password, from letters, digits and, if asked, symbols.
#[tauri::command]
#[specta::specta]
pub(crate) fn vault_generate(
    caller: WebviewWindow,
    length: u32,
    symbols: bool,
) -> Result<String, String> {
    if !authorize(&caller, CallerPolicy::Main, "vault_generate") {
        return Err("denied".to_owned());
    }
    generate(length.clamp(8, 128) as usize, symbols)
}

/// Imports a CSV file of passwords the person exported from another
/// browser or password manager (Opera, Opera GX, Chrome, Edge, Brave,
/// Firefox, Bitwarden). A sign-in already saved for the same site and
/// username is updated rather than added twice.
#[tauri::command]
#[specta::specta]
pub(crate) async fn vault_import(
    caller: WebviewWindow,
    profile: String,
) -> Result<VaultImportV1, String> {
    use tauri_plugin_dialog::DialogExt;
    if !authorize(&caller, CallerPolicy::Main, "vault_import") {
        return Err("denied".to_owned());
    }
    let app = caller.app_handle().clone();
    let picker = app.clone();
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .add_filter("CSV", &["csv"])
            .blocking_pick_file()
    })
    .await
    .map_err(|_| "the file picker failed".to_owned())?;
    let Some(chosen) = chosen else {
        return Ok(VaultImportV1 {
            added: 0,
            updated: 0,
            skipped: 0,
            cancelled: true,
        });
    };
    let path = chosen
        .into_path()
        .map_err(|_| "that file cannot be read".to_owned())?;
    tauri::async_runtime::spawn_blocking(move || {
        if std::fs::metadata(&path)
            .map_err(|_| "that file cannot be read".to_owned())?
            .len()
            > MAX_CSV
        {
            return Err("that file is too large".to_owned());
        }
        let text = Zeroizing::new(
            std::fs::read_to_string(&path).map_err(|_| "that file is not text".to_owned())?,
        );
        let rows =
            csv::logins(&text).ok_or("this file is not a password export Aven recognizes")?;
        with_store(&app, &profile, |store| {
            let at = now();
            let (mut added, mut updated, mut skipped) = (0u32, 0u32, 0u32);
            let mut known: std::collections::HashMap<(String, String), usize> = store
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.kind == VaultKindV1::Login)
                .map(|(index, item)| ((host_of(&item.url), item.username.clone()), index))
                .collect();
            for row in rows {
                if row.password.is_empty() && row.totp.is_empty() {
                    skipped += 1;
                    continue;
                }
                let host = host_of(&row.url);
                let place = (host.clone(), row.username.clone());
                if let Some(item) = known
                    .get(&place)
                    .and_then(|index| store.items.get_mut(*index))
                {
                    if item.secret != row.password
                        || (!row.totp.is_empty() && item.totp != row.totp)
                    {
                        item.secret = bounded(&row.password);
                        if !row.totp.is_empty() {
                            item.totp = bounded(&row.totp);
                        }
                        item.updated = at;
                        updated += 1;
                    } else {
                        skipped += 1;
                    }
                    continue;
                }
                if store.items.len() >= MAX_ITEMS {
                    skipped += 1;
                    continue;
                }
                let title = if row.title.is_empty() {
                    host.clone()
                } else {
                    row.title.clone()
                };
                known.insert(place, store.items.len());
                store.items.push(Item {
                    id: new_id()?,
                    kind: VaultKindV1::Login,
                    title: bounded(&title),
                    url: bounded(&row.url),
                    username: bounded(&row.username),
                    secret: bounded(&row.password),
                    notes: bounded(&row.notes),
                    totp: bounded(&row.totp),
                    created: at,
                    updated: at,
                });
                added += 1;
            }
            Ok((
                VaultImportV1 {
                    added,
                    updated,
                    skipped,
                    cancelled: false,
                },
                added + updated > 0,
            ))
        })
    })
    .await
    .map_err(|_| "the import failed".to_owned())?
}

fn generate(length: usize, symbols: bool) -> Result<String, String> {
    const LETTERS: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";
    const DIGITS: &[u8] = b"23456789";
    const SYMBOLS: &[u8] = b"!@#$%^&*-_=+?";
    let mut pool: Vec<u8> = LETTERS.iter().chain(DIGITS).copied().collect();
    if symbols {
        pool.extend_from_slice(SYMBOLS);
    }
    let random = SystemRandom::new();
    // A number below `count`, every one equally likely: draws past the last
    // whole multiple of `count` are thrown away rather than folded back.
    let below = |count: usize| -> Result<usize, String> {
        const SPAN: u64 = 1 << 32;
        let count = count as u64;
        let limit = SPAN - SPAN % count;
        loop {
            let mut bytes = [0u8; 4];
            random
                .fill(&mut bytes)
                .map_err(|_| "no randomness".to_owned())?;
            let drawn = u64::from(u32::from_le_bytes(bytes));
            if drawn < limit {
                return Ok((drawn % count) as usize);
            }
        }
    };
    let pick = |from: &[u8]| below(from.len()).map(|index| from[index]);
    let mut out: Vec<u8> = Vec::with_capacity(length);
    // At least one letter and one digit, and a symbol when asked.
    out.push(pick(LETTERS)?);
    out.push(pick(DIGITS)?);
    if symbols {
        out.push(pick(SYMBOLS)?);
    }
    while out.len() < length {
        out.push(pick(&pool)?);
    }
    // Shuffle so the required characters are not always first.
    for index in (1..out.len()).rev() {
        out.swap(index, below(index + 1)?);
    }
    String::from_utf8(out).map_err(|_| "no password".to_owned())
}

/// Time-based one-time codes (RFC 6238), from a base32 secret or an
/// `otpauth://` link.
mod totp {
    use ring::hmac;

    fn base32(text: &str) -> Option<Vec<u8>> {
        let mut bits = 0u64;
        let mut count = 0u32;
        let mut out = Vec::new();
        for c in text
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-' && *c != '=')
        {
            let value = match c.to_ascii_uppercase() {
                c @ 'A'..='Z' => c as u64 - 'A' as u64,
                c @ '2'..='7' => c as u64 - '2' as u64 + 26,
                _ => return None,
            };
            bits = (bits << 5) | value;
            count += 5;
            if count >= 8 {
                count -= 8;
                out.push((bits >> count) as u8);
                bits &= (1 << count) - 1;
            }
        }
        (!out.is_empty()).then_some(out)
    }

    struct Spec {
        key: Vec<u8>,
        digits: u32,
        period: u64,
        algorithm: hmac::Algorithm,
    }

    fn spec(secret: &str) -> Option<Spec> {
        let secret = secret.trim();
        if let Some(rest) = secret.strip_prefix("otpauth://") {
            let query = rest.split_once('?')?.1;
            let mut key = None;
            let (mut digits, mut period, mut algorithm) =
                (6, 30, hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY);
            for pair in query.split('&') {
                let (name, value) = pair.split_once('=')?;
                match name.to_ascii_lowercase().as_str() {
                    "secret" => key = base32(value),
                    "digits" => digits = value.parse().ok().filter(|d| (6..=8).contains(d))?,
                    "period" => period = value.parse().ok().filter(|p| (10..=300).contains(p))?,
                    "algorithm" => {
                        algorithm = match value.to_ascii_uppercase().as_str() {
                            "SHA256" => hmac::HMAC_SHA256,
                            "SHA512" => hmac::HMAC_SHA512,
                            _ => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
                        }
                    }
                    _ => {}
                }
            }
            return Some(Spec {
                key: key?,
                digits,
                period,
                algorithm,
            });
        }
        Some(Spec {
            key: base32(secret)?,
            digits: 6,
            period: 30,
            algorithm: hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
        })
    }

    pub(super) fn at(secret: &str, unix: u64) -> Option<(String, u32)> {
        let spec = spec(secret)?;
        let counter = unix / spec.period;
        let key = hmac::Key::new(spec.algorithm, &spec.key);
        let tag = hmac::sign(&key, &counter.to_be_bytes());
        let hash = tag.as_ref();
        let offset = (hash[hash.len() - 1] & 0x0f) as usize;
        let value = (u32::from(hash[offset] & 0x7f) << 24)
            | (u32::from(hash[offset + 1]) << 16)
            | (u32::from(hash[offset + 2]) << 8)
            | u32::from(hash[offset + 3]);
        let code = value % 10u32.pow(spec.digits);
        let remaining = (spec.period - unix % spec.period) as u32;
        Some((
            format!("{code:0width$}", width = spec.digits as usize),
            remaining,
        ))
    }

    pub(super) fn now(secret: &str) -> Option<(String, u32)> {
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        at(secret, unix)
    }
}

/// Password exports as other browsers write them.
mod csv {
    pub(super) struct Login {
        pub(super) title: String,
        pub(super) url: String,
        pub(super) username: String,
        pub(super) password: String,
        pub(super) notes: String,
        pub(super) totp: String,
    }

    /// RFC 4180 records: quoted fields may hold commas, quotes and newlines.
    fn records(text: &str) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        let mut row = Vec::new();
        let mut field = String::new();
        let mut quoted = false;
        let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
        while let Some(c) = chars.next() {
            if quoted {
                if c == '"' {
                    if chars.peek() == Some(&'"') {
                        field.push('"');
                        chars.next();
                    } else {
                        quoted = false;
                    }
                } else {
                    field.push(c);
                }
                continue;
            }
            match c {
                '"' => quoted = true,
                ',' => row.push(std::mem::take(&mut field)),
                '\r' => {}
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    if row.iter().any(|cell| !cell.is_empty()) {
                        rows.push(std::mem::take(&mut row));
                    } else {
                        row.clear();
                    }
                }
                _ => field.push(c),
            }
        }
        row.push(field);
        if row.iter().any(|cell| !cell.is_empty()) {
            rows.push(row);
        }
        rows
    }

    /// The sign-ins of an export, read by its header's column names.
    pub(super) fn logins(text: &str) -> Option<Vec<Login>> {
        let mut rows = records(text).into_iter();
        let header: Vec<String> = rows
            .next()?
            .into_iter()
            .map(|cell| cell.trim().to_ascii_lowercase())
            .collect();
        let column = |names: &[&str]| {
            names
                .iter()
                .find_map(|name| header.iter().position(|cell| cell == name))
        };
        let url = column(&["url", "login_uri", "website", "origin", "uri"]);
        let username = column(&["username", "login_username", "user", "email", "login"]);
        let password = column(&["password", "login_password"])?;
        let title = column(&["name", "title"]);
        let notes = column(&["note", "notes", "extra"]);
        let totp = column(&["totp", "login_totp", "otpauth"]);
        let cell = |row: &Vec<String>, index: Option<usize>| {
            index.and_then(|i| row.get(i)).cloned().unwrap_or_default()
        };
        Some(
            rows.map(|row| Login {
                title: cell(&row, title),
                url: cell(&row, url),
                username: cell(&row, username),
                password: cell(&row, Some(password)),
                notes: cell(&row, notes),
                totp: cell(&row, totp),
            })
            .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vault_goes_only_with_a_profile_the_person_deleted() {
        let now = std::time::Instant::now();
        let mut leaving = vec![("gone".to_owned(), now), ("kept".to_owned(), now)];
        // An empty list is never read as every profile being gone.
        assert!(gone_profiles(&mut leaving, &[]).is_empty());
        assert_eq!(leaving.len(), 2);
        // Absent and asked for: gone. Absent but never asked for: untouched.
        assert_eq!(gone_profiles(&mut leaving, &["kept", "other"]), ["gone"]);
        // Still listed: its deletion may be under way, so it stays marked
        // until the list drops it.
        assert_eq!(leaving.len(), 1);
        assert_eq!(gone_profiles(&mut leaving, &["other"]), ["kept"]);
        assert!(leaving.is_empty());
        // Never asked for: absent or not, nothing goes.
        assert!(gone_profiles(&mut leaving, &["other"]).is_empty());
    }

    #[test]
    fn only_the_profile_the_window_browses_as_opens_its_vault() {
        let regular = ("01ABC".to_owned(), false);
        assert!(focused(Some(&regular), "01ABC"));
        // Another profile, a private scope, and no word yet from the shell.
        assert!(!focused(Some(&regular), "01XYZ"));
        assert!(!focused(Some(&("01ABC".to_owned(), true)), "01ABC"));
        assert!(!focused(None, "01ABC"));
    }

    #[test]
    fn a_deleted_profile_takes_its_vault_and_its_backup_with_it() {
        let folder = tempfile::tempdir().expect("folder");
        let path = folder.path().join("p.aven-vault");
        let backup = folder.path().join("p.aven-vault.bak");
        let other = folder.path().join("q.aven-vault");
        for file in [&path, &backup, &other] {
            std::fs::write(file, b"sealed").expect("written");
        }
        forget_files(&path);
        assert!(!path.exists() && !backup.exists());
        assert!(other.exists());
    }

    #[test]
    fn a_typed_sign_in_is_kept_for_the_address_the_browser_committed() {
        let (origin, name, secret) = typed_sign_in(
            "https://www.example.com/login?next=/",
            r#"{"host":"evil.example","name":"me@example.com","secret":"hunter2"}"#,
        )
        .expect("a sign-in");
        assert_eq!(origin, "https://www.example.com/");
        assert_eq!(name, "me@example.com");
        assert_eq!(*secret, "hunter2");
        assert_eq!(host_of(&origin), "example.com");
    }

    #[test]
    fn a_typed_sign_in_is_refused_off_an_encrypted_page_or_without_a_password() {
        let typed = r#"{"name":"me","secret":"hunter2"}"#;
        assert!(typed_sign_in("http://example.com/login", typed).is_none());
        assert!(typed_sign_in("file:///C:/login.html", typed).is_none());
        assert!(typed_sign_in("https://example.com/", r#"{"name":"me","secret":""}"#).is_none());
        assert!(typed_sign_in("https://example.com/", "null").is_none());
        let long = format!(r#"{{"name":"me","secret":"{}"}}"#, "x".repeat(5000));
        assert!(typed_sign_in("https://example.com/", &long).is_none());
    }

    #[test]
    fn totp_matches_the_rfc_6238_vectors() {
        // RFC 6238 SHA1 secret "12345678901234567890" in base32.
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        let (code, _) =
            totp::at(&format!("otpauth://totp/x?secret={secret}&digits=8"), 59).expect("code");
        assert_eq!(code, "94287082");
        let (code, _) = totp::at(
            &format!("otpauth://totp/x?secret={secret}&digits=8"),
            1_111_111_109,
        )
        .expect("code");
        assert_eq!(code, "07081804");
        let (six, remaining) = totp::at(secret, 59).expect("code");
        assert_eq!(six, "287082");
        assert_eq!(remaining, 1);
    }

    #[test]
    fn chrome_opera_firefox_and_bitwarden_exports_are_read() {
        let chrome = "name,url,username,password,note\nexample.com,https://example.com/login,ada,\"p,a\"\"ss\",\n";
        let rows = csv::logins(chrome).expect("chrome");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].password, "p,a\"ss");
        assert_eq!(rows[0].username, "ada");

        let firefox = "\"url\",\"username\",\"password\",\"httpRealm\"\n\"https://a.test\",\"bo\",\"x1\",\"\"\n";
        assert_eq!(csv::logins(firefox).expect("firefox")[0].password, "x1");

        let bitwarden = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n,,login,Site,,,0,https://b.test,cy,pw,JBSWY3DPEHPK3PXP\n";
        let row = &csv::logins(bitwarden).expect("bitwarden")[0];
        assert_eq!(
            (row.url.as_str(), row.username.as_str(), row.totp.as_str()),
            ("https://b.test", "cy", "JBSWY3DPEHPK3PXP")
        );

        assert!(csv::logins("a,b\n1,2\n").is_none(), "no password column");
    }

    #[test]
    fn generated_passwords_have_the_length_and_kinds_asked() {
        let password = generate(24, true).expect("password");
        assert_eq!(password.len(), 24);
        assert!(password.chars().any(|c| c.is_ascii_digit()));
        assert!(password.chars().any(|c| "!@#$%^&*-_=+?".contains(c)));
        assert_ne!(generate(24, true).expect("again"), password);
    }

    #[test]
    fn sealed_vaults_round_trip_and_refuse_another_key() {
        let folder = tempfile::tempdir().expect("folder");
        let path = folder.path().join("p.aven-vault");
        let key = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, &[7u8; 32]).expect("key"));
        let store = Store {
            version: 1,
            items: vec![Item {
                id: "a".into(),
                kind: VaultKindV1::Login,
                title: "Site".into(),
                url: "https://site.test".into(),
                username: "me".into(),
                secret: "hunter2".into(),
                notes: String::new(),
                totp: String::new(),
                created: 1.0,
                updated: 1.0,
            }],
        };
        save(&path, &key, &store).expect("save");
        let raw = std::fs::read(&path).expect("raw");
        assert!(
            !raw.windows(7).any(|window| window == b"hunter2"),
            "the password is not stored in clear"
        );
        assert_eq!(load(&path, &key).expect("load").items[0].secret, "hunter2");
        let other = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, &[8u8; 32]).expect("key"));
        assert!(load(&path, &other).is_err());
    }

    #[test]
    fn a_save_keeps_the_vault_it_replaces_and_leaves_nothing_half_written() {
        let folder = tempfile::tempdir().expect("folder");
        let path = folder.path().join("p.aven-vault");
        let backup = folder.path().join("p.aven-vault.bak");
        let key = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, &[7u8; 32]).expect("key"));
        let item = |secret: &str| Item {
            id: "a".into(),
            kind: VaultKindV1::Login,
            title: "Site".into(),
            url: "https://site.test".into(),
            username: "me".into(),
            secret: secret.into(),
            notes: String::new(),
            totp: String::new(),
            created: 1.0,
            updated: 1.0,
        };
        let holding = |secret: &str| Store {
            version: 1,
            items: vec![item(secret)],
        };

        save(&path, &key, &holding("first")).expect("first save");
        assert!(!backup.exists(), "a first save has nothing to keep");

        save(&path, &key, &holding("second")).expect("second save");
        assert_eq!(load(&path, &key).expect("load").items[0].secret, "second");
        assert_eq!(
            load(&backup, &key).expect("backup").items[0].secret,
            "first",
            "the vault one save back still opens"
        );
        assert!(
            !folder.path().join("p.aven-vault.new").exists(),
            "the file written beside is moved, not left"
        );
    }
}
