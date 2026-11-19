<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  // The window's own controls on Windows, drawn in the band the chrome keeps
  // above the page. They are part of the window, so they move, animate and
  // hide with it; a native popup of their own could not, and stood over other
  // applications while the window was on its way.
  let {
    maximized = false,
    canMaximize = true,
    labels,
  }: {
    maximized?: boolean;
    /** False while the window holds one size, as during onboarding. */
    canMaximize?: boolean;
    labels: { minimize: string; maximize: string; restore: string; close: string };
  } = $props();

  type Appwindow = ReturnType<typeof getCurrentWindow>;
  function act(run: (appWindow: Appwindow) => Promise<void>) {
    try {
      void run(getCurrentWindow()).catch(() => {});
    } catch {
      // Outside the app's own window there is no window to act on.
    }
  }
</script>

<!-- Out of the tab order, as a title bar's buttons are: the keyboard has
     Alt+F4, Win+Up and Win+Down, and Alt+Space for the system menu. -->
<div class="window-caption">
  <button
    type="button"
    class="control"
    tabindex="-1"
    aria-label={labels.minimize}
    title={labels.minimize}
    onclick={() => act((appWindow) => appWindow.minimize())}
  >
    <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M1 6.5h10" /></svg>
  </button>
  {#if canMaximize}
    <button
      type="button"
      class="control"
      tabindex="-1"
      aria-label={maximized ? labels.restore : labels.maximize}
      title={maximized ? labels.restore : labels.maximize}
      onclick={() => act((appWindow) => appWindow.toggleMaximize())}
    >
      {#if maximized}
        <svg viewBox="0 0 12 12" aria-hidden="true"
          ><path d="M1.5 3.5h7v7h-7zM3.5 3.5v-2h7v7h-2" /></svg
        >
      {:else}
        <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M1.5 1.5h9v9h-9z" /></svg>
      {/if}
    </button>
  {/if}
  <button
    type="button"
    class="control close"
    tabindex="-1"
    aria-label={labels.close}
    title={labels.close}
    onclick={() => act((appWindow) => appWindow.close())}
  >
    <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M1.5 1.5l9 9M10.5 1.5l-9 9" /></svg>
  </button>
</div>

<style>
  /* In the window's corner to its last pixel, so a pointer thrown at the
     corner of a maximized window lands on Close. Above a dimmed dialog too:
     the window can always be closed or put away. */
  .window-caption {
    position: fixed;
    top: 0;
    right: 0;
    z-index: 90;
    display: flex;
    height: 36px;
  }

  .control {
    display: grid;
    place-items: center;
    width: 46px;
    height: 36px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-text);
    cursor: default;
    transition: background-color var(--motion-fast) var(--ease-out);
  }

  /* Strokes on pixel centres: one crisp pixel each at 100%. */
  .control svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentcolor;
    stroke-width: 1;
    shape-rendering: crispedges;
  }

  .control.close svg {
    shape-rendering: geometricprecision;
  }

  .control:hover {
    background: var(--color-fill-hover);
  }

  .control:active {
    background: var(--color-fill-pressed);
  }

  .control.close:hover,
  .control.close:active {
    background: var(--color-window-close);
    color: var(--color-on-window-close);
  }

  .control:focus-visible {
    outline: 2px solid var(--color-ring);
    outline-offset: -2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .control {
      transition: none;
    }
  }

  @media (forced-colors: active) {
    .control:hover,
    .control.close:hover {
      background: Highlight;
      color: HighlightText;
    }
  }
</style>
