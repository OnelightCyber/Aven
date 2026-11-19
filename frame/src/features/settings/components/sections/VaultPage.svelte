<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import {
    commands,
    type VaultInputV1,
    type VaultItemV1,
    type VaultKindV1,
  } from "$shared/ipc/bindings";
  import { profiles } from "$domain/profiles";
  import { tabs } from "$domain/tabs";
  import {
    Copy01Icon,
    Delete02Icon,
    Key01Icon,
    Note01Icon,
    PencilEdit02Icon,
    TwoFactorAccessIcon,
    UserIcon,
    ViewIcon,
    ViewOffIcon,
  } from "@hugeicons/core-free-icons";
  import type { IconSvgElement } from "@hugeicons/svelte";
  import Button from "$shared/ui/Button";
  import Field from "$shared/ui/Field";
  import Icon from "$shared/ui/Icon";
  import IconButton from "$shared/ui/IconButton";
  import SearchField from "$shared/ui/SearchField";
  import SegmentedControl from "$shared/ui/SegmentedControl";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import SettingsRow from "$shared/ui/SettingsRow";
  import TextArea from "$shared/ui/TextArea";

  // Sign-ins, secure notes, API keys and two-factor codes, sealed on this
  // computer. Secrets come up only when asked for, one at a time.
  // A private scope keeps nothing: its vault is the regular profile's,
  // opened from a regular window.
  let profile = $derived(
    profiles.inPrivate() || tabs.profile()?.kind === "incognito" ? "" : (tabs.profile()?.id ?? ""),
  );
  let items = $state.raw<VaultItemV1[]>([]);
  let query = $state("");
  let filter = $state<"all" | VaultKindV1>("all");
  let message = $state<{ text: string; tone: "done" | "error" } | null>(null);
  let shown = $state<Record<string, string>>({});
  let codes = $state<Record<string, { code: string; remaining: number }>>({});
  let editing = $state<(VaultInputV1 & { secretShown: boolean }) | null>(null);
  let confirming = $state<string | null>(null);
  let busy = $state(false);

  async function reload() {
    if (!profile) return;
    const result = await commands.vaultList(profile);
    if (result.status === "ok") items = result.data;
    else message = { text: m.vault_failed({ reason: result.error }), tone: "error" };
  }
  $effect(() => {
    // Another profile's secrets are never left shown under this one's.
    shown = {};
    if (profile) void reload();
  });

  // A secret shown goes back into hiding after a while, and as soon as the
  // window is put away: it is never left on screen for whoever comes by.
  const SHOWN_FOR_MS = 30_000;
  $effect(() => {
    if (Object.keys(shown).length === 0) return;
    const hide = () => (shown = {});
    const timer = setTimeout(hide, SHOWN_FOR_MS);
    const away = () => {
      if (document.visibilityState !== "visible") hide();
    };
    document.addEventListener("visibilitychange", away);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", away);
    };
  });

  const kinds = $derived([
    { value: "all", label: m.vault_kind_all() },
    { value: "login", label: m.vault_kind_login() },
    { value: "note", label: m.vault_kind_note() },
    { value: "key", label: m.vault_kind_key() },
    { value: "totp", label: m.vault_kind_totp() },
  ]);
  const labels: Record<VaultKindV1, () => string> = {
    login: m.vault_kind_login,
    note: m.vault_kind_note,
    key: m.vault_kind_key,
    totp: m.vault_kind_totp,
  };
  const marks: Record<VaultKindV1, IconSvgElement> = {
    login: UserIcon,
    note: Note01Icon,
    key: Key01Icon,
    totp: TwoFactorAccessIcon,
  };
  // A six-digit code reads in two halves, as authenticator apps print it.
  const spaced = (code: string) =>
    code.length % 2 === 0
      ? `${code.slice(0, code.length / 2)} ${code.slice(code.length / 2)}`
      : code;
  const whereOf = (item: VaultItemV1) =>
    [item.username, hostOf(item.url) || item.url].filter(Boolean).join(" · ");

  const visible = $derived(
    items.filter((item) => {
      if (filter !== "all" && item.kind !== filter) return false;
      const words = query.trim().toLocaleLowerCase();
      return (
        !words ||
        [item.title, item.url, item.username].join(" ").toLocaleLowerCase().includes(words)
      );
    }),
  );

  // Two-factor codes on screen: one look at the vault for all of them, then
  // counted down here, and looked up again only when the first is about to
  // change. A string, so typing in the search that keeps the same items on
  // screen starts nothing again.
  const codeIds = $derived(
    visible
      .filter((item) => item.has_totp)
      .slice(0, 40)
      .map((item) => item.id)
      .join(","),
  );
  $effect(() => {
    const ids = codeIds ? codeIds.split(",") : [];
    if (!profile || ids.length === 0) {
      codes = {};
      return;
    }
    const owner = profile;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let shown: Record<string, { code: string; remaining: number }> = {};
    const refresh = async () => {
      const result = await commands.vaultCodes(owner, ids);
      if (stopped) return;
      if (result.status === "ok")
        shown = Object.fromEntries(
          result.data.map((entry) => [entry.id, { code: entry.code, remaining: entry.remaining }]),
        );
      codes = shown;
      // With no valid secret on screen there is nothing to count down.
      timer = setTimeout(
        Object.keys(shown).length ? tick : refresh,
        1000 * (Object.keys(shown).length ? 1 : 30),
      );
    };
    const tick = () => {
      if (stopped) return;
      shown = Object.fromEntries(
        Object.entries(shown).map(([id, entry]) => [
          id,
          { code: entry.code, remaining: entry.remaining - 1 },
        ]),
      );
      if (Object.values(shown).some((entry) => entry.remaining <= 0)) {
        void refresh();
        return;
      }
      codes = shown;
      timer = setTimeout(tick, 1000);
    };
    void refresh();
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  });

  async function secretOf(id: string): Promise<string | null> {
    const result = await commands.vaultReveal(profile, id);
    if (result.status === "ok") return result.data;
    message = { text: m.vault_failed({ reason: result.error }), tone: "error" };
    return null;
  }

  async function toggleShown(id: string) {
    if (shown[id] !== undefined) {
      const { [id]: _, ...rest } = shown;
      shown = rest;
      return;
    }
    const secret = await secretOf(id);
    if (secret !== null) shown = { ...shown, [id]: secret };
  }

  // A secret is copied by native, which keeps it out of the clipboard
  // history and takes it off again a little later; anything else, and a
  // secret where that is not possible, is copied the ordinary way.
  async function copy(text: string, done: string, secret = false) {
    try {
      const kept = secret && (await commands.clipboardCopyPrivate(text).catch(() => false));
      if (!kept) await navigator.clipboard.writeText(text);
      message = { text: done, tone: "done" };
    } catch {
      message = { text: m.vault_copy_failed(), tone: "error" };
    }
  }

  async function copySecret(id: string) {
    const secret = await secretOf(id);
    if (secret !== null) await copy(secret, m.vault_copied_secret(), true);
  }

  function startNew(kind: VaultKindV1) {
    editing = {
      id: null,
      kind,
      title: "",
      url: "",
      username: "",
      notes: "",
      secret: "",
      totp: "",
      secretShown: kind !== "login",
    };
  }

  async function startEdit(item: VaultItemV1) {
    const secret = await secretOf(item.id);
    if (secret === null) return;
    editing = {
      id: item.id,
      kind: item.kind,
      title: item.title,
      url: item.url,
      username: item.username,
      notes: item.notes,
      secret,
      totp: null,
      secretShown: item.kind !== "login",
    };
  }

  async function generate() {
    if (!editing) return;
    const result = await commands.vaultGenerate(20, true);
    if (result.status === "ok") editing = { ...editing, secret: result.data, secretShown: true };
  }

  async function saveEditing() {
    if (!editing || busy) return;
    busy = true;
    const { secretShown: _, ...item } = editing;
    const result = await commands.vaultSave(profile, {
      ...item,
      title: item.title.trim() || hostOf(item.url) || labels[item.kind](),
      totp: item.totp ? item.totp : null,
    });
    busy = false;
    if (result.status === "ok") {
      editing = null;
      message = { text: m.vault_saved(), tone: "done" };
      await reload();
    } else message = { text: m.vault_failed({ reason: result.error }), tone: "error" };
  }

  async function remove(id: string) {
    const result = await commands.vaultDelete(profile, id);
    confirming = null;
    if (result.status === "ok") {
      message = { text: m.vault_deleted(), tone: "done" };
      await reload();
    } else message = { text: m.vault_failed({ reason: result.error }), tone: "error" };
  }

  async function importCsv() {
    if (busy) return;
    busy = true;
    message = null;
    const result = await commands.vaultImport(profile);
    busy = false;
    if (result.status === "error") {
      message = { text: m.vault_failed({ reason: result.error }), tone: "error" };
      return;
    }
    if (result.data.cancelled) return;
    message = {
      text: m.vault_imported({
        added: String(result.data.added),
        updated: String(result.data.updated),
        skipped: String(result.data.skipped),
      }),
      tone: "done",
    };
    await reload();
  }

  function hostOf(url: string): string {
    try {
      return new URL(url).host.replace(/^www\./u, "");
    } catch {
      return "";
    }
  }
