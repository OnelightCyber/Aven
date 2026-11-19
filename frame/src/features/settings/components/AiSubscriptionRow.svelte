<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$shared/i18n/messages";
  import { commands, type WorkLocalServerV1 } from "$shared/ipc/bindings";
  import type { ModelsSession } from "$domain/ai";
  import Button from "$shared/ui/Button";
  import SettingsGroup from "$shared/ui/SettingsGroup";

  // Claude Code and Codex, through the bridge Aven runs on this computer:
  // the person's subscription instead of an API key. Shown first, in one
  // click, because it is the simplest way in for someone who already uses them.
  let { session }: { session: ModelsSession } = $props();

  let bridge = $state.raw<WorkLocalServerV1 | null>(null);
  let looked = $state(false);
  let message = $state<string | null>(null);

  onMount(() => {
    let live = true;
    void Promise.resolve()
      .then(() => commands.workLocalServers())
      .then((found) => {
        if (live) bridge = found.find((server) => server.kind === "subscriptions") ?? null;
      })
      .catch(() => {})
      .finally(() => {
        if (live) looked = true;
      });
    return () => (live = false);
  });

  const base = $derived(
    session.models?.providers.find((row) => row.provider === "compatible")?.base ?? null,
  );
  const active = $derived(!!bridge && base === bridge.base);
  const busy = $derived(session.busy === "endpoint");

  // An address saved before the bridge asked for its secret points at the
  // same port without it: it is brought up to date once, on its own.
  $effect(() => {
    if (bridge && base && base !== bridge.base && base.startsWith("http://127.0.0.1:11500/"))
      void session.setEndpoint(bridge.base);
  });

  async function use(on: boolean) {
    message = null;
    if (!(await session.setEndpoint(on && bridge ? bridge.base : null)))
      message = m.ai_subscription_failed();
  }
</script>

<SettingsGroup title={m.ai_subscription_title()} description={m.ai_subscription_desc()}>
  <div class="row">
    <p class="status">
      {#if !looked}{m.ai_subscription_looking()}
      {:else if active}{m.ai_subscription_active()}
      {:else if bridge}{m.ai_subscription_found()}
      {:else}{m.ai_subscription_missing()}{/if}
    </p>
    {#if bridge}<Button
        size="compact"
        variant={active ? "secondary" : "primary"}
        disabled={busy}
        onclick={() => void use(!active)}
        >{active ? m.ai_subscription_stop() : m.ai_subscription_use()}</Button
      >{/if}
  </div>
  {#if message}<p class="message" role="alert">{message}</p>{/if}
</SettingsGroup>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    box-sizing: border-box;
    min-height: var(--row-page);
    padding: 12px 18px 12px 14px;
  }

  .status {
    flex: 1;
    margin: 0;
  }

  .message {
    margin: 0;
    padding: 0 18px 12px 14px;
  }
</style>
