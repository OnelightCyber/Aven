<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { commands } from "$shared/ipc/bindings";
  import { preferences } from "$domain/preferences";
  import Button from "$shared/ui/Button";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import SettingsRow from "$shared/ui/SettingsRow";
  import PreferenceSwitch from "./PreferenceSwitch.svelte";

  let on = $derived(preferences.value("ai.mcp-server") === "true");
  // The address carries a secret: it is copied, never shown.
  let address = $state<string | null>(null);
  let copied = $state(false);

  // The server takes a moment to listen after the switch: asked a few times,
  // then left alone. Native decides whether it runs; this only reads it.
  $effect(() => {
    if (!on) {
      address = null;
      return;
    }
    let live = true;
    let tries = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const look = async () => {
      const found = await commands.mcpServerAddress().catch(() => null);
      if (!live) return;
      address = found;
      if (!found && ++tries < 8) timer = setTimeout(() => void look(), 700);
    };
    void look();
    return () => {
      live = false;
      clearTimeout(timer);
    };
  });

  async function copy() {
    if (!address) return;
    try {
      // The secret it carries stays out of the clipboard history and the
      // cloud, as a vault secret does; where Windows cannot do that, it is
      // copied the ordinary way.
      const kept = await commands.clipboardCopyPrivate(address).catch(() => false);
      if (!kept) await navigator.clipboard.writeText(address);
      copied = true;
      setTimeout(() => (copied = false), 1600);
    } catch {
      copied = false;
    }
  }
</script>

<SettingsGroup title={m.mcp_group()}>
  <PreferenceSwitch id="ai.mcp-server" preference="ai.mcp-server" />
  {#if on}
    <SettingsRow
      title={m.mcp_address_title()}
      description={address ? m.mcp_address_help() : m.mcp_address_waiting()}
    >
      <Button variant="secondary" disabled={!address} onclick={() => void copy()}
        >{copied ? m.mcp_address_copied() : m.mcp_address_copy()}</Button
      >
    </SettingsRow>
  {/if}
</SettingsGroup>
