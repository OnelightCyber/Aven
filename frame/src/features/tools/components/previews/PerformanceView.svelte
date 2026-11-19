<script lang="ts">
  import { onMount } from "svelte";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
  import { Settings01Icon } from "@hugeicons/core-free-icons";
  import type { ToolHostProps } from "$session/tool-drafts.svelte";
  import { commands, type TaskManagerView, type TaskTabView } from "$shared/ipc/bindings";
  import { preferences } from "$domain/preferences";
  import { tabTitle } from "$shared/lib/display-names";
  import type { MenuEntry } from "$shared/ui/Menu";
  import Button from "$shared/ui/Button";
  import Switch from "$shared/ui/Switch";
  import * as m from "$shared/i18n/messages";
  import ToolFrame from "../ToolFrame.svelte";
  let props: ToolHostProps = $props();

  // Native measures Aven every three seconds and a page on each look; a
  // shorter wait would only read the same numbers again.
  const POLL_MS = 2000;
  // A minute of history for the two traces.
  const HISTORY = 30;

  let view = $state<TaskManagerView | null>(null);
  let failed = $state(false);
  let memoryTrace = $state<number[]>([]);
  let cpuTrace = $state<number[]>([]);
  // A page that is safe to put away goes within a second or two; one still
  // here after this long has said no.
  const REFUSAL_MS = 6000;
  // Tabs the person sent to sleep, and when: their button waits until the
  // tab leaves the list, or the page refuses.
  const asked = new SvelteMap<string, number>();
  // Tabs that refused, so the row says why nothing happened.
  const refused = new SvelteSet<string>();

  async function look() {
    const result = await commands.taskManagerSnapshot();
    const next = result.status === "ok" ? result.data : null;
    if (!next) {
      failed = view === null;
      return;
    }
    failed = false;
    view = next;
    if (next.memory_mb !== null) memoryTrace = [...memoryTrace, next.memory_mb].slice(-HISTORY);
    if (next.cpu_percent !== null) cpuTrace = [...cpuTrace, next.cpu_percent].slice(-HISTORY);
    const live = new Set(next.tabs.map((tab) => tab.id));
    const now = Date.now();
    for (const [id, since] of asked) {
      if (!live.has(id)) asked.delete(id);
      else if (now - since >= REFUSAL_MS) {
        asked.delete(id);
        refused.add(id);
      }
    }
    for (const id of refused) if (!live.has(id)) refused.delete(id);
  }

  onMount(() => {
    let live = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const loop = async () => {
      timer = undefined;
      // Nobody reads a hidden window: each look asks native to measure every
      // page, so it waits until the window is back.
      if (document.visibilityState !== "visible") return;
      try {
        await look();
      } catch {
        if (view === null) failed = true;
      }
      if (live && timer === undefined) timer = setTimeout(loop, POLL_MS);
    };
    const shown = () => {
      if (live && timer === undefined && document.visibilityState === "visible") void loop();
    };
    document.addEventListener("visibilitychange", shown);
    void loop();
    return () => {
      live = false;
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", shown);
    };
  });

  const memoryText = (mb: number) =>
    mb >= 1024
      ? m.resources_memory_gb({
          gb: (mb / 1024).toLocaleString(undefined, { maximumFractionDigits: 1 }),
        })
      : m.resources_memory_mb({ mb: String(Math.round(mb)) });

  const tabMemory = (tab: TaskTabView) =>
    tab.memory_kb === null ? "–" : memoryText(tab.memory_kb / 1024);

  const stateText = (tab: TaskTabView) =>
    tab.state === "shown"
      ? m.perf_state_shown()
      : tab.state === "suspended"
        ? m.perf_state_suspended()
        : m.perf_state_running();

  // The site, then what the tab is doing: joined here, because markup drops
  // the space after the dot.
  const metaText = (tab: TaskTabView) =>
    [
      tab.host,
      asked.has(tab.id)
        ? m.perf_sleep_asked()
        : refused.has(tab.id)
          ? m.perf_sleep_refused()
          : stateText(tab),
    ]
      .filter(Boolean)
      .join(" · ");

  // The trace's points, scaled to its own peak so a quiet minute still reads.
  function trace(values: number[], floor: number) {
    if (values.length < 2) return "";
    const peak = Math.max(floor, ...values);
    return values
      .map((value, index) => {
        const x = (index / (HISTORY - 1)) * 100;
        const y = 28 - (value / peak) * 26;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }

  async function sleep(tab: TaskTabView) {
    refused.delete(tab.id);
    asked.set(tab.id, Date.now());
    const admission = await commands.taskManagerSleep(tab.id);
    if (!admission.accepted) {
      asked.delete(tab.id);
      refused.add(tab.id);
    }
    void look();
  }

  let gameMode = $derived(preferences.value("performance.game-mode") !== "false");

  const entries: MenuEntry[] = [
    { kind: "item", id: "settings", label: m.perf_settings(), icon: Settings01Icon },
  ];
  function act(id: string) {
    if (id === "settings") {
      void commands.runCommand("settings.performance");
      props.onclose();
    }
  }
</script>

<ToolFrame {...props} caption={false} more={entries} onmore={act}>
  <div class="performance">
    {#if failed}<p class="note" role="status">{m.perf_unavailable()}</p>
    {:else if view === null}<p class="note" role="status">{m.perf_measuring()}</p>
    {:else}
      <div class="meters">
        <section class="meter" aria-label={m.perf_memory()}>
          <span class="meter-label">{m.perf_memory()}</span>
          <span class="meter-value"
            >{view.memory_mb === null ? m.perf_measuring() : memoryText(view.memory_mb)}</span
          >
          <svg viewBox="0 0 100 30" preserveAspectRatio="none" aria-hidden="true"
            ><polyline points={trace(memoryTrace, 256)} /></svg
          >
        </section>
        <section class="meter" aria-label={m.perf_cpu()}>
          <span class="meter-label">{m.perf_cpu()}</span>
          <span class="meter-value"
            >{view.cpu_percent === null
              ? m.perf_measuring()
              : m.perf_cpu_value({ percent: String(Math.round(view.cpu_percent)) })}</span
          >
          <svg viewBox="0 0 100 30" preserveAspectRatio="none" aria-hidden="true"
            ><polyline points={trace(cpuTrace, 10)} /></svg
          >
        </section>
      </div>
      <p class="facts">
        {#if view.processes > 0}<span>{m.perf_processes({ count: String(view.processes) })}</span
          >{/if}
        {#if view.game}<span class="fact-on">{m.resources_game()}</span>
        {:else if view.throttled}<span class="fact-on">{m.resources_throttled()}</span>{/if}
      </p>

      <div class="switch-row">
        <Switch
          label={m.pref_performance_game_mode()}
          checked={gameMode}
          onchange={(on) => void preferences.set("performance.game-mode", on ? "true" : "false")}
        />
      </div>

      <h3 class="tabs-title">
        {m.perf_tabs_title()}
        <span class="tabs-summary"
          >{m.perf_tabs_summary({
            live: String(view.tabs.length),
            sleeping: String(view.sleeping),
          })}</span
        >
      </h3>
      {#if view.tabs.length === 0}<p class="note">{m.tool_performance_empty()}</p>
      {:else}<ul class="tabs">
          {#each view.tabs as tab (tab.id)}<li class="tab" data-state={tab.state}>
              <div class="tab-text">
                <span class="tab-title">{tabTitle(tab.title)}</span>
                <span class="tab-meta">{metaText(tab)}</span>
              </div>
              <span class="tab-memory">{tabMemory(tab)}</span>
              {#if tab.can_sleep}<Button
                  variant="ghost"
                  size="compact"
                  disabled={asked.has(tab.id)}
                  aria-label={m.perf_sleep_label({ title: tabTitle(tab.title) })}
                  onclick={() => void sleep(tab)}>{m.perf_sleep()}</Button
                >{/if}
            </li>{/each}
        </ul>{/if}
      <p class="note">{m.perf_sleep_note()}</p>
    {/if}
  </div>
</ToolFrame>

<style>
  .performance {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 4px 12px 16px;
  }

  .meters {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .meter {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 10px 6px;
    border-radius: var(--radius-row);
    background: var(--color-fill);
  }

  .meter-label {
    color: var(--color-label-secondary);
    font-size: var(--text-caption);
  }

  .meter-value {
    color: var(--color-text);
    font-size: 18px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .meter svg {
    width: 100%;
    height: 30px;
    overflow: visible;
  }

  .meter polyline {
    fill: none;
    stroke: var(--color-accent);
    stroke-width: 1.5;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    margin: 0;
    color: var(--color-label-secondary);
    font-size: var(--text-caption);
  }

  .fact-on {
    color: var(--color-success);
  }

  .switch-row {
    padding: 2px 0;
  }

  .tabs-title {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    margin: 4px 0 0;
    color: var(--color-text);
    font-size: 13px;
    font-weight: 600;
  }

  .tabs-summary {
    color: var(--color-label-secondary);
    font-size: var(--text-caption);
    font-weight: 400;
  }

  .tabs {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tab {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 8px;
    padding: 6px 4px 6px 8px;
    border-radius: var(--radius-inset);
  }

  .tab:hover {
    background: var(--color-fill-hover);
  }

  .tab-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .tab-title,
  .tab-meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-title {
    color: var(--color-text);
    font-size: 13px;
  }

  .tab-meta {
    color: var(--color-label-secondary);
    font-size: var(--text-caption);
  }

  .tab[data-state="shown"] .tab-meta {
    color: var(--color-accent);
  }

  .tab-memory {
    color: var(--color-text);
    font-size: var(--text-caption);
    font-variant-numeric: tabular-nums;
  }

  .note {
    margin: 0;
    color: var(--color-label-secondary);
    font-size: var(--text-caption);
    line-height: 1.4;
  }
</style>
