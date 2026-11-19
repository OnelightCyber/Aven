/**
 * What Aven uses and whether it is stepping back, as native reports it every
 * few seconds (Windows): memory for all of Aven's processes, game mode, and
 * efficiency mode under the processor cap.
 */
let memory = $state<number | null>(null);
let game = $state(false);
let throttled = $state(false);
let listening = false;

function listen() {
  if (listening || typeof window === "undefined") return;
  listening = true;
  window.addEventListener("aven:resources", (event) => {
    const detail = (event as CustomEvent<unknown>).detail;
    if (typeof detail !== "object" || detail === null) return;
    const { memory: mb, game: playing, throttled: slow } = detail as Record<string, unknown>;
    if (typeof mb === "number" && Number.isFinite(mb) && mb >= 0) memory = mb;
    if (typeof playing === "boolean") game = playing;
    if (typeof slow === "boolean") throttled = slow;
  });
}

/** Megabytes used by Aven and its pages, once native has said. */
export function memoryUsed(): number | null {
  listen();
  return memory;
}

/** Whether a game fills the screen and Aven is stepping back. */
export function gameMode(): boolean {
  listen();
  return game;
}

/** Whether Aven runs in efficiency mode (game mode or the processor cap). */
export function efficiency(): boolean {
  listen();
  return throttled;
}