</script>

<SettingsGroup title={m.vault_import_title()} description={m.vault_import_help()}>
  <SettingsRow title={m.vault_import_row()} description={m.vault_import_how()}>
    <Button
      size="compact"
      variant="primary"
      disabled={busy || !profile}
      onclick={() => void importCsv()}>{m.vault_import_action()}</Button
    >
  </SettingsRow>
</SettingsGroup>

{#if message}<p class="message" data-tone={message.tone} role="status">{message.text}</p>{/if}

{#if editing}
  <SettingsGroup
    title={editing.id ? m.vault_edit_title() : m.vault_new_title({ kind: labels[editing.kind]() })}
  >
    <form
      class="editor"
      data-private
      onsubmit={(event) => {
        event.preventDefault();
        void saveEditing();
      }}
    >
      <Field label={m.vault_field_title()} bind:value={editing.title} />
      {#if editing.kind === "login"}
        <Field label={m.vault_field_url()} placeholder="https://" bind:value={editing.url} />
        <Field label={m.vault_field_username()} autocomplete="off" bind:value={editing.username} />
      {/if}
      {#if editing.kind === "note"}
        <TextArea
          label={m.vault_field_note()}
          rows={6}
          value={editing.secret ?? ""}
          oninput={(event) =>
            editing &&
            (editing = { ...editing, secret: (event.currentTarget as HTMLTextAreaElement).value })}
        />
      {:else}
        <div class="secret-line">
          <Field
            label={editing.kind === "totp"
              ? m.vault_field_totp()
              : editing.kind === "key"
                ? m.vault_field_key()
                : m.vault_field_password()}
            type={editing.secretShown ? "text" : "password"}
            autocomplete="off"
            spellcheck="false"
            value={editing.secret ?? ""}
            oninput={(event) =>
              editing &&
              (editing = { ...editing, secret: (event.currentTarget as HTMLInputElement).value })}
          />
          <Button
            size="compact"
            variant="secondary"
            type="button"
            onclick={() => editing && (editing = { ...editing, secretShown: !editing.secretShown })}
            >{editing.secretShown ? m.vault_hide() : m.vault_show()}</Button
          >
          {#if editing.kind === "login"}<Button
              size="compact"
              variant="secondary"
              type="button"
              onclick={() => void generate()}>{m.vault_generate()}</Button
            >{/if}
        </div>
      {/if}
      {#if editing.kind === "login"}
        <Field
          label={m.vault_field_login_totp()}
          hint={m.vault_field_login_totp_hint()}
          autocomplete="off"
          value={editing.totp ?? ""}
          oninput={(event) =>
            editing &&
            (editing = { ...editing, totp: (event.currentTarget as HTMLInputElement).value })}
        />
        <TextArea label={m.vault_field_notes()} rows={3} bind:value={editing.notes} />
      {/if}
      <div class="actions">
        <Button size="compact" variant="secondary" type="button" onclick={() => (editing = null)}
          >{m.vault_cancel()}</Button
        >
        <Button size="compact" variant="primary" type="submit" disabled={busy}
          >{m.vault_save()}</Button
        >
      </div>
    </form>
  </SettingsGroup>
{/if}

<SettingsGroup
  title={m.vault_items_title({ count: String(items.length) })}
  description={m.vault_footnote()}
>
  <div class="toolbar">
    <div class="search">
      <SearchField
        label={m.vault_search()}
        placeholder={m.vault_search()}
        value={query}
        oninput={(value) => (query = value)}
      />
    </div>
    <SegmentedControl label={m.vault_filter()} options={kinds} bind:value={filter} />
  </div>
  <div class="add">
    <Button size="compact" variant="secondary" onclick={() => startNew("login")}
      >{m.vault_add_login()}</Button
    >
    <Button size="compact" variant="secondary" onclick={() => startNew("totp")}
      >{m.vault_add_totp()}</Button
    >
    <Button size="compact" variant="secondary" onclick={() => startNew("key")}
      >{m.vault_add_key()}</Button
    >
    <Button size="compact" variant="secondary" onclick={() => startNew("note")}
      >{m.vault_add_note()}</Button
    >
  </div>
  {#if visible.length === 0}
    <p class="empty">{items.length === 0 ? m.vault_empty() : m.vault_no_match()}</p>
  {:else}
    <ul class="list">
      {#each visible as item (item.id)}
        <li class="item">
          <span class="plate" aria-hidden="true"><Icon icon={marks[item.kind]} size={16} /></span>
          <div class="main">
            <div class="head">
              <strong class="title">{item.title}</strong>
              <span class="kind">{labels[item.kind]()}</span>
            </div>
            {#if whereOf(item)}<span class="sub" data-private>{whereOf(item)}</span>{/if}
            {#if shown[item.id] !== undefined}<code class="secret" data-private
                >{shown[item.id]}</code
              >{/if}
            {#if codes[item.id]}<span class="code" data-private
                ><code>{spaced(codes[item.id]!.code)}</code><small
                  >{m.vault_code_left({ seconds: String(codes[item.id]!.remaining) })}</small
                ></span
              >{/if}
          </div>
          {#if confirming === item.id}
            <div class="buttons">
              <Button size="compact" variant="danger" onclick={() => void remove(item.id)}
                >{m.vault_delete_confirm()}</Button
              >
              <Button size="compact" variant="ghost" onclick={() => (confirming = null)}
                >{m.vault_cancel()}</Button
              >
            </div>
          {:else}
            <div
              class="buttons"
              role="group"
              aria-label={m.vault_item_actions({ title: item.title })}
            >
              {#if item.username}<IconButton
                  icon={UserIcon}
                  label={m.vault_copy_username()}
                  onclick={() => void copy(item.username, m.vault_copied_username())}
                />{/if}
              {#if codes[item.id]}<IconButton
                  icon={TwoFactorAccessIcon}
                  label={m.vault_copy_code()}
                  onclick={() => void copy(codes[item.id]!.code, m.vault_copied_code(), true)}
                />{/if}
              {#if item.has_secret && item.kind !== "totp"}<IconButton
                  icon={Copy01Icon}
                  label={m.vault_copy_secret()}
                  onclick={() => void copySecret(item.id)}
                /><IconButton
                  icon={shown[item.id] !== undefined ? ViewOffIcon : ViewIcon}
                  label={shown[item.id] !== undefined ? m.vault_hide() : m.vault_show()}
                  active={shown[item.id] !== undefined}
                  onclick={() => void toggleShown(item.id)}
                />{/if}
              <IconButton
                icon={PencilEdit02Icon}
                label={m.vault_edit()}
                onclick={() => void startEdit(item)}
              />
              <IconButton
                icon={Delete02Icon}
                label={m.vault_delete()}
                onclick={() => (confirming = item.id)}
              />
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</SettingsGroup>

<style>
  .toolbar,
  .add {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 12px 16px 0;
  }

  .add {
    padding-block: 10px 12px;
  }

  .search {
    flex: 1;
    min-width: 12rem;
  }

  .message {
    margin: -18px 16px 24px;
    color: var(--color-muted);
    font-size: var(--text-label);
  }

  .message[data-tone="error"] {
    color: var(--color-danger);
  }

  .editor {
    display: grid;
    gap: 12px;
    padding: 14px 16px 16px;
  }

  .secret-line {
    display: flex;
    align-items: flex-end;
    gap: 8px;
  }

  .secret-line > :global(:first-child) {
    flex: 1;
    min-width: 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .empty {
    margin: 0;
    padding: 4px 16px 16px;
    color: var(--color-muted);
    font-size: var(--text-label);
  }

  .list {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .item {
    position: relative;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    min-height: var(--row-page);
    padding: 10px 12px 10px 16px;
  }

  /* The same inset hairline the settings rows draw between themselves. */
  .item::before {
    content: "";
    position: absolute;
    inset-inline: 16px 0;
    top: 0;
    height: 1px;
    background: var(--color-border);
  }

  .plate {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: var(--radius-inset);
    background: var(--color-fill);
    color: var(--color-muted);
  }

  .main {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }

  .title,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    color: var(--color-text);
    font-size: var(--text-page-title);
    font-weight: 500;
  }

  .kind {
    flex: none;
    color: var(--color-faint);
    font-size: var(--text-caption);
  }

  .sub {
    color: var(--color-muted);
    font-size: var(--text-label);
  }

  .secret,
  .code code {
    font-family: var(--font-mono);
    user-select: text;
  }

  .secret {
    overflow-wrap: anywhere;
    color: var(--color-text);
    font-size: var(--text-label);
  }

  .code {
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
  }

  .code code {
    color: var(--color-text);
    font-size: 15px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.06em;
  }

  .code small {
    color: var(--color-faint);
    font-size: var(--text-caption);
    font-variant-numeric: tabular-nums;
  }

  .buttons {
    display: flex;
    align-items: center;
    gap: 2px;
  }
</style>
