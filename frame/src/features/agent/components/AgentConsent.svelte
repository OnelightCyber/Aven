<script lang="ts">
  import { Cancel01Icon, Tick02Icon } from "@hugeicons/core-free-icons";
  import * as m from "$shared/i18n/messages";
  import * as attention from "$session/attention.svelte";
  import SidebarCard from "$shared/ui/SidebarCard";
  import { stepText } from "$shared/lib/step-label";
  import { agentRequest, agentWorking, answer, stop } from "../lib/agent-consent.svelte";

  // A card that appears where the pointer already is must not take a click
  // aimed at what it covered.
  const SETTLE_MS = 500;

  let request = $derived(agentRequest());
  let working = $derived(agentWorking());
  let failed = $state(false);
  let shownAt = 0;

  // A waiting request brings the sidebar out: the page cannot answer it.
  $effect(() => {
    if (request === null) return;
    shownAt = performance.now();
    failed = false;
    return attention.hold();
  });

  async function respond(choice: "once" | "site" | "refuse") {
    const asked = request;
    if (asked === null) return;
    if (choice !== "refuse" && performance.now() - shownAt < SETTLE_MS) return;
    failed = !(await answer(asked.id, choice));
  }
</script>

{#if request}
  {#key request.id}
    <div data-aven-agent-consent>
      <SidebarCard
        title={m.agent_consent_title({ site: request.site })}
        detail={failed
          ? m.agent_consent_failed()
          : request.weighty
            ? m.agent_consent_weighty({ action: stepText(request.action) })
            : stepText(request.action)}
        dismissLabel={m.agent_consent_refuse()}
        ondismiss={() => void respond("refuse")}
        actions={[
          { label: m.agent_consent_once(), icon: Tick02Icon, onclick: () => void respond("once") },
          ...(request.weighty
            ? []
            : [
                {
                  label: m.agent_consent_site(),
                  icon: Tick02Icon,
                  onclick: () => void respond("site"),
                },
              ]),
          {
            label: m.agent_consent_refuse(),
            icon: Cancel01Icon,
            onclick: () => void respond("refuse"),
          },
        ]}
      />
    </div>
  {/key}
{:else if working}
  <div data-aven-agent-working>
    <SidebarCard
      title={m.agent_working_title()}
      detail={m.agent_working_detail({ site: working })}
      pending
      actions={[{ label: m.agent_working_stop(), icon: Cancel01Icon, onclick: () => void stop() }]}
    />
  </div>
{/if}
