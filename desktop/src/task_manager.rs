//! The task manager: Aven's own footprint, from the resource watcher, and
//! each live tab's share, from the shell. Titles and sites only, never page
//! content or full addresses.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use aven_app::{Command, Handle};
use aven_ipc::{OperationAdmission, TaskManagerView};
use tauri::{Manager, State, WebviewWindow};

// One look at a time: the panel asks every couple of seconds, and a slow
// answer must not pile up more behind it.
static SNAPSHOT_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

fn admitted(caller: &WebviewWindow, command: &str) -> bool {
    super::authorize(caller, super::CallerPolicy::Main, command)
        && !super::shutdown_started(caller.app_handle())
}

/// Aven's processes as the resource watcher last measured them.
#[cfg(target_os = "windows")]
fn totals() -> (Option<u32>, Option<f32>, u32, bool, bool) {
    match super::game_mode::latest() {
        Some(totals) => (
            Some(u32::try_from(totals.memory_mb).unwrap_or(u32::MAX)),
            Some(totals.cpu_percent as f32),
            u32::try_from(totals.processes).unwrap_or(u32::MAX),
            totals.game,
            totals.throttled,
        ),
        None => (None, None, 0, false, false),
    }
}

#[cfg(not(target_os = "windows"))]
fn totals() -> (Option<u32>, Option<f32>, u32, bool, bool) {
    (None, None, 0, false, false)
}

/// What the task manager shows now. `None` when the shell could not answer
/// in time; the panel keeps its last look and asks again.
#[tauri::command]
#[specta::specta]
pub(crate) async fn task_manager_snapshot(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
) -> Result<Option<TaskManagerView>, ()> {
    if !admitted(&caller, "task_manager_snapshot") {
        return Ok(None);
    }
    if SNAPSHOT_IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(None);
    }
    let _guard = super::AtomicFlagReset(&SNAPSHOT_IN_FLIGHT);
    let receiver = shell.task_snapshot();
    let Some((tabs, sleeping)) =
        tokio::task::spawn_blocking(move || receiver.recv_timeout(Duration::from_secs(2)))
            .await
            .ok()
            .and_then(Result::ok)
    else {
        return Ok(None);
    };
    let (memory_mb, cpu_percent, processes, game, throttled) = totals();
    Ok(Some(TaskManagerView {
        memory_mb,
        cpu_percent,
        processes,
        game,
        throttled,
        tabs,
        sleeping,
    }))
}

/// Puts a hidden tab to sleep. The page's own safety check still decides,
/// as for any other sleep, and a page on screen never sleeps.
#[tauri::command]
#[specta::specta]
pub(crate) fn task_manager_sleep(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    id: String,
) -> OperationAdmission {
    if !admitted(&caller, "task_manager_sleep") || !super::bounded(&id, super::MAX_ITEM_ID_BYTES) {
        return super::rejected_operation();
    }
    let Some(id) = aven_core::ids::ItemId::parse(&id) else {
        return super::rejected_operation();
    };
    // A plain command, not a tracked operation: nothing settles here. The
    // panel reads the outcome from its next look, where the tab has left the
    // list or is still in it.
    if shell.dispatch(Command::SleepTab { id }) {
        super::accepted_ui_operation()
    } else {
        super::rejected_operation()
    }
}
