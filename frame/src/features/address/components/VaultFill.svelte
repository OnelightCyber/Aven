<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { commands, type VaultItemV1 } from "$shared/ipc/bindings";
  import { profiles } from "$domain/profiles";
  import { tabs } from "$domain/tabs";
  import Button from "$shared/ui/Button";

  /** The page's address, and whether the site menu is open. */
  let { url, open }: { url: string | null; open: boolean } = $props();

  // Native decides where a sign-in may go; this only chooses which to offer.
  const siteOf = (address: string): string => {
    try {
      const parsed = new URL(address);
      return parsed.protocol === "https:" ? parsed.hostname.replace(/^www\./, "") : "";
    } catch {
      return "";
    }
  };

  // A private scope keeps nothing: no vault is offered there.
  let profile = $derived(
    profiles.inPrivate() || tabs.profile()?.kind === "incognito" ? "" : (tabs.profile()?.id ?? ""),
  );
  let site = $derived(url ? siteOf(url) : "");
  let items = $state<VaultItemV1[]>([]);
  let outcome = $state<"" | "filled" | "elsewhere" | "failed" | "saved" | "updated" | "nothing">(
    "",
  );
  let busy = $state(false);
  let offered = $derived(
    site
      ? items
          .filter((item) => item.kind === "login" && item.has_secret && siteOf(item.url) === site)
          .slice(0, 6)
      : [],
  );

  // The vault is read when the menu opens on an encrypted page, not before.
  $effect(() => {
    if (!open || !profile || !site) return;
    let live = true;
    outcome = "";
    void commands.vaultList(profile).then((result) => {
      if (live && result.status === "ok") items = result.data;
    });
    return () => {
      live = false;
    };
  });

  async function fill(id: string) {
    if (busy) return;
    busy = true;
    try {
      const result = await commands.vaultFill(profile, id);
      outcome = result.status === "ok" ? (result.data ? "filled" : "elsewhere") : "failed";
    } catch {
      outcome = "failed";
    } finally {
      busy = false;
    }
  }

  // The page is read once, here, because the person asked: native takes
  // what is typed in its sign-in form and keeps it for this site.
  async function saveTyped() {
    if (busy) return;
    busy = true;
    try {
      const result = await commands.vaultSaveTyped(profile);
      outcome = result.status === "ok" ? result.data : "failed";
      if (result.status === "ok" && result.data !== "nothing") {
        const listed = await commands.vaultList(profile);
        if (listed.status === "ok") items = listed.data;
      }
    } catch {
      outcome = "failed";
    } finally {
      busy = false;
    }
  }
</script>

{#if site && profile}
  <div class="fill" data-tray-tools>
    {#if offered.length > 0}<p class="title">{m.vault_fill_title()}</p>{/if}
    {#each offered as item (item.id)}
      <div class="row">
        <span class="name" data-private>{item.username || item.title}</span>
        <Button variant="ghost" disabled={busy} onclick={() => void fill(item.id)}
          >{m.vault_fill_action()}</Button
        >
      </div>
    {/each}
    <div class="row">
      <span class="name">{m.vault_save_typed()}</span>
      <Button variant="ghost" disabled={busy} onclick={() => void saveTyped()}
        >{m.vault_save_typed_action()}</Button
      >
    </div>
    {#if outcome}
      <p role="status" aria-live="polite" class="outcome">
        {outcome === "filled"
          ? m.vault_fill_done()
          : outcome === "elsewhere"
            ? m.vault_fill_elsewhere()
            : outcome === "saved"
              ? m.vault_save_typed_saved()
              : outcome === "updated"
                ? m.vault_save_typed_updated()
                : outcome === "nothing"
                  ? m.vault_save_typed_nothing()
                  : m.vault_fill_failed()}
      </p>
    {/if}
  </div>
{/if}

<style>
  .title,
  .outcome {
    margin-block: 4px;
    margin-inline: var(--menu-item-inset);
    font-size: var(--text-label);
    line-height: 1.4;
    color: var(--color-muted);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-block-size: var(--menu-row);
    padding-inline: var(--menu-item-inset) calc(var(--menu-item-inset) - 4px);
  }

  .name {
    flex: 1;
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
