<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { MAX_NAME_CHARS, MAX_SETUPS, setups, type Setup } from "$domain/setups";
  import Button from "$shared/ui/Button";
  import Field from "$shared/ui/Field";
  import SettingsRow from "$shared/ui/SettingsRow";

  let draft = $state("");
  // What is being done: one thing at a time, named so its own button shows it.
  let working = $state<string | null>(null);
  let note = $state<{ text: string; error: boolean } | null>(null);
  let saved = $derived(setups.list());
  let locked = $derived(working !== null || setups.busy());
  // The community's list is fetched when the person asks to see it, and
  // kept only while this page is open.
  let shared = $state<Awaited<ReturnType<typeof setups.gallery>> | null>(null);

  function say(text: string, error = false) {
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

  const save = () =>
    run("save", async () => {
      const outcome = await setups.save(draft);
      if (outcome === "saved") {
        draft = "";
        say(m.setups_saved());
      } else if (outcome === "full") say(m.setups_full({ count: MAX_SETUPS }), true);
      else if (outcome === "failed") say(m.setups_failed(), true);
    });

  const apply = (setup: Setup) =>
    run(`apply:${setup.id}`, async () => {
      if (await setups.apply(setup.id)) say(m.setups_applied());
      else say(m.setups_apply_failed(), true);
    });

  const remove = (setup: Setup) =>
    run(`remove:${setup.id}`, async () => {
      if (!(await setups.remove(setup.id))) say(m.setups_failed(), true);
    });

  const exportFile = (setup: Setup) =>
    run(`export:${setup.id}`, async () => {
      const outcome = await setups.exportFile(setup.id);
      if (outcome === "saved") say(m.setups_exported());
      else if (outcome !== "cancelled") say(m.setups_failed(), true);
    });

  type Taken = Awaited<ReturnType<typeof setups.importFile>>;
  function tell({ outcome, dropped }: Taken) {
    if (outcome === "read") {
      say(dropped > 0 ? m.setups_imported_partly({ count: dropped }) : m.setups_imported());
    } else if (outcome === "full") say(m.setups_full({ count: MAX_SETUPS }), true);
    else if (outcome === "not_a_setup" || outcome === "too_large" || outcome === "unreadable")
      say(m.setups_not_a_setup(), true);
    else if (outcome === "newer") say(m.setups_newer(), true);
    else if (outcome === "empty") say(m.setups_nothing(), true);
    else if (outcome === "offline") say(m.setups_gallery_offline(), true);
    else if (outcome === "failed") say(m.setups_failed(), true);
  }

  const importFile = () =>
    run("import", async () => tell(await setups.importFile(m.setups_imported_name())));

  const browse = () =>
    run("browse", async () => {
      const list = await setups.gallery();
      shared = list.outcome === "listed" ? list : null;
      if (list.outcome === "offline") say(m.setups_gallery_offline(), true);
      else if (list.outcome === "unreadable") say(m.setups_gallery_unreadable(), true);
      else if (list.outcome === "newer") say(m.setups_newer(), true);
    });

  const take = (entry: NonNullable<typeof shared>["entries"][number]) =>
    run(`take:${entry.id}`, async () => tell(await setups.takeShared(entry)));
</script>

<div data-setting="appearance.setups">
  {#each saved as setup (setup.id)}
    {@const current = setups.inUse(setup)}
    <SettingsRow title={setup.name} description={current ? m.setups_in_use() : undefined}>
      <div class="actions">
        <Button
          variant={current ? "secondary" : "primary"}
          pending={working === `apply:${setup.id}`}
          disabled={locked || current}
          aria-label={`${m.assist_style_apply()} ${setup.name}`}
          onclick={() => void apply(setup)}>{m.assist_style_apply()}</Button
        >
        <Button
          variant="ghost"
          pending={working === `export:${setup.id}`}
          disabled={locked}
          aria-label={`${m.general_export_button()} ${setup.name}`}
          onclick={() => void exportFile(setup)}>{m.general_export_button()}</Button
        >
        <Button
          variant="danger"
          pending={working === `remove:${setup.id}`}
          disabled={locked}
          aria-label={`${m.vault_delete()} ${setup.name}`}
          onclick={() => void remove(setup)}>{m.vault_delete()}</Button
        >
      </div>
    </SettingsRow>
  {:else}
    <p class="empty">{m.setups_empty()}</p>
  {/each}
  <SettingsRow title={m.setups_save_title()} description={m.setups_save_help()}>
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <Field
        class="name"
        label={m.field_name()}
        labelHidden
        placeholder={m.setups_name_placeholder()}
        bind:value={draft}
        autocomplete="off"
        spellcheck="false"
        maxlength={MAX_NAME_CHARS}
        disabled={locked}
      />
      <Button
        type="submit"
        pending={working === "save"}
        disabled={!draft.trim() || locked || saved.length >= MAX_SETUPS}>{m.vault_save()}</Button
      >
    </form>
  </SettingsRow>
  <SettingsRow title={m.setups_import_title()} description={m.setups_import_help()}>
    <Button pending={working === "import"} disabled={locked} onclick={() => void importFile()}
      >{m.general_import_button()}</Button
    >
  </SettingsRow>
  <SettingsRow title={m.setups_gallery_title()} description={m.setups_gallery_help()}>
    <Button pending={working === "browse"} disabled={locked} onclick={() => void browse()}
      >{m.setups_gallery_open()}</Button
    >
  </SettingsRow>
  {#if shared}
    {#each shared.entries as entry (entry.id)}
      <SettingsRow
        title={entry.name}
        description={[entry.author && m.setups_gallery_by({ author: entry.author }), entry.about]
          .filter(Boolean)
          .join(" · ")}
      >
        <Button
          pending={working === `take:${entry.id}`}
          disabled={locked || saved.length >= MAX_SETUPS}
          aria-label={`${m.setups_gallery_add()} ${entry.name}`}
          onclick={() => void take(entry)}>{m.setups_gallery_add()}</Button
        >
      </SettingsRow>
    {:else}
      <p class="empty">{m.setups_gallery_empty()}</p>
    {/each}
  {/if}
  {#if note}<p class="result" class:error={note.error} role={note.error ? "alert" : "status"}>
      {note.text}
    </p>{/if}
</div>

<style>
  .actions,
  form {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* The name keeps a readable width beside its button. */
  form > :global(.name) {
    inline-size: 200px;
  }

  .empty,
  .result {
    margin: 12px 18px;
    color: var(--color-muted);
    font-size: var(--text-body);
  }

  .error {
    color: var(--color-danger);
  }
</style>
