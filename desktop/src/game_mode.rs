//! Game mode and resource caps, on Windows.
//!
//! Every few seconds Aven looks at what runs in front of the person and at
//! what Aven itself uses (its own process and every WebView2 process under
//! it). While a game runs fullscreen, or while Aven is over the memory or
//! processor cap the person chose, Aven steps back:
//! - its hidden pages are put to sleep, through the same memory-pressure
//!   path Windows' own low-memory signal takes;
//! - its processes run at below-normal priority with Windows' efficiency mode
//!   (EcoQoS), so the game gets the processor first;
//! - the interface stops its animations.
//!
//! Everything returns to normal on its own once the game closes. Nothing here
//! ends a process or touches any other application.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

use aven_app::{Command, Handle};
use aven_core::ports::engine::MemoryPressure;
use aven_core::ports::store::Store;
use tauri::Manager;

const POLL: Duration = Duration::from_secs(3);
/// Polls in a row over or under the processor cap before Aven steps back or
/// comes back, so a short burst (opening a page) does not flip it.
const CPU_STREAK: u32 = 2;

pub(crate) const GAME_MODE: &str = "performance.game-mode";
pub(crate) const MEMORY_CAP: &str = "performance.memory-cap";
pub(crate) const CPU_CAP: &str = "performance.cpu-cap";

/// The system's memory state and what Aven asks for itself, as levels:
/// 0 normal, 1 warning, 2 critical. The shell gets the stronger of the two.
static SYSTEM: AtomicU8 = AtomicU8::new(0);
static OWN: AtomicU8 = AtomicU8::new(0);
static SENT: AtomicU8 = AtomicU8::new(0);

fn level(pressure: MemoryPressure) -> u8 {
    match pressure {
        MemoryPressure::Normal => 0,
        MemoryPressure::Warning => 1,
        MemoryPressure::Critical => 2,
    }
}

fn pressure(level: u8) -> MemoryPressure {
    match level {
        0 => MemoryPressure::Normal,
        1 => MemoryPressure::Warning,
        _ => MemoryPressure::Critical,
    }
}

/// Windows' own memory state, from the memory-pressure monitor.
pub(crate) fn set_system_pressure(shell: &Handle, tier: MemoryPressure) {
    SYSTEM.store(level(tier), Ordering::Release);
    publish(shell);
}

fn set_own_pressure(shell: &Handle, wanted: u8) {
    OWN.store(wanted, Ordering::Release);
    publish(shell);
}

fn publish(shell: &Handle) {
    let wanted = SYSTEM
        .load(Ordering::Acquire)
        .max(OWN.load(Ordering::Acquire));
    if SENT.swap(wanted, Ordering::AcqRel) != wanted {
        let _ = shell.dispatch(Command::SetMemoryPressure(pressure(wanted)));
    }
}

/// What the person chose in Settings › Performance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Choices {
    game_mode: bool,
    memory_cap_mb: u64,
    cpu_cap_percent: u32,
}

fn choices() -> Choices {
    let read = |key: &str| {
        super::APP_STORE
            .get()
            .and_then(|store| store.app_setting(key))
    };
    Choices {
        game_mode: read(GAME_MODE).is_none_or(|value| value != "false"),
        memory_cap_mb: read(MEMORY_CAP)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        cpu_cap_percent: read(CPU_CAP)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
    }
}

/// The memory level the cap asks for, with some room on each side so it
/// does not flicker around the line.
fn cap_level(used_mb: u64, cap_mb: u64, previous: u8) -> u8 {
    if cap_mb == 0 {
        return 0;
    }
    if used_mb * 100 > cap_mb * 115 {
        2
    } else if used_mb > cap_mb {
        1
    } else if used_mb * 100 < cap_mb * 90 {
        0
    } else {
        previous.min(1)
    }
}

