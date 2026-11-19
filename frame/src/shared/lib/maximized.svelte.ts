import { commands } from "$shared/ipc/bindings";
import { IS_WINDOWS } from "$shared/platform";

/**
 * Whether the window is maximized, on Windows, where the chrome draws the
 * window's controls and one of them maximizes or restores. Native says so
 * each time it changes; it is asked once, for a window already maximized
 * when the chrome mounts.
 */
let maximized = $state(false);
let watching = false;

export function windowMaximized(): boolean {
  if (IS_WINDOWS && !watching && typeof window !== "undefined") {
    watching = true;
    window.addEventListener("aven:maximized", (event) => {
      maximized = (event as CustomEvent<unknown>).detail === true;
    });
    try {
      void commands.captionReplay().catch(() => {});
    } catch {
      // Outside the app there is no native side to ask.
    }
  }
  return maximized;
}
