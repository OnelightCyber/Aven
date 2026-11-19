<script lang="ts" module>
  import { SvelteMap } from "svelte/reactivity";

  // The level each tab was set to, for as long as the window lives. Native
  // keeps its own copy and applies it; this is only what the slider shows.
  const levels = new SvelteMap<string, number>();
</script>

<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { commands } from "$shared/ipc/bindings";
  import { preferences, siteList } from "$domain/preferences";
  import Slider from "$shared/ui/Slider";
  import Switch from "$shared/ui/Switch";

  /** The page's address and its tab. */
  let { url, tab }: { url: string | null; tab: string | null } = $props();

  let site = $derived(siteList.siteOf(url));
  let darkOn = $derived(preferences.value("ui.force-dark") === "true");
  let refusalOn = $derived(preferences.value("privacy.cookie-banners") !== "false");
  let darkHere = $derived(!siteList.covers(preferences.value("ui.force-dark-except"), site));
  let refusalHere = $derived(
    !siteList.covers(preferences.value("privacy.cookie-banners-except"), site),
  );
  let level = $derived(tab ? (levels.get(tab) ?? 100) : 100);

  // A site set apart goes on the list; applying here again takes it off.
  function apply(key: "ui.force-dark-except" | "privacy.cookie-banners-except", here: boolean) {
    const stored = preferences.value(key);
    void preferences.set(
      key,
      here ? siteList.withoutSite(stored, site) : siteList.withSite(stored, site),
    );
  }

  function volume(next: number) {
    if (!tab) return;
    const owner = tab;
    const before = levels.get(owner) ?? 100;
    const percent = Math.min(100, Math.max(0, Math.round(next)));
    levels.set(owner, percent);
    // Native refused, or could not be reached: the slider goes back to the
    // level the page still plays at, unless the person moved it since.
    const undo = () => {
      if (levels.get(owner) === percent) levels.set(owner, before);
    };
    void commands.tabVolumeSet(owner, percent).then((applied) => {
      if (!applied) undo();
    }, undo);
  }
</script>

{#if site}
  <div class="tuning" data-tray-tools>
    {#if tab}
      <div class="row">
        <span class="name">{m.site_volume()}</span>
        <span class="rail">
          <Slider
            label={m.site_volume()}
            value={level}
            slim
            format={(current) => `${current}%`}
            onchange={volume}
          />
        </span>
      </div>
    {/if}
    {#if darkOn}
      <div class="row">
        <span class="name">{m.site_dark_here()}</span>
        <Switch
          label={m.site_dark_here()}
          labelHidden
          checked={darkHere}
          onchange={(checked) => apply("ui.force-dark-except", checked)}
        />
      </div>
    {/if}
    {#if refusalOn}
      <div class="row">
        <span class="name">{m.site_cookies_here()}</span>
        <Switch
          label={m.site_cookies_here()}
          labelHidden
          checked={refusalHere}
          onchange={(checked) => apply("privacy.cookie-banners-except", checked)}
        />
      </div>
    {/if}
  </div>
{/if}

<style>
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

  .rail {
    flex: 0 0 46%;
    min-inline-size: 0;
  }
</style>
