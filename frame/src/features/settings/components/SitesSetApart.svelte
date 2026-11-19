<script lang="ts">
  import { Cancel01Icon } from "@hugeicons/core-free-icons";
  import * as m from "$shared/i18n/messages";
  import { preferences, siteList } from "$domain/preferences";
  import IconButton from "$shared/ui/IconButton";

  /** The preference that keeps the sites set apart. */
  let { preference }: { preference: "ui.force-dark-except" | "privacy.cookie-banners-except" } =
    $props();

  let stored = $derived(preferences.value(preference));
  let sites = $derived(siteList.sitesOf(stored));
</script>

<div class="exceptions">
  <p class="lead">{sites.length > 0 ? m.site_exceptions_title() : m.site_exceptions_none()}</p>
  {#if sites.length > 0}
    <ul>
      {#each sites as site (site)}
        <li>
          <span class="site">{site}</span>
          <IconButton
            label={m.site_exceptions_remove({ site })}
            icon={Cancel01Icon}
            size={12}
            onclick={() => void preferences.set(preference, siteList.withoutSite(stored, site))}
          />
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .exceptions {
    padding-block: 6px;
    padding-inline: var(--settings-row-inset, 12px);
  }

  .lead {
    font-size: var(--text-label);
    line-height: 1.4;
    color: var(--color-muted);
  }

  ul {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-block-start: 6px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-inline-start: 8px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-control);
  }

  .site {
    font-size: var(--text-label);
  }
</style>
