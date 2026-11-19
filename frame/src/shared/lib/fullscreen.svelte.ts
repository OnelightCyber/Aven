import { getCurrentWindow } from "@tauri-apps/api/window";
import { IS_MAC, IS_WINDOWS } from "$shared/platform";

/**
 * Whether the window is fullscreen. On Windows that is F11, where the chrome
 * steps aside and leaves the page alone. On macOS AppKit hides the traffic
 * lights and the row the chrome keeps for them is empty. Native says so the
 * moment a transition begins, from AppKit's own notifications, so the chrome
 * moves with the window whether or not its size changes (a window filling
 * the desktop is as large as a fullscreen one). The window is asked once at
 * the start, for one restored already in fullscreen.
 */
let full = $state(false);
let watching = false;
// A notification is newer than any answer to the first question.
let heard = false;

export function fullscreen(): boolean {
  if ((IS_MAC || IS_WINDOWS) && !watching) {
    watching = true;
    window.addEventListener("aven:fullscreen", (event) => {
      heard = true;
      full = (event as CustomEvent<unknown>).detail === true;
    });
    try {
      void getCurrentWindow()
        .isFullscreen()
        .then(
          (value) => {
            if (!heard) full = value;
          },
          () => {},
        );
    } catch {
      // Outside the app's own window there is no window to ask.
    }
  }
  return full;
}
