import { commands } from "$shared/ipc/bindings";
import { IS_WINDOWS } from "$shared/platform";

/**
 * Whether a recording or streaming program runs, on Windows. Native says so
 * each time it changes; it is asked once, for a program already running when
 * the chrome mounts.
 */
let capturing = $state(false);
let watching = false;

export function captureRunning(): boolean {
  if (IS_WINDOWS && !watching && typeof window !== "undefined") {
    watching = true;
    window.addEventListener("aven:capture", (event) => {
      capturing = (event as CustomEvent<unknown>).detail === true;
    });
    try {
      void commands
        .streamerCapturing()
        .then((running) => {
          capturing = running;
        })
        .catch(() => {});
    } catch {
      // Outside the app there is no native side to ask.
    }
  }
  return capturing;
}