/// Starts the watch. Windows only; elsewhere there is nothing to do.
pub(crate) fn install(app: &tauri::AppHandle) {
    let Some(shell) = app.try_state::<Handle>().map(|state| state.inner().clone()) else {
        return;
    };
    let app = app.clone();
    let _ = std::thread::Builder::new()
        .name("game-mode".into())
        .spawn(move || watch(app, shell));
}

/// Aven's processes as last measured, for the task manager.
#[derive(Clone, Copy)]
pub(crate) struct Totals {
    /// What Aven's processes hold in memory that is their own: the figure
    /// Windows' Task Manager adds up, so the two agree.
    pub memory_mb: u64,
    pub cpu_percent: f64,
    pub processes: usize,
    pub game: bool,
    pub throttled: bool,
}

static LATEST: std::sync::Mutex<Option<Totals>> = std::sync::Mutex::new(None);

pub(crate) fn latest() -> Option<Totals> {
    *LATEST
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn watch(app: tauri::AppHandle, shell: Handle) {
    let own = std::process::id();
    let processors = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
    let mut cpu = HashMap::<u32, u64>::new();
    let mut last_poll = Instant::now();
    let mut cap = 0u8;
    let mut over = 0u32;
    let mut under = 0u32;
    let mut cpu_throttled = false;
    let mut throttled = HashSet::<u32>::new();
    let mut throttling = false;
    let mut shown: Option<(u64, bool, bool)> = None;
    let mut told_gaming = false;
    loop {
        std::thread::sleep(POLL);
        let (tree, capturing) = native::process_tree(own);
        // Streamer mode follows a recording or streaming program.
        crate::streamer::set_capturing(&app, capturing);
        let wanted = choices();
        let (used_mb, times) = native::measure(&tree);
        // Processor use since the last poll, as a share of the whole machine.
        let elapsed = last_poll.elapsed().as_secs_f64().max(0.001);
        last_poll = Instant::now();
        let busy: u64 = times
            .iter()
            .filter_map(|(pid, time)| cpu.get(pid).map(|before| time.saturating_sub(*before)))
            .sum();
        cpu = times;
        // FILETIME counts 100 ns steps.
        let cpu_percent = (busy as f64 / 1e7) / (elapsed * processors) * 100.0;
        if wanted.cpu_cap_percent > 0 {
            if cpu_percent > f64::from(wanted.cpu_cap_percent) {
                over += 1;
                under = 0;
            } else if cpu_percent < f64::from(wanted.cpu_cap_percent) * 0.8 {
                under += 1;
                over = 0;
            }
            if over >= CPU_STREAK {
                cpu_throttled = true;
            } else if under >= CPU_STREAK {
                cpu_throttled = false;
            }
        } else {
            cpu_throttled = false;
        }

        let gaming = wanted.game_mode && native::game_in_front(&tree);
        cap = cap_level(used_mb, wanted.memory_cap_mb, cap);
        set_own_pressure(&shell, if gaming { 2 } else { cap });
        // The shell holds pages to the network limit chosen for games.
        if gaming != told_gaming && shell.dispatch(Command::SetGameInFront(gaming)) {
            told_gaming = gaming;
        }

        let throttle = gaming || cpu_throttled;
        if throttle != throttling {
            throttling = throttle;
            if !throttle {
                for pid in throttled.drain() {
                    native::set_efficiency(pid, false);
                }
            }
        }
        if throttle {
            // New processes (a page opened) join the others.
            for pid in &tree {
                if throttled.insert(*pid) {
                    native::set_efficiency(*pid, true);
                }
            }
            throttled.retain(|pid| tree.contains(pid));
        }

        *LATEST
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Totals {
            memory_mb: used_mb,
            cpu_percent,
            processes: tree.len(),
            game: gaming,
            throttled: throttle,
        });

        // The meter prints tens of MB below 1 GB and tenths of a GB above:
        // the interface is told when what it shows changes, not on every
        // step it would round away. Efficiency mode alone used to go untold.
        let step = if used_mb >= 1024 { 100 } else { 10 };
        let now = (used_mb / step * step, gaming, throttle);
        if shown != Some(now) {
            shown = Some(now);
            if let Some(window) = app.get_webview_window(super::MAIN_LABEL) {
                let _ = window.eval(format!(
                    "window.dispatchEvent(new CustomEvent('aven:resources',{{detail:{{memory:{},game:{},throttled:{}}}}}))",
                    now.0, gaming, throttle
                ));
            }
        }
    }
}

