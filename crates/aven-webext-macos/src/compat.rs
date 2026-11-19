//! The JavaScript half of the compatibility layer, injected into packages by
//! `aven_webext::prepare`.

pub const SCRIPT: &str = include_str!("compat/compat.js");

/// The Chrome release extensions are told they run in; the Web Store also
/// uses it to pick package versions.
pub use aven_webext::store::CHROME_VERSION;
