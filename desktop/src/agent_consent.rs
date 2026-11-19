//! The person's say on what an agent outside Aven does to a page.
//!
//! An agent that connects to the local server may read on its own. To press
//! or type it needs a yes, and the yes is given in the browser's own
//! interface: never by the page, and never by the agent. A yes covers one
//! action, or the site in that tab until the person stops the agent or the
//! browser closes. Anything that sends, buys, deletes or saves is asked
//! every time, whatever was agreed before.
//!
//! Nothing here is kept on disk, and what an action is called is page text:
//! it is shown as text and never logged.

use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewWindow};

use crate::{authorize, CallerPolicy};

/// How long the person has to answer before the action is refused.
const ANSWER_WITHIN: Duration = Duration::from_secs(60);
/// Requests waiting at once; one more is refused unasked.
const MAX_WAITING: usize = 4;
/// Standing agreements kept; the oldest goes first.
const MAX_GRANTS: usize = 64;
const MAX_ACTION_CHARS: usize = 160;

/// What the interface shows the person.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, specta::Type)]
pub(crate) struct AgentConsentRequestV1 {
    id: u32,
    /// The site the action is on.
    site: String,
    /// What would be done, such as `Click "Send"`.
    action: String,
    /// It sends, buys, deletes or saves: a yes covers this one action only.
    weighty: bool,
}

/// The person's answer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentConsentAnswerV1 {
    /// This action.
    Once,
    /// This site in this tab, until stopped.
    Site,
    /// No.
    Refuse,
}

struct Waiting {
    request: AgentConsentRequestV1,
    answer: tokio::sync::oneshot::Sender<AgentConsentAnswerV1>,
}

struct State {
    next: u32,
    waiting: Vec<Waiting>,
    /// The tab and site of each standing agreement.
    granted: Vec<(String, String)>,
}

impl State {
    /// Whether a standing agreement already covers this action.
    fn covered(&self, tab: &str, site: &str, weighty: bool) -> bool {
        !weighty
            && self
                .granted
                .iter()
                .any(|(granted_tab, granted_site)| granted_tab == tab && granted_site == site)
    }

    /// What an answer allows, remembering a yes for the site.
    fn settle(
        &mut self,
        tab: &str,
        site: &str,
        weighty: bool,
        answer: AgentConsentAnswerV1,
    ) -> bool {
        match answer {
            AgentConsentAnswerV1::Refuse => false,
            AgentConsentAnswerV1::Once => true,
            AgentConsentAnswerV1::Site => {
                if !weighty && !self.covered(tab, site, false) {
                    if self.granted.len() >= MAX_GRANTS {
                        self.granted.remove(0);
                    }
                    self.granted.push((tab.to_owned(), site.to_owned()));
                }
                true
            }
        }
    }

    /// Takes back every agreement and refuses what is waiting.
    fn stop(&mut self) {
        self.granted.clear();
        for waiting in self.waiting.drain(..) {
            let _ = waiting.answer.send(AgentConsentAnswerV1::Refuse);
        }
    }
}

