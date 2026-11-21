<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "$shared/i18n/messages";
  import * as sync from "$domain/sync";
  import { commands } from "$shared/ipc/bindings";
  import Button from "$shared/ui/Button";
  import Field from "$shared/ui/Field";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import SettingsRow from "$shared/ui/SettingsRow";

  // Nothing is shown until native says this build can sync at all.
  let device = $state<sync.SyncStatus | null>(null);
  let working = $state<string | null>(null);
  let note = $state<{ text: string; error: boolean } | null>(null);
  // The code is held only while it is on screen.
  let shown = $state("");
  let typed = $state("");
  let locked = $derived(working !== null);

  onMount(() => {
    let alive = true;
    void sync.status().then((status) => {
      if (alive) device = status;
    });
    return () => {
      alive = false;
      shown = "";
    };
  });

  function tell(outcome: sync.SyncOutcome) {
    const error = outcome !== "synced";
    const text =
      outcome === "synced"
        ? m.sync_done()
        : outcome === "offline"
          ? m.sync_offline()
          : outcome === "refused"
            ? m.sync_refused()
            : outcome === "unreadable"
              ? m.sync_unreadable()
              : outcome === "newer"
                ? m.sync_newer()
                : outcome === "busy"
                  ? m.sync_busy()
                  : m.setups_failed();
    note = { text, error };
  }

  async function run(tag: string, action: () => Promise<void>) {
    if (working !== null) return;
    working = tag;
    note = null;
    try {
      await action();
    } finally {
      working = null;
    }
  }

  const reveal = (thenSync: boolean) =>
    run("code", async () => {
      const made = await sync.code();
      if (made.outcome !== "synced") return tell(made.outcome);
      shown = made.code;
      if (device) device = { ...device, on: true };
      if (thenSync) tell(await sync.now());
    });

  const join = () =>
    run("join", async () => {
      const outcome = await sync.join(typed);
      if (outcome === "synced") {
        typed = "";
        if (device) device = { ...device, on: true };
      }
      tell(outcome);
    });

  const now = () => run("now", async () => tell(await sync.now()));

  const stop = () =>
    run("stop", async () => {
      if (await sync.stop()) {
        shown = "";
        if (device) device = { ...device, on: false };
      } else note = { text: m.setups_failed(), error: true };
    });

  const copy = () =>
    run("copy", async () => {
      // Kept out of the clipboard's history, like a password.
      const kept = await commands.clipboardCopyPrivate(shown).catch(() => false);
      note = kept
        ? { text: m.assist_copied(), error: false }
        : { text: m.setups_failed(), error: true };
    });
</script>

{#if device?.available}
  <SettingsGroup title={m.sync_title()} description={m.sync_help()}>
    {#if !device.on}
      <SettingsRow title={m.sync_start_title()} description={m.sync_start_help()}>
        <Button pending={working === "code"} disabled={locked} onclick={() => void reveal(true)}
          >{m.sync_start()}</Button
        >
      </SettingsRow>
      <SettingsRow title={m.sync_join_title()} description={m.sync_join_help()}>
        <form
          onsubmit={(event) => {
            event.preventDefault();
            void join();
          }}
        >
          <Field
            class="typed"
            label={m.sync_code_label()}
            labelHidden
            placeholder="XXXX-XXXX-XXXX-XXXX-…"
            bind:value={typed}
            autocomplete="off"
            autocapitalize="characters"
            spellcheck="false"
            maxlength={64}
            disabled={locked}
          />
          <Button type="submit" pending={working === "join"} disabled={!typed.trim() || locked}
            >{m.sync_join()}</Button
          >
        </form>
      </SettingsRow>
    {:else}
      <SettingsRow title={m.sync_on_title()} description={m.sync_on_help()}>
        <Button pending={working === "now"} disabled={locked} onclick={() => void now()}
          >{m.sync_now()}</Button
        >
      </SettingsRow>
      <SettingsRow title={m.sync_show_code_title()} description={m.sync_show_code_help()}>
        {#if shown}
          <div class="shown">
            <code data-private aria-label={m.sync_code_label()}>{shown}</code>
            <Button variant="ghost" disabled={locked} onclick={() => void copy()}
              >{m.vault_copy_secret()}</Button
            >
          </div>
        {:else}
          <Button pending={working === "code"} disabled={locked} onclick={() => void reveal(false)}
            >{m.sync_show_code()}</Button
          >
        {/if}
      </SettingsRow>
      <SettingsRow title={m.sync_stop_title()} description={m.sync_stop_help()}>
        <Button
          variant="danger"
          pending={working === "stop"}
          disabled={locked}
          onclick={() => void stop()}>{m.sync_stop()}</Button
        >
      </SettingsRow>
    {/if}
    {#if note}<p class="result" class:error={note.error} role={note.error ? "alert" : "status"}>
        {note.text}
      </p>{/if}
  </SettingsGroup>
{/if}

<style>
  form,
  .shown {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  form > :global(.typed) {
    inline-size: 240px;
  }

  code {
    font-family: var(--font-mono);
    font-size: var(--text-body);
    letter-spacing: 0.02em;
    user-select: all;
  }

  .result {
    margin: 12px 18px;
    color: var(--color-muted);
    font-size: var(--text-body);
  }

  .error {
    color: var(--color-danger);
  }
</style>
