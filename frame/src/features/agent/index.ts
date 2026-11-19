export { agentRequest, agentWorking } from "./lib/agent-consent.svelte";
// The card is loaded when an outside agent first asks or acts: it is no part
// of startup.
export const loadAgentConsent = () => import("./components/AgentConsent.svelte");
