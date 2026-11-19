<script lang="ts">
  import * as tools from "$session/tools.svelte";
  import { keymap } from "$domain/keymap";
  import { preferences } from "$domain/preferences";
  import { profiles } from "$domain/profiles";
  import { tabs } from "$domain/tabs";
  import { acceleratorKeys } from "$shared/lib/accelerator";
  import { IS_MAC } from "$shared/platform";
  import Icon from "$shared/ui/Icon";
  import { ASK_TOOL, toolPresentation } from "../lib/dock-tools";

  /** Down the rail the button is a little smaller, as the avatar is. */
  let { compact = false }: { compact?: boolean } = $props();

  // Offered where the assistant is: on, and not in a private window.
  const shown = $derived(
    preferences.aiEnabled() && !profiles.inPrivate() && tabs.profile()?.kind !== "incognito",
  );
  const open = $derived(tools.activeTool() === ASK_TOOL);
  const view = toolPresentation(ASK_TOOL);
  const keys = $derived.by(() => {
    const accelerator = keymap.all().find((entry) => entry.id === `tool.${ASK_TOOL}`)?.accelerator;
    return accelerator ? acceleratorKeys(accelerator, IS_MAC).join(IS_MAC ? "" : "+") : null;
  });

  function toggle() {
    if (open) tools.close();
    else tools.open(ASK_TOOL);
  }
</script>

{#if shown}
  <button
    type="button"
    class="ask"
    data-compact={compact}
    aria-pressed={open}
    aria-label={view.label()}
    title={keys ? `${view.label()}  ${keys}` : view.label()}
    onclick={toggle}
  >
    <Icon icon={view.icon} size={compact ? 17 : 18} strokeWidth={1.7} />
  </button>
{/if}

<style>
  /* Beside you at the foot of the column, lit the way your own circle is. */
  .ask {
    display: grid;
    flex: none;
    place-items: center;
    inline-size: 40px;
    block-size: 40px;
    margin-inline: 2px;
    border: 0;
    border-radius: var(--radius-capsule);
    background: var(--row-active);
    box-shadow: var(--shine-top);
    color: var(--color-label-secondary);
    cursor: default;
    transition:
      background-color var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out),
      scale var(--motion-slow) var(--ease-spring);
  }

  .ask[data-compact="true"] {
    inline-size: 36px;
    block-size: 36px;
  }

  .ask:hover,
  .ask[aria-pressed="true"] {
    background: var(--row-pressed);
    color: var(--color-text);
  }

  .ask:active {
    scale: 0.94;
    transition-duration: var(--motion-instant);
  }

  .ask:focus-visible {
    outline: 2px solid var(--color-ring);
    outline-offset: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .ask {
      transition: none;
    }

    .ask:active {
      scale: none;
    }
  }
</style>
