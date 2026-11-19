import {
  commands,
  type AgentConsentAnswerV1,
  type AgentConsentRequestV1,
} from "$shared/ipc/bindings";

/**
 * What an agent outside Aven asks to do to a page, and whether one is acting
 * now. Native raises both and decides everything: the interface shows them
 * and sends the person's answer back.
 */
let request = $state<AgentConsentRequestV1 | null>(null);
let working = $state<string | null>(null);
let watching = false;

function requestOf(detail: unknown): AgentConsentRequestV1 | null {
  if (typeof detail !== "object" || detail === null) return null;
  const { id, site, action, weighty } = detail as Record<string, unknown>;
  return typeof id === "number" &&
    typeof site === "string" &&
    typeof action === "string" &&
    typeof weighty === "boolean"
    ? { id, site, action, weighty }
    : null;
}

function watch() {
  if (watching || typeof window === "undefined") return;
  watching = true;
  window.addEventListener("aven:agent-consent", (event) => {
    request = requestOf((event as CustomEvent<unknown>).detail);
  });
  window.addEventListener("aven:agent-working", (event) => {
    const detail = (event as CustomEvent<unknown>).detail;
    working = typeof detail === "string" && detail !== "" ? detail : null;
  });
  try {
    // A request raised before the interface was up is asked for once.
    void commands
      .agentConsentPending()
      .then((pending) => {
        if (request === null) request = pending;
      })
      .catch(() => {});
  } catch {
    // Outside the app there is no native side to ask.
  }
}

/** The request waiting for the person's answer. */
export function agentRequest(): AgentConsentRequestV1 | null {
  watch();
  return request;
}

/** The site an outside agent is acting on right now. */
export function agentWorking(): string | null {
  watch();
  return working;
}

/** Sends the person's answer. The card leaves when native says so. */
export async function answer(id: number, choice: AgentConsentAnswerV1): Promise<boolean> {
  try {
    return await commands.agentConsentAnswer(id, choice);
  } catch {
    return false;
  }
}

/** Takes back every standing yes and refuses what is waiting. */
export async function stop(): Promise<boolean> {
  try {
    return await commands.agentConsentStop();
  } catch {
    return false;
  }
}