static STATE: Mutex<State> = Mutex::new(State {
    next: 0,
    waiting: Vec::new(),
    granted: Vec::new(),
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Raises `name` in the interface with `detail`, which is data and is
/// written as JSON: never as code.
fn raise(app: &tauri::AppHandle, name: &str, detail: &impl Serialize) {
    let Ok(detail) = serde_json::to_string(detail) else {
        return;
    };
    if let Some(window) = app.get_webview_window(crate::MAIN_LABEL) {
        let _ = window.eval(format!(
            "window.dispatchEvent(new CustomEvent('{name}',{{detail:{detail}}}))"
        ));
    }
}

/// Tells the interface which request waits for an answer, or that none does.
fn announce(app: &tauri::AppHandle, state: &State) {
    raise(
        app,
        "aven:agent-consent",
        &state.waiting.first().map(|waiting| &waiting.request),
    );
}

/// Tells the interface an outside agent is acting on `site`, or stopped.
pub(crate) fn working(app: &tauri::AppHandle, site: Option<&str>) {
    raise(app, "aven:agent-working", &site);
}

/// Whether the person lets an outside agent do `action` on `site` in `tab`.
/// They are asked unless a standing agreement covers it; no answer in time
/// is a no.
pub(crate) async fn allowed(
    app: &tauri::AppHandle,
    tab: &str,
    site: &str,
    action: &str,
    weighty: bool,
) -> bool {
    let (id, receiver) = {
        let mut state = state();
        if state.covered(tab, site, weighty) {
            return true;
        }
        if state.waiting.len() >= MAX_WAITING {
            return false;
        }
        state.next = state.next.wrapping_add(1).max(1);
        let id = state.next;
        let (answer, receiver) = tokio::sync::oneshot::channel();
        state.waiting.push(Waiting {
            request: AgentConsentRequestV1 {
                id,
                site: site.to_owned(),
                action: action.chars().take(MAX_ACTION_CHARS).collect(),
                weighty,
            },
            answer,
        });
        announce(app, &state);
        (id, receiver)
    };
    let answer = tokio::time::timeout(ANSWER_WITHIN, receiver)
        .await
        .ok()
        .and_then(Result::ok);
    let mut state = state();
    // Unanswered: withdrawn, so the card does not outlive the request.
    let before = state.waiting.len();
    state.waiting.retain(|waiting| waiting.request.id != id);
    if state.waiting.len() != before {
        announce(app, &state);
    }
    answer.is_some_and(|answer| state.settle(tab, site, weighty, answer))
}

/// The person's answer to the request `id`.
#[tauri::command]
#[specta::specta]
pub(crate) fn agent_consent_answer(
    caller: WebviewWindow,
    id: u32,
    answer: AgentConsentAnswerV1,
) -> bool {
    if !authorize(&caller, CallerPolicy::Main, "agent_consent_answer") {
        return false;
    }
    let mut state = state();
    let Some(index) = state
        .waiting
        .iter()
        .position(|waiting| waiting.request.id == id)
    else {
        return false;
    };
    let waiting = state.waiting.remove(index);
    announce(caller.app_handle(), &state);
    waiting.answer.send(answer).is_ok()
}

/// The request waiting for an answer, for an interface that came up after
/// it was raised.
#[tauri::command]
#[specta::specta]
pub(crate) fn agent_consent_pending(caller: WebviewWindow) -> Option<AgentConsentRequestV1> {
    if !authorize(&caller, CallerPolicy::Main, "agent_consent_pending") {
        return None;
    }
    state()
        .waiting
        .first()
        .map(|waiting| waiting.request.clone())
}

/// Takes back every standing agreement and refuses what is waiting: the
/// person stopped the agent, or turned off the server it came through.
pub(crate) fn stop_all(app: &tauri::AppHandle) {
    let mut state = state();
    state.stop();
    announce(app, &state);
    working(app, None);
}

/// The person's stop: every standing agreement is taken back and what is
/// waiting is refused.
#[tauri::command]
#[specta::specta]
pub(crate) fn agent_consent_stop(caller: WebviewWindow) -> bool {
    if !authorize(&caller, CallerPolicy::Main, "agent_consent_stop") {
        return false;
    }
    stop_all(caller.app_handle());
    true
}

/// The words shown on a page while an agent acts on it, in the person's
/// language: the interface has the translations, native draws them.
#[tauri::command]
#[specta::specta]
pub(crate) fn agent_caption_set(caller: WebviewWindow, caption: String) -> bool {
    if !authorize(&caller, CallerPolicy::Main, "agent_caption_set") || caption.len() > 400 {
        return false;
    }
    aven_engine::page_reader::set_working_caption(&caption);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> State {
        State {
            next: 0,
            waiting: Vec::new(),
            granted: Vec::new(),
        }
    }

    #[test]
    fn a_yes_for_the_site_covers_that_site_in_that_tab_only() {
        let mut state = empty();
        assert!(!state.covered("tab-1", "example.com", false));
        assert!(state.settle("tab-1", "example.com", false, AgentConsentAnswerV1::Site));
        assert!(state.covered("tab-1", "example.com", false));
        assert!(!state.covered("tab-2", "example.com", false));
        assert!(!state.covered("tab-1", "other.example", false));
    }

    #[test]
    fn once_and_refuse_leave_nothing_standing() {
        let mut state = empty();
        assert!(state.settle("tab-1", "example.com", false, AgentConsentAnswerV1::Once));
        assert!(!state.settle("tab-1", "example.com", false, AgentConsentAnswerV1::Refuse));
        assert!(!state.covered("tab-1", "example.com", false));
    }

    #[test]
    fn what_sends_or_buys_is_asked_every_time() {
        let mut state = empty();
        assert!(state.settle("tab-1", "example.com", false, AgentConsentAnswerV1::Site));
        assert!(!state.covered("tab-1", "example.com", true));
        // A yes for the site given on such an action allows it, and no more.
        let mut fresh = empty();
        assert!(fresh.settle("tab-1", "example.com", true, AgentConsentAnswerV1::Site));
        assert!(!fresh.covered("tab-1", "example.com", false));
    }

    #[test]
    fn stopping_takes_back_agreements_and_refuses_what_waits() {
        let mut state = empty();
        state.settle("tab-1", "example.com", false, AgentConsentAnswerV1::Site);
        let (answer, mut receiver) = tokio::sync::oneshot::channel();
        state.waiting.push(Waiting {
            request: AgentConsentRequestV1 {
                id: 1,
                site: "example.com".into(),
                action: "Click".into(),
                weighty: false,
            },
            answer,
        });
        state.stop();
        assert!(!state.covered("tab-1", "example.com", false));
        assert!(state.waiting.is_empty());
        assert_eq!(receiver.try_recv(), Ok(AgentConsentAnswerV1::Refuse));
    }

    #[test]
    fn standing_agreements_are_bounded() {
        let mut state = empty();
        for n in 0..MAX_GRANTS + 5 {
            state.settle(
                &format!("tab-{n}"),
                "example.com",
                false,
                AgentConsentAnswerV1::Site,
            );
        }
        assert_eq!(state.granted.len(), MAX_GRANTS);
        assert!(!state.covered("tab-0", "example.com", false));
    }
}
