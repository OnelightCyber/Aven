# Windows hardening review — October 7, 2026

Branch: `windows/hardening`, based on `origin/main` at `e25c8a86`, including
the public-release trust/privacy merge `3259a03e` (#54). This is an engineering
review in progress, not a production-readiness certification. Everyday browsing,
visual acceptance, video fullscreen, and real workload resource measurements
remain with the owner. No personal browser profile is used by these checks.

Review commits:

| Commit | Change |
| --- | --- |
| `5b61919d` | Preserve runtime ownership across independent data roots. |
| `95f90547` | Bound privileged renderer recovery and Windows initialization; verify native visibility. |
| `3ec48edb` | Validate settled fullscreen exit and use the fallback on script failure. |
| `345a8133` | Bound shared media imports while reading a single file handle. |
| `8c7b12cb` | Correct Windows test fixtures and diagnose native extension delivery stalls. |
| `58c7bba9` | Resolve four development dependency advisories with published fixes. |

## First changes

### October 10: reported download, icon and video failures

Local changes on `windows/hardening`, awaiting owner visual acceptance:

- Reproduced `FILE_FAILED` with a loopback HTTP attachment and the production
  private staging directory. WebView2 accepted the verbatim destination spelling
  but failed the transfer. Verified ordinary Win32 spelling completed the same
  transfer, publication and Internet-zone/file-identity checks. Canonical ancestor
  leases and staging ACLs remain authoritative. Windows now honors automatic save
  to Downloads and the optional ask-first setting; a custom folder still requires
  its native picker identity. Multiple automatic downloads remain denied.
- Fixed favicon delivery ownership when switching profiles. Returning to a profile
  resends unchanged rasters; a late background-profile icon cannot replace the
  foreground webview's cache. Empty profiles explicitly reset the surface owner.
  All 18 favicon tests pass, including both regressions.
- Native generated-video qualification opened and closed video PiP successfully.
  The installed runtime exposes its menu command as `pictureInPciture`; the host
  filter had discarded it. Preserve that command and the observed native media
  controls, retaining foreground/document admission. Document PiP is unavailable
  in this runtime; no emulation or media URL extraction was introduced.
- Element fullscreen now conceals intermediate native geometry with a black owned
  popup, then fades out after geometry settles. Windows animation preferences are
  respected; the cover allocates only the current client area and releases its
  HWND/timer within 600 ms. Native tests cover expiry and replacement. Visual
  smoothness, mixed-DPI monitors and real-site acceptance remain owner checks.

Validation: 446 application tests and 264 engine tests passed (3 opt-in engine
tests ignored by the ordinary suite). The native download/PiP qualifier was run
separately and passed, including dispatch of the actual native PiP menu command
and profile cleanup after browser-process exit. Strict Clippy passed for both
crates and all their targets. Frontend check passed (736 tests in 131 files), and
the frontend build passed its bundle budgets. Existing build warnings about motion
initialization and CSS highlight selectors remain outside these fixes.
These are debug correctness checks, not release performance measurements.

The Review QA identity is also admitted to the closed Windows Work-storage identity
list so the requested isolated app can start. The separate namespace-validation
fixture still fails with `Unsafe` while preparing its original Product fixture,
before reaching the added Review QA assertion; no security predicate was relaxed.

Owner checks for the rebuilt candidate: start fresh downloads from the original
sites; toggle ask-first and choose a folder; verify filename collisions and Open/
Reveal; enter/exit YouTube fullscreen from normal and maximized windows; open PiP
from the native video menu; switch private/regular profiles and revisit favicons.

### October 7: initial hardening

- Runtime-generation maintenance no longer deletes shared volatile registry
  markers just because their generation is absent from the current data root.
  QA, production, and other app identities share the registry namespace. The
  previous prune could erase another identity's same-boot proof and let a later
  launch reclaim its runtime files without process-exit proof. Exact cleanup
  tickets still remove their own markers; otherwise they expire on reboot.
  A regression covers two independent roots and a simulated parent restart for
  both private and privileged runtime kinds. Filesystem formats are unchanged.
- Privileged Windows chrome previously had no `ProcessFailed` observer. The main
  document and launcher now each have two recovery attempts per application run,
  with a 60-second readiness deadline. Recovery reloads the fixed trusted
  document, keeps its existing native security handlers, and waits for readiness.
  Browser-process exit, exhausted attempts, or failed recovery produces a native
  restart explanation and uses orderly terminal shutdown. GPU/utility failures
  and mere unresponsiveness do not trigger destructive reloads. A renderer crash
  can still lose edits that had not reached durable storage.
- Windows startup and onboarding handover allow 60 seconds instead of 15.
  Other platforms keep their existing timeout. This changes failure tolerance,
  not measured startup speed.
- Windows chrome visibility now reports the actual native result. Previously a
  successfully dispatched callback could return success despite a failed
  `SetIsVisible`, leaving onboarding with hidden controls.
- Fullscreen exit now waits for the JavaScript promise and validates the CDP
  result. An HRESULT success carrying `exceptionDetails`, a rejected promise,
  or a false result triggers the existing script fallback. Invalid execution
  context IDs are rejected. Video and multi-monitor acceptance remain separate.
- Local media import opens the selected file once and bounds the read itself.
  A file that grows after the metadata check cannot cause an unlimited read.
  This shared fix applies to Windows and macOS.
- Work fault fixtures wait for asynchronous worker reaping before releasing
  their test serialization lock. Production runtime exclusivity and failure
  outcomes are preserved. Command-policy tests now exercise POSIX and Windows
  explicitly instead of expecting the POSIX allowlist on Windows.
- The native strict-CSP fixture now matches the current discard policy: it
  excludes transient activity/edit grace bits from its durable-state assertions,
  uses native text input to protect an edited closed-shadow form, and exercises
  the current 50,000-element scan limit. No production discard rule was weakened.

Recovery follows [Microsoft's process-failure guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-related-events):
a renderer can be navigated/reloaded, while an exited browser process leaves its
controllers closed. The fullscreen result check follows the
[Runtime.evaluate contract](https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#method-evaluate).

## Initial platform inventory

These are code findings, not claims that every native mechanism or feature has
passed live testing. The remaining work column is deliberately explicit.

| Area | Windows implementation / macOS comparison | Remaining work |
| --- | --- | --- |
| Native chrome | Custom native caption, `HTMAXBUTTON` Snap hit testing, DPI-change handling, and high-contrast menu fallback exist. | Physical DPI, Snap, focus/IME, Narrator and touch acceptance; broader shortcut trace. |
| Lifecycle | Content process-failure handling exists. Hidden tabs receive low-memory hints; sleeping tabs use `TrySuspend`/`Resume`. Native low/high-memory notifications drive the shared policy. Privileged recovery passed two real renderer crashes. | Longer sleep/wake and discarded-tab stress; launcher recovery acceptance. |
| Certificates / runtime | Content certificate errors explicitly cancel. Actual WebView2 version, private mode and UDF identity are checked. Shipping arguments retain SmartScreen. Wry enables custom crash reporting to suppress automatic uploads. | Native failure-path checks and final security review. The owner-controlled vendor release blocker is unchanged. |
| Permissions | Windows camera/microphone use WebView2's prompt; macOS has a browser-owned deferred broker. Other page permissions and client certificates are denied; HTTP sign-in on human tabs uses Aven's own card, and is denied elsewhere. Privileged UI denies ambient capabilities. | Permission UX parity and supported capability decisions; no silent granting of currently denied capabilities. |
| IPC / navigation | Chrome has a navigation lock and caller-scoped commands; content uses separate native views. Privileged subframes and external URI launches have native guards. External-app requests pass through browser approval before `ShellExecuteW`. Windows userscripts remain disabled pending CDP isolation proof. | Complete origin/source and navigation-race review across all bridges; userscript support is a deliberate parity gap. |
| Downloads | Windows uses private staging, handle/identity checks, Attachment Services, and verified Internet-zone `Zone.Identifier` metadata. | Real interrupted/resumed downloads, scanner rejection, filesystem and file-lock edge cases. |
| Private data | Runtime generations and browser-process exit ownership govern UDF cleanup; deleting live locked WebView2 data is avoided. | Crash/relaunch residue checks and broader cookie-isolation qualification. |
| Storage | Root composition still uses `app_data_dir` (Roaming on Windows). Native paths verify canonical identity before projecting Win32/short spelling. | Local AppData migration design; redirected/long-path and SQLite locking stress. |
| Ad blocking | Document-source HTTP(S) filters cover selected resource contexts; reusable bounded request buffers and one blocked response reduce per-request allocation. | End-to-end release A/B numbers. These optimizations do not establish zero overhead on clean pages. |
| Fullscreen / painting | Windows has native fullscreen observation, isolated-world exit, stage placement and paint covers; macOS uses WKWebView-specific transitions. | Owner video/F11/Snap/monitor checks; native fallback result fix covered separately. |
| Updates / distribution | Per-user NSIS, signature re-verification, retained interrupted update, and install after a clean requested relaunch exist. Release workflow stages the NSIS installer/signature, separately retains PDBs, verifies updater signatures, and emits both Windows updater target keys. Automatic install-on-quit is macOS-only. | Interrupted installer/rollback qualification; Windows install-on-quit decision. Signing remains owner-provided SignPath work. |
| Default browser / links | Windows registers per-user HTTP/HTTPS capabilities on request and opens Default Apps; startup link handoffs are bounded and queued until the shell exists. Uninstall removes those registration keys. | `.html`/`.pdf` file associations are absent from this registration; early-startup handoff stress. |
| Logging / OS services | Windows stderr has bounded rotation and retention; Show Logs opens Explorer. `media.rs` manages imported files, not media-key integration. Focus timer OS notifications are macOS-only; the non-macOS branch silently returns. | Log-content privacy review (rotation does not redact URLs/paths), media controls and native notification integration. Page notifications currently remain denied. |
| Work | Shared policy includes approval for typing on sites the person did not name; Windows has native input, screenshot, network and rendering adapters. Frame capture bounds native jobs to eight, image workers to two, and PNG/dimension/output budgets; image decoding is off the STA. | End-to-end Windows enforcement and capture-cost verification beyond shared unit tests. |
| Extensions | Current main includes native action-host crash recovery and popup/worker lifecycle handling, superseding several old handoff findings. All six old cases were traced; one complete native run covered recovery, restored-page closing, bounded worker snapshots, cap/removal, popup cleanup and process restart. | Repeated native runs exposed intermittent missing extension notifications after the forced host crash; see reproducible open issue below. Real-extension compatibility remains separate. |

## Extension follow-up still open

`host::webext_windows::qualification::native_windows_runtime_recovery` is not a
stable passing gate. Three of four observed runs timed out on the injected
action-change notification after host-renderer recovery. The complete passing
run is in `target/windows-extension-native.log`; the final failure, with added
diagnostics, is in `target/windows-extension-native-repeat.log`.

The failed run reaches 20 tab switches with two bounded worker snapshots, then
the extension page's `chrome.runtime.sendMessage` remains pending and the host
observes neither the notification nor a new action snapshot. That is evidence
of a delivery failure in this native fixture; it does not yet establish whether
the cause is runtime recovery, suspended-view behavior, or fixture sequencing.
The added diagnostics preserve the failure instead of silently swallowing it.
Do not treat the single passing run as closure of this issue.

Reproduce on Windows using the workspace build so feature selection matches:

```powershell
$env:CARGO_PROFILE_DEV_DEBUG='0'
$env:CARGO_PROFILE_TEST_DEBUG='0'
$env:CARGO_INCREMENTAL='0'
cargo test --workspace --locked -j 2 --no-run
# Use the aven_engine test executable printed above; run alone:
& <engine-test-executable> native_windows_runtime_recovery --ignored --nocapture --test-threads=1
```

The test creates its own temporary profile and packages; it does not need an
installed extension or the owner's browsing profile. Its passing branches also
verify sleeping-tab style refresh, popup crash closure, one failed native popup
close, disabled-extension removal at capacity, and restoration in a fresh
process. Stable queue order and retry on a freed slot are implemented in
`crates/aven-app/src/shell/webext.rs`; native admission/removal ownership is in
`crates/aven-engine/src/host/webext_windows.rs`.

## Validation record

The approved npm dependency audit found four moderate development-tool
advisories, resolved by pinning `fast-uri` 3.1.8, `brace-expansion` 5.0.12,
`smol-toml` 1.9.0 and `postcss-selector-parser` 7.1.6. The repeated registry
audit reports zero moderate/critical findings and one high finding for
`braces` 3.0.3. That package has no published upstream fix; its existing local
depth-bound patch remains in place. The repository's exact-byte verification
and all four audit regression tests pass, including hostile patterns and ASTs.
This is the existing narrowly verified development-tool exception, not a clean
raw audit or an additional waiver. The original local installation lacked the
patched bytes despite its patch identifier; a forced reinstall repaired it
without changing the verifier or its expected hashes. Rust advisory scanning
was not performed (`cargo-audit` and `cargo-deny` are not installed).

Advisories: [fast-uri](https://github.com/advisories/GHSA-hrr3-gc8f-f4qj),
[brace-expansion](https://github.com/advisories/GHSA-q2hr-2g5m-vwhr),
[smol-toml](https://github.com/advisories/GHSA-r4xh-jqrq-34v2),
[selector parser](https://github.com/advisories/GHSA-rj75-hqrm-r3gf), and
[braces](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm).

Machine: Windows 11 Pro, build 26200; Intel i3-1115G4, two cores/four logical
processors; 12,377,400 KiB visible RAM. Installed WebView2 folders include
154.0.4258.53 and 154.0.4258.62. The isolated chrome recovery test reports the
actually selected runtime as 154.0.4258.62.

Toolchain: Rust 1.95.0; repository-local Node 24.18.0 and pnpm 11.17.0.
Use one `target` directory. Tests use `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0`, and `-j 2` to limit disk/RAM.
These are debug correctness checks, not release performance measurements.

- Frontend `pnpm -C frame run check`: passed; 736 tests in 131 files.
- Frontend `pnpm -C frame run build`: passed, including bundle budgets. Existing
  build warnings concern initial Svelte motion state, CSS highlight parsing, and
  the onboarding wordmark reference.
- Frontend check/build were repeated successfully after the dependency fixes;
  logs are `target/windows-frame-check-dependencies.log` and
  `target/windows-frame-build-dependencies.log`. The approved raw registry
  audit reports are `target/windows-dependency-audit.json` and
  `target/windows-dependency-audit-after.json`.
- Isolated native chrome recovery: passed, including two `Page.crash` events,
  navigation/readiness recovery and exhausted-budget refusal. Log:
  `target/windows-renderer-native.log`.
- Rust workspace: 3,559 passed, 12 opt-in tests ignored; no failures. These
  include the eight Windows runtime-generation ownership/cleanup tests.
- Strict workspace clippy: passed with `--all-targets -- -D warnings`.
- Formatting and diff whitespace: passed.
- Native network protection: passed (100 useful script requests reached the
  fixture, 100 ad requests were blocked, zero matcher/fail-open errors).
- Native strict-CSP protection: passed, including edited closed-shadow forms,
  unload handlers, bounded scans, cosmetic styles, undo and picker lifecycle.
- Extension action host/observer JavaScript tests: 11 passed. Native extension
  qualification remains intermittent as described above.
- Logs: `target/windows-hardening-tests.log`,
  `target/windows-hardening-clippy.log`, `target/windows-runtime-isolation.log`,
  `target/windows-protection-network.log`, `target/windows-protection-csp.log`,
  `target/windows-extension-js.log`.
- No new startup, RAM, CPU, GPU, battery, or ad-blocker speed numbers are claimed.

The native chrome test is opt-in under
`platform::windows::renderer::tests::privileged_renderer_recovers_twice_through_native_process_failed`.
The engine's `native_protection_qualification` test uses
`AVEN_PROTECTION_SITE=fixture` or `csp`, `AVEN_PROTECTION_MODE=on`,
and `--ignored --nocapture --test-threads=1`. All reported native checks here
use debug binaries for correctness. Their incidental timing output must not
be presented as release performance evidence.

## October 18 download handoff follow-up

The owner confirmed ordinary download completion, fullscreen and manual video
PiP, then reported Open/Show in Folder failures and fullscreen reveals before
slow page layouts settled. The two completed QA download receipts still name
existing files, but neither file retains a `Zone.Identifier` stream. This
explains the protection error; the cause of the later stream removal is not
established.

Show in Folder now verifies the recorded file identity without requiring
execution protection. Open still requires protection: for an unchanged original
it reruns Attachment Services when protection is missing, restores the receipt's
last-write timestamp after the ADS-only update, and verifies protection again.
Modified or substituted files remain refused. Explorer and shell verbs receive
the strictly validated ordinary spelling of the canonical local path.

The Windows download suite passes 22 tests (one native opt-in test excluded),
including missing-stream recovery, rejection of edited payloads, and shell PIDL
parsing of Unicode names with spaces. No downloaded executable was run during
qualification. The native loopback download also retains its provenance after
the original WebView2 process exits; that fixture did not reproduce later stream
removal.

The earlier 600 ms fullscreen cover is superseded by readiness sampling. The
cover now waits for native window geometry, a matching page viewport/fullscreen
state, and 200 ms of stable video rectangles, then fades over 180 ms (or reveals
without animation for reduced motion). Enter requires the page to fill the
window and the window to cover its monitor; exit requires restoration of the
saved window rectangle. A four-second failure deadline prevents a hung renderer
from leaving the window black. Competing page slides are suppressed while the
cover is active. Engine tests pass 267 cases with three opt-in tests excluded.
Automatic PiP remains a separate implementation on the integrated branch.

## October 18 integrated branch

The Windows work was committed and pushed independently at `f937e2db` before
merging into `feat/work-production` (`ce746743`). Merge `f1328d0a` preserves the
integrated preview cleanup and clipping alongside fullscreen slide suppression,
and retains both profile handling and Windows chrome-renderer recovery. The
split drop bar and recorded bundle budgets were not replaced or re-recorded.

Automatic video PiP now follows ordinary tab selection through a native engine
port. Windows evaluates the fixed presentation program in a dedicated CDP world;
macOS uses a named public WKContentWorld. Neither exposes a page-callable chrome
command or copies media into another browser window. Only a substantial playing
video with sound is eligible, and site PiP opt-outs remain effective. Returning
closes only presentation opened by this feature. Manual dismissal is respected,
rapid switches share pending native entry, visible split panes are excluded,
and profile/private-context changes cancel automatic presentation. The switch
is in Settings > Tabs; disabling it closes existing automatic presentation.
Existing discard protection now treats native PiP as active media even when
paused or muted.

The Windows native fixture opens PiP after hiding its source WebView2 and closes
it on return. JavaScript regression cases also exercise pending-entry races,
manual ownership, dismissal, opt-out, rejection and WebKit presentation dispatch.
This does not qualify native macOS behavior. The bounded video scan includes
the main document and accessible same-origin frames; cross-origin embeds and
sites that disable native PiP are not claimed as supported.

The integration exposed Chromium model-menu focus loss. Focus is now assigned
in the popover's opening lifecycle rather than an earlier render tick, and
closing from the trigger preserves focus restored to the composer. Existing
interaction assertions are retained. Windows Work qualification also needed
macOS-only profile seeding separated from shared loopback requests. Windows
refuses that optional seeding mode explicitly; it does not fake cookie-store
inspection results.

Correctness logs for this integration live under `target/review-qa/integrated-*`,
with native PiP evidence in `automatic-pip-native.log`. These are debug-build
checks, not release performance measurements. Final Windows appearance and
native macOS PiP still require owner acceptance.

### Integrated Windows verification and blockers

This integration is not release-qualified. The machine is Windows 11 Pro
10.0.26200 (build 26200), Intel Core i3-1115G4, four logical processors,
approximately 12 GiB installed RAM. WebView2 155.0.4283.45 and 155.0.4283.62
are installed; installation inventory alone does not identify which runtime
each completed fixture used. The final running Review QA process uses
155.0.4283.62, verified from its actual child executable. All native evidence
below uses debug builds.

Verified changes are `50e6a8c1` (automatic PiP), `5e727e94` (model-picker
keyboard focus), `9e4cc00a` (calendar edits commit only once on Enter/blur),
`e9079b0b` (build/test portability), `56a7f4d8` (Windows Work probe), and
`92c1bdf2` (Chromium editor fixture line breaks).
The earlier Windows branch remains available independently at `f937e2db`.

| Check | Observed result |
| --- | --- |
| Workspace/all-target check | Passed on the integrated tree; subsequent formatting and test portability changes are recorded separately |
| Engine with `agentic-browser` | 532 passed, 3 opt-in tests ignored |
| Composition with `durable-runtime` | 135 passed |
| Controller with `provider-transport,probe-harness` | 127 passed |
| Agentic with `provider-transport` | 1,110 passed after retaining SystemRoot in the isolated Windows proxy subprocess |
| App with `work-runtime,work-execution` | Initial 859 passed/41 failed/14 ignored; using safe temporary fixtures: 893 passed/7 failed/14 ignored |
| Frontend unit tests | Initial 907 passed and two load-related timeouts; both failing files passed a serial rerun (10 tests) |
| Scoped Chromium components | Initial 427 passed/5 failed/6 skipped; four affected files passed all 12 tests after the calendar fix and test portability/readiness corrections |
| Final frontend typecheck, scoped lint, build | Passed, zero Svelte diagnostics; existing bundle budgets retained |
| Native automatic PiP | Entry after hiding the page and exit on return passed |
| Final-tree native download and media fixture | Passed, including download provenance after browser exit and automatic PiP round trip |
| Rust formatting, compact semantic runtime, PiP JavaScript | Passed; all seven PiP policy/race tests passed |
| Native Work lifecycle, first preview, action guard | Passed |
| Native Work `webmail-inbox`, `writes`, `human` | Initial runs failed; fresh narrow reruns and limits are detailed below |
| Workspace source-contract tests (`xtask`) | 159 passed, 14 failed; the first workspace run stopped here |
| Remaining workspace targets (`--exclude xtask --no-fail-fast`) | Completed; only `aven-app --lib` failed (the default AppData temporary-directory run described above) |
| Strict workspace Clippy | Failed: oversized variants, boolean-expression lints, argument count and an `unreachable!` in the integrated Work code |
| Frame emitted-style check | Failed on the integrated Work input glow (`0 -10px 40px -20px var(--glow)`); the checker still requires at most a 3px contact-shadow blur |

Application tests must place disposable fixtures outside AppData. Work
intentionally rejects every AppData descendant, including the default Windows
temporary directory. Set `TMP` and `TEMP` for the test process to a dedicated
directory such as `target/review-qa/work-test-temp`. This does not change the
product policy. The seven remaining failures are one git-index test attempting
to overwrite a handle-pinned index (Windows sharing violation), two CLI tests
expecting execution despite `ensure_available()` refusing that capability, and
four trash-retention tests requiring a private namespace unavailable on Windows.
Do not enable those capabilities or loosen their guards to satisfy the tests.

The native mail fixture initially lost exactly three newlines: Chromium's
Shift+Enter inserts BR elements inside a paragraph, whereas the fixture read
`textContent`, which omits them. The controlled fixture now serializes those
breaks and excludes the trailing caret placeholder. `node --test
scripts/test-work-editor.mjs` verifies native paragraph insertion, Shift+Enter,
trailing breaks and rejection of unadmitted raw DOM replacement. The exact
native send assertion passes after rebuilding the probe: one approved send,
unchanged expected multiline content, no reply, clean native teardown.
The final run still reports failure to read/preserve the result after sending;
it explicitly refuses automatic repetition. Thus the fixture's exact-send
success is not evidence of complete end-to-end result extraction. Logs:
`work-webmail-diagnostic.log` and `work-webmail-final.log`.

The native `writes` and `human` fixtures encounter a selected-profile browser
generation mismatch after a temporary cookie-presence controller closes. The
new controller reports another browser PID while the retained process handle
still reports its original process running and its exit observer remains
pending. Admission correctly refuses and retains the uncertain owners; later
cases in the same fixture then fail because the profile is quarantined. This
does not establish a vendor defect. Fresh narrow `slackwrite` and `human-signin`
runs subsequently pass with clean exact teardown; the generation failure is
intermittent and remains unresolved. The sign-in fixture preserves the native
page through its hold, resumes with the original limits, and refuses a retired
check. This is synthetic handover evidence, not manual move/resize acceptance.
Reproduce in a fresh disposable session
with `windows-work-probe --loopback-site slackwrite` or `human-signin`; never
accept a different PID without exact retirement proof for the old generation.

The 14 source-contract failures are in `agent_controller_boundary`,
`agentic_probe_boundary`, and `work_resource_boundary`. Several assertions
search for implementation literals that moved in the integrated Work branch.
They still need a security-boundary review; no assertion was rewritten just
to obtain a passing gate. Full details are retained in
`target/review-qa/integrated-workspace-tests.log` and
`target/review-qa/integrated-clippy-4.log`.

The final QA executable is `target/review-qa/Aven Review QA.exe`, built with
`--debug --no-bundle --config ../target/review-qa/tauri.review-qa.conf.json`.
Its identifier is `app.aven.review-qa`; its security configuration matches
the integrated product configuration. Direct launch from the packaged development
host redirects newly created Roaming descendants into that host's MSIX cache,
which correctly fails the runtime canonical-parent check. Launching through
`explorer.exe` opens the app normally outside that inherited package identity;
the window is responding. No ownership check or product storage policy changed.

## Remaining acceptance and engineering work

The extension delivery issue above is an engineering follow-up. The inventory
also leaves installer interruption, locked-file storage behavior, private-data
residue, complete log-content privacy, and full Work bridge enforcement open;
code inspection and the passing workspace suite do not close those native cases.
Existing denied/gated capabilities must not be advertised as Windows parity.

Owner acceptance should concentrate on video fullscreen versus F11 (including
Snap/maximize restoration and monitor changes), keyboard focus/IME across the
address field and pages, and minimize/lock/sleep/wake with active media. Use a
separate QA identity for a desktop build. A debug Review QA executable is open;
no release package or installer was produced by this pass. Check download
Open/Show in Folder, slow video fullscreen entry/exit, and automatic PiP when
switching tabs and returning. Native macOS automatic PiP still needs its own
build and acceptance run.

For resource measurements, use a release build with the same tabs and settings,
record WebView2 version and hardware, and include all child WebView2 processes.
Measure protection on/off over repeated matched loads, plus focused/minimized
idle and tab counts. This remains owner-run work; the code changes above make
no claim that blocking has zero cost on every page.

## Decisions for a later reviewed change

- Local AppData: migrate only with exclusive profile ownership, verify the copied
  databases and profile tree before switching, retain a recoverable old location,
  and make an interrupted migration resumable. Do not move a live WebView2 UDF.
- Windows install-on-quit: prepare/reverify the installer before terminal teardown,
  launch only after clean shutdown, suppress relaunch for ordinary quit, and keep
  installer execution outside the hard-exit watchdog. Keep this a separate change.
- Uninstall: recommend retaining browsing data by default, with an explicit
  owner-approved removal option if desired.

No migration, installer-scope change, signing implementation, or release-blocker
change is part of this pass. This change is suitable for draft review of the
verified fixes, not release approval. Merging remains a separate owner decision;
the existing pending-vendor-fix publication block remains active.

## Windows chrome polish (2026-10-19)

The acrylic wash was overridden by the more specific transparent-material
reset. Its ground now lives in the material axis, retaining the existing 78%
theme tint instead of exposing bare system acrylic. Vibrancy and the opaque
fallback keep their previous grounds.

Windows now reveals a fading native caption across the top of the page.
It leaves sidebar navigation accessible; internal pages reveal corner controls.
The page retains its 8 DIP gutter, with no reserved title band and no page
resize on hover. The existing native overlay handles pointer input above
WebView2, dragging, resize edges, minimize/restore/close and keyboard focus.
Maximize nonclient messages target the resizable owner for Snap integration.
The reveal target also works when maximized and retires while the caption
is visible so it cannot intercept the buttons. Fullscreen suppression is
per window. Reduced motion and high contrast skip the fade.
The retired DOM caption no longer receives JavaScript state updates on
every resize and focus event. A reveal originating above the sidebar keeps
checking for departure until the pointer leaves that otherwise hidden target.

The Windows rail beside Downloads and other tool panels puts Browse/Work
in its top row and search below it. Settings keeps the same ordering;
macOS keeps its traffic-light and search layout.

Owner acceptance for this pass:

- Compare dark and light material over the same wallpaper and over another
  window; check the sidebar and outer gutter for a visible seam.
- Hover the top edge, then leave it. The caption must fade over a stationary
  page. Repeat maximized and after moving between monitors with different DPI.
- Use caption drag, double-click, minimize, restore, close and maximize-hover
  Snap. Check Alt+Space and Alt+F10 keyboard access, and return focus to the page.
- Enter and exit video fullscreen and F11; a second browser window must keep
  its controls. Check Browse/Work and search in Downloads, Bookmarks and Settings.

These are native visual/input acceptance checks, not claims established by
the component tests. The broader engineering limitations above remain open.

Validation for this UI pass: all 10 scoped shell/sidebar component tests pass,
including the computed acrylic ground and Windows rail ordering. The eight
core layout tests and the native caption resize-hit test pass on Windows.
Frontend type checking, lint, stylelint, formatting, unused-code checks and
both UI builds pass; bundle budgets are unchanged. The unit run passed 908
of 909 tests; its locale test imported the entire generated message catalog
and timed out even on a focused rerun. Importing only the message that test
exercises preserves its assertions and makes all five locale tests pass in
1.61 seconds (909 unit tests passed across the run and focused recheck).
The emitted-style gate still rejects the existing Work glow's 40px shadow;
that previously recorded failure is unchanged.

Implementation: `03bab7f2` (Windows chrome) and `d4507b04` (locale test).
The final debug Review QA build succeeded and was launched through Explorer;
the isolated executable has a responding window. Its SHA-256 is
`b4c2726a4e4c1cb65b382f19a4bf7df816d361b3885f60dae153243d632a9305`.
No installer or release-performance measurement was produced.

## Caption refinement after owner feedback (2026-10-19)

This supersedes the preceding caption appearance and acceptance notes. The
window's theme wash is now 68% rather than 78%. The caption exposes the same
acrylic as the sidebar: its background is color-keyed instead of painting a
separate opaque grey band. High contrast retains the system's opaque colors.

Revealing the caption translates the native page containers downward by 40 DIP
over 400 ms. The WebView2 controller keeps its dimensions, and the stage clips
the displaced page to its original bounds. This uses the split preview's native
window positioning mechanism, not a screenshot or a compositor-only transform.
One animation clock drives the controls and the page. Reduced motion skips the
animation. Fullscreen suppression and native move/size transitions restore the
page's original position; translation remains local to each window.

The input surface covers each complete 46-by-32 DIP button rectangle, including
transparent pixels. A press captures input and activates only after release in
the same button. Motion pauses during a press so a moving target cannot escape
the pointer. Maximize clicks now issue an explicit owner system command rather
than forwarding nonclient button tracking between different HWNDs. Maximize
hover still forwards to the resizable owner for Snap. Caption dragging waits
for the system drag threshold, preserving the second click of a double-click.

Focused owner acceptance:

- Reveal and dismiss the caption over a web page and a split. Check for smooth
  travel in both directions, unchanged page layout, and matching acrylic.
- Click maximize near each corner and in the center of its button; repeat to
  restore. Press, move outside, and release: it must not activate. Try a press
  during reveal, caption double-click/drag, and the maximize-hover Snap menu.
- Repeat at fractional DPI, after maximize/minimize, and after video fullscreen
  or F11. Check a second window, internal pages, and keyboard caption access.

Native visual/input acceptance remains owner-run: desktop automation did not
expose the existing Review QA window in this session. This pass does not replace
the broader release qualification and open failures recorded above.

Validation: `frame check` passes, including all 909 unit tests; both focused
Windows chrome component tests pass. Both frontend builds pass without budget
changes. Eight Windows stage tests and two native caption geometry tests pass,
including unchanged controller dimensions during translation, fractional DPI,
negative monitor coordinates, and isolation after owner-handle reuse. Formatting
and the isolated debug application build pass. The emitted-style gate still
rejects the existing Work glow (`0 -10px 40px -20px var(--glow)`) against its 3px
contact-shadow limit; that unrelated visual treatment was left unchanged.

## Alpha caption rendering and lighter material (2026-10-20)

This supersedes the preceding color-keyed caption implementation. A small,
cached, premultiplied-alpha bitmap now draws the three controls with antialiased
glyphs, rounded hover surfaces and focus rings. Native BUTTONs retain input and
accessibility on a separate input host; their default theme painting cannot
appear in the visible surface. Both owned windows hide before close or a window
state command can trigger focus repaint. A vetoed close restores the reveal
target. High contrast uses the system's opaque colors.

The window wash is 28%, down from 68%; raised surfaces use 56%, while launcher
contrast is retained. Windows 10's additional acrylic tint is also reduced.
This remains Windows native acrylic, not an implementation of refractive glass.

Caption travel is 32 DIP over 220 ms, with 44-by-26 DIP button targets. Page
containers still move without resizing the WebView2 viewport. A press or native
move loop holds the current position until release; dragging no longer retires
the caption. Hover works on an inactive window without deliberately raising or
activating it. Clicking activates the owner. Keyboard access exposes the full
target before focusing it, and accessibility invocation does not depend on the
cursor position. Stationary hover uses leave tracking instead of periodic
dismissal polling. Geometry and bitmap updates are skipped when unchanged.

The alpha buffer occupies 17,664 bytes at 100% scaling and 70,656 bytes at 200%.
These figures exclude the input host, Windows and compositor allocations.
On this Windows 11 build 26200 / Core i3-1115G4 machine, an optimized isolated
benchmark of the exact `raster` function measured:

| Scale | Mean redraw | Median | p95 |
| --- | --- | --- | --- |
| 100% | 223.18 us | 215.60 us | 242.50 us |
| 200% | 863.45 us | 815.90 us | 969.70 us |

Method: compile the constants, Appearance, dimensions and pure raster helpers
from `windows_caption_paint.rs` with `rustc -O --edition=2021`; allocate the
buffer once, vary the hovered button and maximized glyph each iteration, and
pass input/output through `std::hint::black_box`. Discard 100 warmups and record
2,000 timed redraws per DPI. Local harness and output are in
`target/review-qa/caption_raster_bench.rs` and `caption-raster-bench.log`.
Animation reuses the rasterized pixels until appearance changes. This measures
neither layered-window upload nor page movement, full-app CPU/GPU, or battery
use. It is not release-performance qualification.

The implementation follows Microsoft's advice to keep an
[UpdateLayeredWindow surface small](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-updatelayeredwindow).
The visible surface never calls SetLayeredWindowAttributes, which is incompatible
with subsequent UpdateLayeredWindow calls until the layered style is reset.
The engine still positions HWND containers; a compositor-only page animation
would require a separate WebView2 hosting change. Existing page-container corner
regions are unchanged; the new alpha edges belong to the caption controls.

Focused visual acceptance: compare material over the same wallpaper; reveal the
caption in Browse, Work and Settings; hover and press each button, drag while
holding the caption, repeat with an inactive window, then close. Check for white
flashes, stable hit targets and smooth rounded edges at fractional DPI. Repeat
maximize/restore, Snap, Alt+F10, reduced motion and high contrast. Automated pixel
and native-surface tests do not substitute for this owner acceptance.

Validation: all five focused native caption tests pass, including real hidden
layered-window presentation, cropping and DPI changes on this machine. Rust
formatting and strict desktop Clippy (`--lib --tests --no-deps -- -D warnings`)
pass. `frame check` passes all 909 unit tests and its type/lint/style/format
checks; both focused Windows chrome component tests and both UI builds pass.
Bundle budgets are unchanged. Dependency-inclusive Clippy still stops on the
seven existing agent-controller diagnostics (large variant, boolean expressions,
argument count and `unreachable!`). The emitted-style gate still rejects the
existing Work glow's 40px shadow. Neither gate was weakened.

The isolated debug Review QA build succeeded and opened with a responding
window. Its SHA-256 is
`425b34603c4e3b39a466de7edf9f3d0bcdceb145ccb0dc8748788707457c68da`.
After testing it, the owner reported it was mostly okay and requested committing
and pushing the completed work. This feedback does not establish every optional
visual check above or broader release qualification.

## Window controls drawn by the chrome (2026-11-16)

This supersedes the native caption's visible surface. Since the controls were
pinned out in their own 36 DIP band above the page, the owned layered popup
that drew them was on screen at all times, and it is a separate top-level
window: it reached the window's new frame at the start of the system's
maximize, restore and minimize animations, and trailed a drag by a frame. For
the length of each, the controls stood detached over whatever application was
behind. The owner reported them coming into contact with other applications.

The chrome now draws Minimize, Maximize or Restore, and Close itself
(`frame/src/shared/ui/WindowCaption`), fixed in the window's top-right corner
inside that band, 46 by 36 DIP each, in the Browse shell and in onboarding
(without Maximize while onboarding holds one size). They are part of the
window, so they move, animate and hide with it. `windows_caption.rs` keeps
what only native can do: the top resize border, the system menu on Alt+Space,
focus on reactivation, and an `aven:maximized` event that tells the chrome
which glyph to show. `CHROME_DRAWS_CONTROLS` stops the native surface from
ever coming out, and the top-edge input region now leaves the controls'
corner to the chrome down to its first pixel, so a pointer thrown at the
corner of a maximized window still lands on Close.

Given up for now: the Windows 11 Snap Layouts flyout on hovering Maximize,
which needs `HTMAXBUTTON` from a native hit target, and F10 moving the
keyboard to the controls. Win+Z, Win+arrows, Alt+Space and dragging to a
screen edge are unchanged. The controls are out of the tab order, as a title
bar's are.

The same change set remembers the main window's frame
(`desktop/src/window_placement.rs`): `GetWindowPlacement` at close, restored
with `SetWindowPlacement` while the window is still hidden, and maximized
again as it is shown. A frame is taken only while a screen still shows its
top edge. Windows only; macOS keeps the platform default and is untested.

Validation on this Windows 11 host (25H2, two 1920x1080 screens at 100%):
`cargo test -p aven-desktop --features tauri/custom-protocol --lib` passes
395 tests with 3 ignored; `cargo clippy -p aven-desktop --all-targets --locked
-- -D warnings` is clean; `cargo test -p aven-app --lib` passes 855; frame
typecheck, ESLint, Stylelint and Prettier pass on the changed files, and the
unit project passes except two timing-bound tests that pass when run alone.
In a development build driven over the inspector port: the three controls
minimize, maximize, restore and close the window; the glyph follows the
window's state, including a window restored maximized at launch; the top-edge
region excludes the controls' corner at its first pixel row; a window closed
at 1500 by 850 on the second screen reopens there, and one closed maximized
reopens maximized with its unmaximized frame kept.

Not established: the look of the controls on a real screen during the
system's animations (no screen capture was taken over other applications),
fractional DPI, high contrast, touch and Narrator, the onboarding window,
and any macOS behaviour. Component tests were not run.

## Local bridge, vault and game mode hardening (2026-11-17)

Scope: the loopback bridge to a local Claude Code install
(`desktop/src/cli_bridge.rs`), the vault (`desktop/src/vault.rs`), game mode's
process tree (`desktop/src/game_mode.rs`) and the task manager's sleep request.

The bridge listened on 127.0.0.1:11500 with no caller check, so any process of
any account on the machine could spend the person's subscription and read the
answers. Every request must now start with a secret: 32 random bytes kept in
the Credential Manager for this Windows account, compared in constant time; a
request without it gets 401. The bridge is offered to the interface only when
this process bound the port, with the secret in the address it offers. Codex
was removed from the bridge because it cannot run without its own tools. Page
and tool text reaches the model as one JSON object per line, so it cannot
close its own turn and open another. The program is looked for folder by
folder along absolute `PATH` entries only. Connections are capped at 8, a
request must arrive within 10 seconds, and a turn waits at most 120 seconds.

The vault is written to a file beside it, flushed to the disk, then moved
over; the vault it replaces stays as `.bak`. A save larger than the size a
load accepts is refused. A vault whose key is no longer in the Credential
Manager is no longer given a new key. Vault commands run off the UI thread,
and the codes on screen are read in one call.

Game mode walks the process tree by parent id. A child is kept only when it
started no earlier than its parent, so a process that inherited a recycled id
is neither counted nor slowed.

Evidence on Windows 11 (10.0.26200), development build: `cargo test -p
aven-app --lib` 858 passed; the desktop library tests 398 passed and 3
ignored, then the vault tests again (6 passed) after the backup test was
added; Clippy with `-D warnings` on aven-core, aven-ipc, aven-app,
aven-engine, aven-agentic and aven-desktop; frame typecheck 0 errors; the unit
project 1367 passed and one timing-bound test failed under load and passed
alone. Over the inspector port on a hidden desktop: the bridge answered 401
without the secret and with a wrong one, 200 with it; the sleep request on a
shown tab was admitted and the tab stayed awake; a malformed id was refused.

Not established: a hidden tab put to sleep by hand in the running build (game
mode was active and had already put hidden tabs to sleep; the reducer tests
cover it), the vault's backup and missing-key paths in the running build, the
bridge driving a real Claude Code turn after the change, component tests, a
release build, and any macOS behaviour. Still open: an endpoint saved before a
port squatter took 11500 is not cleared, the bridge's child process tree is not
in a job object, copied secrets stay in the clipboard history, and deleting a
profile leaves its vault file and key.

## Page tuning over the DevTools channel, vault fill and streamer mode (2026-11-17)

Scope: forced dark mode, the network limit and consent refusal
(`crates/aven-engine/src/{devtools,force_dark,network_limit,consent}.rs`),
vault fill (`crates/aven-core/src/login_fill.rs`, `vault_fill` in
`desktop/src/vault.rs`), the apps in the tools menu and streamer mode
(interface only).

The engine asks each content view's own DevTools channel
(`CallDevToolsProtocolMethod`) for three things, when a view is made and when
the person's choice changes. Nothing is evaluated in the page's world for
them. Forced dark mode is `Emulation.setAutoDarkModeOverride`; turning it off
clears the override instead of forcing light. The network limit is
`Network.emulateNetworkConditions`; this runtime applies it only while the
view's Network domain is enabled, so the domain is enabled for as long as a
limit holds and disabled with it. Consent refusal registers one script in a
world of its own (`Page.addScriptToEvaluateOnNewDocument` with a world name);
it clicks a refusal the site offers and holds nothing worth taking. Leaving
the Page domain drops it. All three reach the page and the frames of its own
site; a frame of another site, which Chromium runs apart, is not reached.

Vault fill sends a saved sign-in only to the page in front, and only when
that page is an encrypted page of the host the sign-in was saved for. The
shell checks this against the address it holds for the tab, and the
expression checks `location` again in the document it runs in, so a page that
navigated in between gets nothing. Values are inserted as JSON strings in one
pass over the template, are wiped when dropped and are never printed. The
fill runs in the page's main world: once filled, the page can read what was
put in its own fields, as with any autofill. The UTF-16 copy handed to
WebView2 is not wiped.

Streamer mode marks the interface's document, and one rule blurs what is
marked private: the account's e-mail and the vault's names, secrets and
codes. Pages are not changed, and notifications are not hidden.

The startup size baseline in `frame/bundle-budgets.json` was recorded again
(browser JS 517238 to 537003 bytes): message texts ship in the startup
bundle for all thirteen languages, and these features add twenty-two of
them. The vault fill section itself is loaded when a site menu opens.

Evidence on Windows 11 (10.0.26200), development build: desktop library
tests 399 passed and 3 ignored; `cargo test -p aven-app --lib` 862 passed;
`cargo test -p aven-core --lib` 527 passed; the engine's force_dark,
network_limit and consent tests 7 passed; Clippy with `-D warnings` on
aven-core, aven-ipc, aven-app, aven-engine, aven-agentic and aven-desktop;
frame typecheck 0 errors, unit project 1371 passed, ESLint, Stylelint, knip
and check:styles clean on the changed files. Over the inspector ports on a
hidden desktop, through the preferences: a light-only page read 255, then 18
with forced dark, 18 again after a navigation to another site, 255 once off;
400000 bytes took 185 ms, 3577 ms under a 1 Mbit/s limit and 99 ms once
lifted; an injected OneTrust refusal button was clicked with the preference
on and left alone with it off; a test sign-in was refused on another site
with the fields untouched and filled on its own site; the streamer mark
followed the preference.

Not established: the network limit switched by a game taking the foreground
in the running build (the shell test covers the decision), whether a file
saved through the download manager is held to the limit, the look of the new
settings rows and of the site menu's fill section, the apps opened from the
tools menu in the running build (the plan they follow is unit tested),
consent refusal on real sites, component tests, a release build, and any
macOS behaviour: all of it is Windows only, and the engine port defaults do
nothing elsewhere.

## Page views kept below the window controls (2026-11-18)

Correction to "Window controls drawn by the chrome (2026-11-16)". The chrome
keeps a 36 px band for the controls it draws, and its own pages (the new tab,
Work) started below it. A web page did not: native places page views from
`aven_core::layout`, whose `caption` metric was still zero, so a page view
started at 8 px and covered the lower 28 px of the three controls. Native
views sit above the chrome, so the controls showed only as a strip and their
hover state bled around the page's corner. The earlier review had checked the
controls through the interface's DOM and on the new tab, never under a web
page.

The host now tells the shell the band (`Command::SetCaptionBand`, 28 px on
Windows, sent before the first layout), every window keeps it in its layout
metrics, and a value that is not a small finite height is no band. Fullscreen
still gives the page the whole window.

Evidence on Windows 11 (10.0.26200), development build: `cargo test -p
aven-app --lib` 863 passed, the desktop library tests 399 passed and 3
ignored, Clippy with `-D warnings` on aven-core, aven-app and aven-desktop.
Measured on a hidden desktop from the window's child windows, with a web page
in front: the page view sat at 257,8 with a size of 1219 by 825 before, and
sits at 257,36 with 1219 by 797 after, in a 1484 by 841 window; maximized to
1920 by 1032 it sits at 257,36 with 1655 by 988.

Not established: the look on a real screen, split view, a hidden sidebar, the
Work pane beside a page, fullscreen in the running build, and any macOS
behaviour (the band is zero there).

## Agent control, typed sign-ins, tab volume and per-site exceptions (2026-11-19)

Scope: an outside agent acting on a page (`desktop/src/agent_consent.rs`,
`outside_controls` and `outside_act` in `desktop/src/page_assist.rs`, the
new tools of `desktop/src/mcp_server.rs`), the pointer that shows where any
agent acts (`pointAt` in `page_reader_recipes.js`, `PageRecipe::point`),
whole-page and screen pictures (`page_whole_view`, `desktop/src/screen_capture.rs`),
the sign-in typed on a page saved to the vault (`TYPED_LOGIN`,
`vault_save_typed`), secrets copied out of the clipboard history
(`desktop/src/private_clipboard.rs`), a vault removed with its profile, tab
volume (`crates/aven-engine/src/tab_volume.rs`, `platform/tab_volume.js`),
per-site exceptions (`crates/aven-core/src/site_list.rs`), streamer mode's
native side (`desktop/src/streamer.rs`) and tagged tool calls in the Claude
Code bridge.

An outside agent reads freely but acts only behind a yes given in the
chrome: the page cannot answer the card, and neither can the agent. A yes
covers one action, or one site in one tab until the person stops the agent;
a step judged to send, buy, delete or save is asked every time. No answer in
60 s is a no, at most four requests wait, and the request is planned again
after the yes: if the control changed meanwhile, nothing runs. Steps go
through the same planner and executor as the built-in assistant, so password,
card and one-time-code fields and sensitive sites are refused there too; such
fields are not even listed to the agent. Nothing about consent is kept on
disk, and the action's label, which is page text, is shown as text and never
logged.

The pointer and the "Aven is working" caption are drawn in the reader's
closed shadow layer: they take no input and change nothing on the page. The
caption's words come from the interface's translations, are stripped of
control characters and bounded to 80 characters. The pointer is taken off
before an outside agent's picture of the page.

The typed sign-in is read once, on the person's click, by an expression run
through the view's DevTools channel, only on an encrypted page; the address
it is saved for is the one the browser committed, not what the page says.
The page answers with JSON that holds a password: it is wiped when dropped,
bounded, and never logged. A secret copied from the vault is put on the
clipboard with the three formats that keep it out of the history and the
cloud clipboard, and taken off after 45 s unless something else was copied.
A vault and its key go only with a profile the person asked to delete, once
the shell's own list no longer has it; an empty list is never read as every
profile gone.

Tab volume wraps `HTMLMediaElement.volume` and each audio context's output
in the page's own world, as WebView2 offers only mute. A page can undo it for
itself, and a frame of another site keeps its loudness until it loads again.
Per-site exceptions are hosts only, bounded to 200 and 16 KiB, and refused as
a preference value otherwise.

The WebView2 floor review of 18 November is recorded in `webview2.rs` and
`security-model.md`, including Microsoft's standing pending-fix notice of
17 November, which holds production release.

Evidence on Windows 11 (10.0.26200), development build: desktop library
tests 420 passed and 3 ignored; `cargo test -p aven-app --lib` 867 passed;
`cargo test -p aven-core --lib` 531 passed; the engine's recipe, volume and
consent tests 13 passed with the reader built in; the assistant's page tests
59 and the prompt tests 62 passed; Clippy with `-D warnings` on aven-core,
aven-ipc, aven-app, aven-engine, aven-agentic, aven-page-assist and
aven-desktop; frame typecheck 0 errors, ESLint, Stylelint, knip and
check:styles clean; frame unit project 1373 passed, the one file that timed
out under load passing alone. On a hidden desktop, over the inspector ports
and a test page served on 127.0.0.1: the nine MCP tools listed, the password
field not among the controls, the card in French, a click too soon after it
appeared ignored, a press, a type, a tick and a choice carried out, a yes for
the site sparing the next question, stop asking again, a refusal and a stale
read refused; the pointer and caption seen on the page and gone after;
a whole page pictured from top to bottom; a tab at 30 % playing at 0.3 and
0.15 once the page set 0.5, kept across a reload and back to 1 at 100; a
light page at 255, 18 darkened, 255 set apart, 18 again; nothing typed
saved nothing, a typed test sign-in saved once and updated, not duplicated;
a copy of ping.exe named obs64.exe turned streamer mode on, the automatic
switch off turned it off, and its end turned it off. The built-in assistant
with a Claude Code model closed a newsletter box by its "No thanks", pressed
the button behind it and answered with the offer.

Not established: a real recording program; the clipboard history exclusion
(the clipboard was left alone); a picture of the screen taken in the running
build; the whole-page and screen chips sent from the panel; the site menu's
new rows and the settings lists seen on screen; a profile deletion removing
its vault in the running build (unit tests cover the decision and the
files); sound actually heard at the set level; consent refusal exceptions on
real sites; a release build; any macOS behaviour. Through the Claude Code
bridge a model sometimes writes its own correction ("I must output only
JSON") into the reply before the call it meant; the call is still read.

## Auto mode, translations and the security review of 19 November (2026-11-19)

Scope: Auto mode in the assistant (`auto` on `PageAssistAskV1`, `prompt::AUTO`,
`MAX_CALLS_AUTO`, `MAX_READS_AUTO`), the pointer shown before a link is
followed (`PagePort::point`), the Claude Code bridge's reading of replies
(`answer_object`, `strip_call_tags`), the page reader's role for a combobox
that takes typing, the Ask button in the sidebar, step labels in the
person's language (`$shared/lib/step-label.ts`), translations, class names
that clashed with stylesheet utilities; then a reading of everything Aven
added to the base for defects: `desktop/src/vault.rs`, `local_http.rs` (the
code `cli_bridge.rs` and `mcp_server.rs` share), `agent_consent.rs`,
`private_clipboard.rs`, `game_mode.rs`, `screen_capture.rs`, `streamer.rs`,
`window_placement.rs`, the outside-agent path of `page_assist.rs`, the
engine's page scripts and the assistant's labels.

Auto mode is the person's choice, made per conversation in the composer and
sent with each message. It takes away one question only: a search is made
without asking, still shown with its exact query and destination. Every
step that sends, buys, deletes or saves still waits for the person's
confirm, password, card and code fields stay refused, sensitive sites stay
unread, and the hand-off to Work is not offered. The longer limits (48 model
turns, 36 page reads) bound a mission.

What the review changed:

- The vault opened for whatever profile id the interface named. In a private
  window that was the private profile's, so opening the site menu made a
  Credential Manager key for it, and saving a typed sign-in wrote a vault
  file, both kept after the window closed. Native now opens a vault only for
  the persistent profile the shell says the window browses as, never while
  its private scope shows, and only a first save makes a key. The interface
  offers no vault in a private scope. An item's fields are wiped when it is
  dropped, and a secret shown on the vault page hides again after 30 seconds,
  when the window is put away, or when the profile changes; the password
  generator no longer folds random bytes onto indices.
- Both local endpoints read a whole body, up to 16 MiB, before checking the
  secret, so any local process could make the browser hold 128 MiB across
  eight connections. The secret is now checked on the request's head, before
  its body, and the MCP server takes 1 MiB per message. A listener whose
  `accept` fails rests 100 ms instead of spinning.
- Turning the MCP server off dropped its listener only: a request waiting
  for the person's answer lived on, and so did every yes given for a site.
  Connections now belong to the listener's task and end with it, and turning
  the server off takes back every agreement. A start that finishes binding
  after the person turned the server off again stops at once. The server's
  description told agents it could not act on pages; it now says each action
  waits for the person.
- The MCP address embeds the server's secret and was copied the ordinary
  way, into the clipboard history and the cloud clipboard where those are on;
  it now goes the vault's private way. The private clipboard reads its
  sequence number while still holding the clipboard, so another program's
  copy is never the one it clears later, and wipes its UTF-16 copy.
- Game mode treated a maximized window as covering the screen when the
  taskbar hides itself; a window with a title bar or maximized is no longer
  taken for a borderless game. One walk of the process list per poll now
  serves both the process tree and streamer mode.
- Labels built from an address split it by hand: text before an `@` was read
  as the host, and `\` did not end the authority. A search engine's redirect
  was named by the first address it carried, which a page could pair with
  another. Both are fixed; what is opened was never affected.
- `mcp_server` used code compiled only with the `work-product` feature; that
  code moved to `local_http.rs`, built in every configuration.
- A compatible server's model list now gives the name and context window it
  states, bounded downstream as before.

Read and left as they are: the consent state (`agent_consent.rs`), the
outside agent's steps, planned again after the person's answer and refused
on password and secret fields, `screen_capture.rs`, `window_placement.rs`,
`tab_volume.js`, `consent_refuse.js` (its own world, run once per document),
`login_fill.rs` and the bridge's process handling (input on standard input,
no tools, settings or MCP servers of its own).

Evidence on Windows 11 (10.0.26200), development build: the desktop (424),
application, core, engine, assistant (62), prompt and model listing tests,
with new tests for the vault's profile rule, the request reader's refusals,
compatible model names and the hosts labels name; Clippy with `-D warnings`;
rustfmt; the frame's typecheck, ESLint, Stylelint, knip, check:styles and
unit project (1,382 tests). On development builds of the same day, in a
separate profile: an Auto mission that searched, refused a consent page
itself, opened Wikipedia and answered with sources, and an outside agent
typing into DuckDuckGo and pressing Return. On the final build, a tour of
every panel, menu and settings page in light and dark raised no error.

The 1.0.5 installer was published on 19 November by the maintainer's
decision while Microsoft's announced WebView2 fix was still outstanding:
the release check in `xtask` refuses it, and it was built outside that
check. WebView2's Evergreen runtime updates itself through Windows,
independently of Aven; the next release raises the floor to the fixed
runtime once Microsoft ships it.

Not established: the component tests, which need a Playwright browser that
was not installed on this machine; the private-window vault refusal and the
game-mode window rule on a live window; a real OBS for streamer mode; the
clipboard history exclusion as Windows shows it; translations reviewed by
native speakers.
