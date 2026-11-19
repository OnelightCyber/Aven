<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { efficiency, gameMode, memoryUsed } from "$shared/lib/resources.svelte";
  import * as tools from "$session/tools.svelte";

  // A quiet line at the foot of the sidebar: what Aven uses right now, and a
  // mark while it steps back for a game or under the processor cap. It opens
  // the task manager, and closes it again.
  let memory = $derived(memoryUsed());
  let shown = $derived.by(() => {
    if (memory === null) return null;
    return memory >= 1024
      ? m.resources_memory_gb({
          gb: (memory / 1024).toLocaleString(undefined, { maximumFractionDigits: 1 }),
        })
      : m.resources_memory_mb({ mb: String(memory) });
  });
  let title = $derived.by(() => {
    if (shown === null) return "";
    const parts = [m.resources_label({ memory: shown })];
    if (gameMode()) parts.push(m.resources_game());
    else if (efficiency()) parts.push(m.resources_throttled());
    return parts.join(" · ");
  });
</script>

{#if shown !== null}<button
    type="button"
    class="meter"
    {title}
    aria-label={`${title} · ${m.tool_performance()}`}
    aria-pressed={tools.activeTool() === "performance"}
    onclick={() =>
      tools.activeTool() === "performance" ? tools.close() : tools.open("performance")}
  >
    <span class="dot" data-game={gameMode() || undefined} data-eco={efficiency() || undefined}
    ></span>
    <span class="value">{shown}</span>
  </button>{/if}

<style>
  .meter {
    display: flex;
    align-items: center;
    align-self: flex-start;
    gap: 6px;
    margin: 0 4px;
    padding: 2px 6px;
    border: 0;
    border-radius: var(--radius-inset);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: var(--text-caption);
    font-variant-numeric: tabular-nums;
    opacity: 0.6;
    cursor: default;
  }

  .meter:hover,
  .meter[aria-pressed="true"] {
    background: var(--color-fill-hover);
    opacity: 0.9;
  }

  .meter:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 1px;
    opacity: 1;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentcolor;
    opacity: 0.5;
  }

  .dot[data-eco] {
    background: var(--color-accent);
    opacity: 1;
  }

  .dot[data-game] {
    background: var(--color-success);
    opacity: 1;
  }
</style>
