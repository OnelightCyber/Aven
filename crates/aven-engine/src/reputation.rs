//! One choice for every Windows WebView2 controller of a launch. WebView2
//! joins per-controller requirements across a user-data folder, so the choice
//! is fixed by the first controller made: until then (onboarding, at first
//! run) it can still change. Its default is ON; only an explicit OFF choice
//! needs the optional Settings8 API.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

struct Choice {
    required: bool,
    /// A controller took the choice: it holds for the rest of the launch.
    fixed: bool,
}

struct Policy {
    choice: Mutex<Choice>,
    managed: AtomicBool,
}
impl Policy {
    fn new(required: bool) -> Self {
        Self {
            choice: Mutex::new(Choice {
                required,
                fixed: false,
            }),
            managed: AtomicBool::new(false),
        }
    }
    fn choice(&self) -> std::sync::MutexGuard<'_, Choice> {
        self.choice
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
    /// Changes the choice while no controller has taken it. False once one
    /// has: the person's new choice then waits for the next launch.
    fn choose(&self, required: bool) -> bool {
        let mut choice = self.choice();
        if choice.fixed {
            return false;
        }
        choice.required = required;
        true
    }
    /// The choice a controller is made with, fixed from now on.
    fn take(&self) -> bool {
        let mut choice = self.choice();
        choice.fixed = true;
        choice.required
    }
    fn apply<E: std::fmt::Display>(&self, disable: impl FnOnce() -> Result<(), E>) -> bool {
        if self.take() || self.managed.load(Ordering::Acquire) {
            return false;
        }
        if let Err(error) = disable() {
            eprintln!("Windows reputation checking is managed; OFF could not be applied: {error}");
            return !self.managed.swap(true, Ordering::AcqRel);
        }
        false
    }
}

#[cfg(target_os = "windows")]
static POLICY: std::sync::OnceLock<Policy> = std::sync::OnceLock::new();
#[cfg(target_os = "windows")]
static MANAGED: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();

/// Install before constructing Windows WebViews. A later edit applies at once
/// while no controller exists yet, else at the next launch.
#[cfg(target_os = "windows")]
pub fn initialize(required: bool) -> bool {
    POLICY.set(Policy::new(required)).is_ok()
}

/// The person's choice, made before any page exists: the first page takes it.
/// False when a page was made already, so it waits for the next launch.
#[cfg(target_os = "windows")]
pub(crate) fn choose_before_first_view(required: bool) -> bool {
    POLICY.get().is_some_and(|policy| policy.choose(required))
}

#[cfg(target_os = "windows")]
pub fn on_managed(handler: impl Fn() + Send + Sync + 'static) {
    let _ = MANAGED.set(Box::new(handler));
}

#[cfg(target_os = "windows")]
pub fn managed() -> bool {
    POLICY
        .get()
        .is_some_and(|policy| policy.managed.load(Ordering::Acquire))
}

#[cfg(target_os = "windows")]
pub fn configure(
    core: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
) -> windows_core::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings8;
    use windows_core::Interface;
    let policy = POLICY.get_or_init(|| Policy::new(true));
    if policy.apply(|| -> windows_core::Result<()> {
        let settings = unsafe { core.Settings()? }.cast::<ICoreWebView2Settings8>()?;
        unsafe { settings.SetIsReputationCheckingRequired(false)? };
        let mut actual = windows_core::BOOL::default();
        unsafe { settings.IsReputationCheckingRequired(&mut actual)? };
        if actual.as_bool() {
            return Err(windows::Win32::Foundation::E_ACCESSDENIED.into());
        }
        Ok(())
    }) {
        if let Some(handler) = MANAGED.get() {
            handler();
        }
    }
    // Missing Settings8 and policy refusal are managed states, never view failures.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_on_never_queries_optional_settings() {
        let policy = Policy::new(true);
        assert!(!policy.apply(|| -> Result<(), &str> { panic!("default ON queried Settings8") }));
        assert!(!policy.managed.load(Ordering::Acquire));
    }
    #[test]
    fn off_failure_projects_managed_and_does_not_fail_view_creation() {
        let policy = Policy::new(false);
        assert!(policy.apply(|| Err("Settings8 unavailable or enterprise policy refused OFF")));
        assert!(policy.managed.load(Ordering::Acquire));
        assert!(!policy.apply(|| -> Result<(), &str> { panic!("managed policy was retried") }));
    }
    #[test]
    fn a_choice_before_the_first_controller_is_the_one_it_takes() {
        let policy = Policy::new(true);
        assert!(policy.choose(false), "nothing has taken the choice yet");
        // The first controller is made with OFF and asks for it.
        let mut asked = false;
        assert!(!policy.apply(|| {
            asked = true;
            Ok::<(), &str>(())
        }));
        assert!(asked);
        // From then on the choice holds for the launch, either way.
        assert!(!policy.choose(true));
        assert!(!policy.apply(|| {
            asked = false;
            Ok::<(), &str>(())
        }));
        assert!(!asked, "still OFF, asked again for each controller");
    }
    #[test]
    fn a_choice_after_the_first_controller_waits_for_the_next_launch() {
        let policy = Policy::new(false);
        assert!(!policy.apply(|| Ok::<(), &str>(())));
        assert!(!policy.choose(true));
        assert!(!policy.take(), "the first controller's OFF holds");
    }
    #[test]
    fn successful_off_preserves_persons_control() {
        let policy = Policy::new(false);
        assert!(!policy.apply(|| Ok::<(), &str>(())));
        assert!(!policy.managed.load(Ordering::Acquire));
    }
}