mod native {
    use std::collections::{HashMap, HashSet};

    use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
        PROCESS_MEMORY_COUNTERS_EX2,
    };
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, ProcessPowerThrottling, SetPriorityClass,
        SetProcessInformation, BELOW_NORMAL_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
        PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        PROCESS_POWER_THROTTLING_STATE, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION,
    };
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowLongW, GetWindowRect,
        GetWindowThreadProcessId, IsZoomed, GWL_STYLE, WS_CAPTION,
    };

    /// Owns a process handle for the length of one call.
    struct Process(HANDLE);

    impl Drop for Process {
        fn drop(&mut self) {
            // SAFETY: the handle came from OpenProcess and is closed once.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    fn open(
        pid: u32,
        access: windows::Win32::System::Threading::PROCESS_ACCESS_RIGHTS,
    ) -> Option<Process> {
        // SAFETY: plain query; a refused or exited process yields an error.
        unsafe { OpenProcess(access, false, pid) }.ok().map(Process)
    }

    /// Aven's own process and every process under it (WebView2's browser,
    /// renderer, GPU and utility processes), and whether a program that
    /// records or broadcasts the screen runs: one walk of the system's
    /// processes for both. Of other programs only file names are looked at.
    pub(super) fn process_tree(root: u32) -> (HashSet<u32>, bool) {
        let mut children = HashMap::<u32, Vec<u32>>::new();
        let mut capturing = false;
        // SAFETY: a snapshot handle owned here and closed below.
        if let Ok(snapshot) = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
            let snapshot = Process(snapshot);
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            // SAFETY: entry is sized and owned for the walk.
            let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
            while more {
                children
                    .entry(entry.th32ParentProcessID)
                    .or_default()
                    .push(entry.th32ProcessID);
                if !capturing {
                    let length = entry
                        .szExeFile
                        .iter()
                        .position(|unit| *unit == 0)
                        .unwrap_or(entry.szExeFile.len());
                    capturing = crate::streamer::is_capture_program(&String::from_utf16_lossy(
                        &entry.szExeFile[..length],
                    ));
                }
                // SAFETY: as above.
                more = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
            }
        }
        let mut tree = HashSet::from([root]);
        let mut queue = vec![root];
        while let Some(pid) = queue.pop() {
            let Some(born) = created(pid) else {
                continue;
            };
            for child in children.get(&pid).into_iter().flatten() {
                // Windows reuses process ids: an orphan whose long-gone
                // parent had the id one of ours has now is not ours, and
                // must not be slowed or counted. A real child is never older
                // than its parent. A recycled id cannot loop back either.
                if *child != 0
                    && !tree.contains(child)
                    && created(*child).is_some_and(|child_born| child_born >= born)
                {
                    tree.insert(*child);
                    queue.push(*child);
                }
            }
        }
        (tree, capturing)
    }

    /// When the process started, in FILETIME ticks; none for one that is
    /// gone or cannot be asked.
    fn created(pid: u32) -> Option<u64> {
        let process = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
        let mut created = FILETIME::default();
        let mut exited = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        // SAFETY: four FILETIMEs owned here.
        unsafe { GetProcessTimes(process.0, &mut created, &mut exited, &mut kernel, &mut user) }
            .ok()?;
        Some(filetime(created))
    }

    fn filetime(time: FILETIME) -> u64 {
        (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
    }

    /// What the process holds in memory that is its own, in bytes: its
    /// private working set, the figure Windows' Task Manager shows. None on
    /// a Windows too old to say.
    fn private_in_memory(process: &Process) -> Option<u64> {
        let mut counters = PROCESS_MEMORY_COUNTERS_EX2 {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32,
            ..Default::default()
        };
        // SAFETY: the counters are sized for the EX2 form and owned here.
        unsafe {
            GetProcessMemoryInfo(
                process.0,
                (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX2)
                    .cast::<PROCESS_MEMORY_COUNTERS>(),
                counters.cb,
            )
        }
        .ok()?;
        Some(counters.PrivateWorkingSetSize as u64)
    }

    /// Whether this Windows gives the private working set, for the tests.
    #[cfg(test)]
    pub(super) fn tells_private_memory() -> bool {
        open(std::process::id(), PROCESS_QUERY_LIMITED_INFORMATION)
            .is_some_and(|process| private_in_memory(&process).is_some())
    }

    /// What the process has asked the system to keep for it, in bytes, in
    /// memory or not: a graphics process asks for several times what it
    /// ever touches, so this overstates what Aven uses.
    fn private_commit(process: &Process) -> Option<u64> {
        let mut counters = PROCESS_MEMORY_COUNTERS_EX {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
            ..Default::default()
        };
        // SAFETY: the counters are sized for the EX form and owned here.
        unsafe {
            GetProcessMemoryInfo(
                process.0,
                (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX)
                    .cast::<PROCESS_MEMORY_COUNTERS>(),
                counters.cb,
            )
        }
        .ok()?;
        Some(counters.PrivateUsage as u64)
    }

    /// What the tree holds in memory in MB, and each process's processor
    /// time. The memory is what Task Manager adds up for the same processes;
    /// where Windows cannot give that figure, what they asked for stands in.
    pub(super) fn measure(tree: &HashSet<u32>) -> (u64, HashMap<u32, u64>) {
        let mut bytes = 0u64;
        let mut times = HashMap::new();
        for pid in tree {
            let Some(process) = open(*pid, PROCESS_QUERY_LIMITED_INFORMATION) else {
                continue;
            };
            bytes += private_in_memory(&process)
                .or_else(|| private_commit(&process))
                .unwrap_or(0);
            let mut created = FILETIME::default();
            let mut exited = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            // SAFETY: four FILETIMEs owned here.
            if unsafe {
                GetProcessTimes(process.0, &mut created, &mut exited, &mut kernel, &mut user)
            }
            .is_ok()
            {
                times.insert(*pid, filetime(kernel) + filetime(user));
            }
        }
        (bytes / (1024 * 1024), times)
    }

    /// Whether another application fills the screen in front: Windows' own
    /// answer (an exclusive fullscreen game, or a full-screen app), or a
    /// borderless window as large as its monitor. Aven itself never counts,
    /// so its own fullscreen video or F11 does not put it to sleep.
    pub(super) fn game_in_front(tree: &HashSet<u32>) -> bool {
        // SAFETY: plain window queries on the current foreground window.
        unsafe {
            let window = GetForegroundWindow();
            // Aven's own window in front (its fullscreen video, F11) or the
            // desktop never count.
            if !window.0.is_null() {
                let mut pid = 0u32;
                GetWindowThreadProcessId(window, Some(&mut pid));
                if tree.contains(&pid) || is_desktop(window) {
                    return false;
                }
            }
            // Windows' own answer holds even when the window in front is not
            // visible from here, as for some exclusive fullscreen games.
            if let Ok(state) = SHQueryUserNotificationState() {
                if state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_BUSY {
                    return true;
                }
            }
            if window.0.is_null() {
                return false;
            }
            // A maximized window, or one with a title bar, is an application
            // at work: with the taskbar set to hide, it covers the screen
            // too. A borderless game has neither.
            let style = GetWindowLongW(window, GWL_STYLE) as u32;
            if IsZoomed(window).as_bool() || style & WS_CAPTION.0 == WS_CAPTION.0 {
                return false;
            }
            let mut rect = RECT::default();
            if GetWindowRect(window, &mut rect).is_err() {
                return false;
            }
            let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(monitor, &mut info).as_bool() {
                return false;
            }
            let screen = info.rcMonitor;
            rect.left <= screen.left
                && rect.top <= screen.top
                && rect.right >= screen.right
                && rect.bottom >= screen.bottom
        }
    }

    /// The desktop and the taskbar fill the screen too; they are not games.
    unsafe fn is_desktop(window: HWND) -> bool {
        let mut name = [0u16; 64];
        // SAFETY: the buffer is owned and sized for the call.
        let length = unsafe { GetClassNameW(window, &mut name) } as usize;
        let class = String::from_utf16_lossy(&name[..length.min(name.len())]);
        matches!(
            class.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        )
    }

    /// Below-normal priority and Windows' efficiency mode, or back to normal.
    pub(super) fn set_efficiency(pid: u32, on: bool) {
        let Some(process) = open(pid, PROCESS_SET_INFORMATION) else {
            return;
        };
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: if on {
                PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            } else {
                0
            },
            StateMask: if on {
                PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            } else {
                0
            },
        };
        // SAFETY: the state is a valid, owned PROCESS_POWER_THROTTLING_STATE.
        unsafe {
            let _ = SetPriorityClass(
                process.0,
                if on {
                    BELOW_NORMAL_PRIORITY_CLASS
                } else {
                    NORMAL_PRIORITY_CLASS
                },
            );
            let _ = SetProcessInformation(
                process.0,
                ProcessPowerThrottling,
                (&state as *const PROCESS_POWER_THROTTLING_STATE).cast(),
                std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memory_figure_counts_what_is_held_not_what_is_asked_for() {
        const MB: usize = 1024 * 1024;
        let own = std::collections::HashSet::from([std::process::id()]);
        let held = || native::measure(&own).0 as i64;
        if !native::tells_private_memory() {
            // A Windows too old for the figure: what was asked for stands in.
            assert!(held() > 0);
            return;
        }
        let before = held();
        assert!(before > 0, "this process holds memory of its own");
        // Asked for and never touched: Windows keeps no memory behind it.
        let mut asked = Vec::<u8>::with_capacity(256 * MB);
        assert!(
            held() - before < 64,
            "256 MB asked for and untouched must not count as used"
        );
        // Touched, one byte a page: now it is held.
        // SAFETY: inside the capacity just reserved; every byte read below
        // was written here first.
        unsafe { asked.set_len(96 * MB) };
        for page in asked.chunks_mut(4096) {
            page[0] = 1;
        }
        let touched = held() - before;
        assert!(
            touched >= 64,
            "96 MB touched must count as used, counted {touched} MB"
        );
        assert_eq!(std::hint::black_box(&asked)[4096], 1);
    }

    #[test]
    fn the_memory_cap_holds_its_level_inside_the_margin() {
        assert_eq!(cap_level(500, 0, 2), 0, "no cap, no pressure");
        assert_eq!(cap_level(800, 1000, 0), 0);
        assert_eq!(cap_level(1050, 1000, 0), 1, "over the cap");
        assert_eq!(cap_level(1200, 1000, 0), 2, "well over the cap");
        assert_eq!(cap_level(950, 1000, 1), 1, "inside the margin, held");
        assert_eq!(cap_level(950, 1000, 0), 0, "inside the margin, held");
        assert_eq!(cap_level(850, 1000, 2), 0, "under the margin, released");
    }

    #[test]
    fn pressure_levels_round_trip() {
        for tier in [
            MemoryPressure::Normal,
            MemoryPressure::Warning,
            MemoryPressure::Critical,
        ] {
            assert_eq!(pressure(level(tier)), tier);
        }
    }
}
