/**
 * The rules that hold every animation still: for the person's own "reduce
 * motion" choice, and while a game fills the screen. They match every
 * element, so they are in the document only while one of those reasons
 * holds; left in it for nothing, they cost each style recalculation a pass
 * over every element. Durations collapse to a frame instead of zero so
 * transitionend and animationend still fire, and delays go, so nothing waits
 * at its first invisible frame. The operating system's own preference keeps
 * its media query in `styles/axes/motion.css`.
 */
const RULES = `*, *::before, *::after {
  scroll-behavior: auto !important;
  transition-duration: 0.01ms !important;
  transition-delay: 0s !important;
  animation-duration: 0.01ms !important;
  animation-delay: 0s !important;
  animation-iteration-count: 1 !important;
}`;

export type StillReason = "motion" | "game";

const holding = new Set<StillReason>();
let sheet: CSSStyleSheet | null = null;

/** Adds or takes away one reason to hold still; the rules stay attached
 *  while any reason holds. */
export function holdStill(reason: StillReason, on: boolean): void {
  if (on) holding.add(reason);
  else holding.delete(reason);
  if (typeof document === "undefined" || !("adoptedStyleSheets" in document)) return;
  if (sheet === null) {
    sheet = new CSSStyleSheet();
    sheet.replaceSync(RULES);
  }
  const attached = document.adoptedStyleSheets.includes(sheet);
  if (holding.size > 0 && !attached) {
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
  } else if (holding.size === 0 && attached) {
    document.adoptedStyleSheets = document.adoptedStyleSheets.filter((kept) => kept !== sheet);
  }
}

/** Whether the rules are attached now. */
export function stillHeld(): boolean {
  return holding.size > 0;
}
