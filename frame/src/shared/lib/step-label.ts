import * as m from "$shared/i18n/messages";

/**
 * A step's label in the person's language. Native names each step in
 * English in a few fixed forms, such as `Click "Send"` or
 * `Type “Paris” into “City”`, and keeps what it quotes word for word: the
 * page's own names stay as the page wrote them, the verbs around them are
 * translated. A label of another form is shown as it came.
 */
export type LabelPart = { text: string; named: boolean };

const QUOTED = /[“"]([^”"]*)[”"]/gu;
const FIRST = "\u0001";
const SECOND = "\u0002";

type Form = (named: { a: string; b: string }) => string;
const FORMS: Record<string, Form> = {
  "Click {0}": (p) => m.assist_step_click(p),
  Click: () => m.assist_step_click_bare(),
  "Open {0}": (p) => m.assist_step_open(p),
  Open: () => m.assist_step_open_bare(),
  "Expand {0}": (p) => m.assist_step_expand(p),
  Expand: () => m.assist_step_expand_bare(),
  "Check {0}": (p) => m.assist_step_check(p),
  Check: () => m.assist_step_check_bare(),
  "Uncheck {0}": (p) => m.assist_step_uncheck(p),
  Uncheck: () => m.assist_step_uncheck_bare(),
  "Type into {0}": (p) => m.assist_step_type_into(p),
  "Type {0} into {1}": (p) => m.assist_step_type_value_into(p),
  "Type {0}": (p) => m.assist_step_type_value(p),
  Type: () => m.assist_step_type_bare(),
  "Choose {0} in {1}": (p) => m.assist_step_choose(p),
  Choose: () => m.assist_step_choose_bare(),
  "Press Return in {0}": (p) => m.assist_step_submit(p),
  "Press Return": () => m.assist_step_submit_bare(),
  "Scroll to {0}": (p) => m.assist_step_scroll_to(p),
  "Scroll to": () => m.assist_step_scroll_bare(),
  "Scroll up": () => m.assist_step_scroll_up(),
  "Scroll down": () => m.assist_step_scroll_down(),
};

/** The label's own words and what it quotes, as native wrote it. */
function rawParts(label: string): LabelPart[] {
  const parts: LabelPart[] = [];
  let at = 0;
  for (const match of label.matchAll(QUOTED)) {
    if (match.index > at) parts.push({ text: label.slice(at, match.index), named: false });
    if (match[1]) parts.push({ text: match[1], named: true });
    at = match.index + match[0].length;
  }
  if (at < label.length) parts.push({ text: label.slice(at), named: false });
  return parts;
}

/** The translated text with its two slots, and what goes in them. */
function translated(label: string): { text: string; named: string[] } | null {
  const host = /^Go to (\S+)$/u.exec(label);
  if (host) return { text: m.assist_step_go_to({ a: FIRST }), named: [host[1]!] };
  const parts = rawParts(label);
  const named = parts.filter((part) => part.named).map((part) => part.text);
  let slot = 0;
  const skeleton = parts
    .map((part) => (part.named ? `{${slot++}}` : part.text))
    .join("")
    .trim();
  const form = FORMS[skeleton];
  return form ? { text: form({ a: FIRST, b: SECOND }), named } : null;
}

/** The label as one line of text, with the quotes of the person's language. */
export function stepText(label: string): string {
  const found = translated(label);
  if (!found) return label;
  return found.text
    .replaceAll(FIRST, found.named[0] ?? "")
    .replaceAll(SECOND, found.named[1] ?? "");
}

const OPENS = /\s*[«“„「『‘"]\s*$/u;
const CLOSES = /^\s*[»”“」』’"]/u;

/**
 * The label's words with what it quotes apart, for a row that sets the
 * page's own names off on their own. The quotes around them are left out:
 * the setting-off shows them.
 */
export function stepParts(label: string): LabelPart[] {
  // A site is named as an address, not quoted as a page's own words.
  if (/^Go to \S+$/u.test(label)) return [{ text: stepText(label), named: false }];
  const found = translated(label);
  if (!found) return rawParts(label);
  // The text cut at its two slot marks, each mark kept as a piece of its own.
  const pieces: string[] = [];
  let run = "";
  for (const char of found.text) {
    if (char !== FIRST && char !== SECOND) {
      run += char;
      continue;
    }
    if (run) pieces.push(run);
    pieces.push(char);
    run = "";
  }
  if (run) pieces.push(run);
  const parts: LabelPart[] = [];
  for (const piece of pieces) {
    if (piece === FIRST || piece === SECOND) {
      const text = found.named[piece === FIRST ? 0 : 1] ?? "";
      const before = parts.at(-1);
      if (before && !before.named)
        before.text = before.text.replace(OPENS, " ").replace(/^ $/u, "");
      parts.push({ text, named: true });
    } else if (piece) {
      const after = parts.at(-1)?.named ? piece.replace(CLOSES, "") : piece;
      if (after) parts.push({ text: after, named: false });
    }
  }
  return parts.filter((part) => part.named || part.text !== "");
}
