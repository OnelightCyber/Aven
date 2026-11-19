/** The wordmark's letters as one path, in the coordinates of its own file
 *  (`public/aven-wordmark.svg`), so the page can cut them out of its
 *  ground and fill them from one vector drawn in the same pass. */
export const WORDMARK = {
  /** The file's view box. */
  x: 0,
  y: 0,
  width: 585,
  height: 84,
  /** The scale the file draws its letters at. */
  scaleX: 1,
  scaleY: 1,
  d: "M90.5 77L121.45 7L133.55 7L164.5 77L152.47 77L127.5 20.52L102.53 77ZM109.04 49L134.09 49L134.09 60L109.04 60ZM208.5 7L220.64 7L244.5 58.07L268.36 7L280.5 7L247.8 77L241.2 77ZM324.5 7L335.5 7L335.5 47.5L324.5 47.5ZM335.5 7L382.5 7L382.5 18L335.5 18ZM335.5 36.5L374.5 36.5L374.5 47.5L335.5 47.5ZM334.4 66L382.5 66L382.5 77L324.5 77ZM426.5 7L437.5 7L437.5 77L426.5 77ZM426.5 7L440.69 7L494.5 77L480.31 77ZM483.5 7L494.5 7L494.5 75.14L483.5 75.14Z",
} as const;
