import { NAME_BOX, NAME_DOTS } from "./dots";

/** When each part of the launch happens, in milliseconds from the window
 *  showing. The page is held off the screen only until `doneAt`. */
export const LAUNCH = {
  /** Each dot takes this long to settle in its place. */
  land: 360,
  /** The glint crosses the name, left to right, from here. */
  glintFrom: 700,
  glintSpan: 240,
  glint: 300,
  /** The name holds, whole, until the cover fades and the sidebar's own
   *  entrance runs: long enough to read it, short enough not to wait. */
  fadeAt: 1150,
  fade: 250,
  doneAt: 1400,
} as const;

/** The widest the name is drawn, and its share of a narrow window. */
const MAX_WIDTH = 468;
const SHARE_OF_WINDOW = 0.42;

export type LaunchDot = {
  key: number;
  left: number;
  top: number;
  size: number;
  /** Where the dot starts, from its place. */
  dx: number;
  dy: number;
  /** When it sets off, and when the glint reaches it. */
  delay: number;
  glint: number;
};

/** The same scatter on every launch, so the name always forms the same way. */
function scatter(index: number, salt: number): number {
  const value = Math.sin(index * 12.9898 + salt * 78.233) * 43758.5453;
  return value - Math.floor(value);
}

export function nameWidth(windowWidth: number): number {
  return Math.round(Math.min(MAX_WIDTH, Math.max(0, windowWidth) * SHARE_OF_WINDOW));
}

/** Every dot of the name at `width`, where it starts from and when it moves. */
export function plan(width: number): { height: number; dots: LaunchDot[] } {
  const scale = width / NAME_BOX.width;
  const reachScale = Math.max(0.4, width / MAX_WIDTH);
  const dots = NAME_DOTS.map(([x, y, r], index) => {
    const size = Math.max(1.4, r * 2 * scale);
    // Each dot comes up where it belongs, from a few pixels away: the name
    // is what is seen, not a flight.
    const angle = scatter(index, 1) * Math.PI * 2;
    const reach = (6 + scatter(index, 2) * 12) * reachScale;
    const across = x / NAME_BOX.width;
    return {
      key: index,
      left: x * scale - size / 2,
      top: y * scale - size / 2,
      size,
      dx: Math.cos(angle) * reach,
      dy: Math.sin(angle) * reach,
      delay: Math.round(40 + across * 220 + scatter(index, 3) * 60),
      glint: Math.round(LAUNCH.glintFrom + across * LAUNCH.glintSpan),
    };
  });
  return { height: NAME_BOX.height * scale, dots };
}

/** Whether the launch plays: it is the person's choice, and it never plays
 *  when they asked for less motion or while a game is in front. */
export function launchWanted(choice: { enabled: boolean; reducedMotion: boolean; game: boolean }) {
  return choice.enabled && !choice.reducedMotion && !choice.game;
}
