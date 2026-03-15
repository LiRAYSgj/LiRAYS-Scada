<script lang="ts">
  import { Button } from "$lib/components/Button";
  import { snackbarStore } from "$lib/stores/snackbar";
  import { themeStore } from "$lib/stores/theme";
  import { onDestroy, tick } from "svelte";

  const DURATION_MS = 5000;

  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  let popoverEl: HTMLElement | null = null;

  $: themeClass = $themeStore === null ? "theme-light" : $themeStore === "dark" ? "theme-dark" : "theme-light";

  /** Move snackbar to end of body so it is last in DOM and stacks on top when shown. */
  function usePortalBody(node: HTMLElement): { destroy: () => void } {
    const parent = node.parentNode;
    if (parent && parent !== document.body) {
      parent.removeChild(node);
      document.body.appendChild(node);
    }
    return {
      destroy() {
        if (node.parentNode === document.body) {
          document.body.removeChild(node);
        }
      },
    };
  }

  $: entry = $snackbarStore;
  $: durationMs = entry?.duration ?? DURATION_MS;

  function clearTimer(): void {
    if (timeoutId != null) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }
  }

  function dismiss(): void {
    clearTimer();
    snackbarStore.hide();
    popoverEl?.hidePopover?.();
  }

  $: if (entry) {
    clearTimer();
    timeoutId = setTimeout(() => {
      timeoutId = null;
      snackbarStore.hide();
      popoverEl?.hidePopover?.();
    }, durationMs);
    void tick().then(() => {
      requestAnimationFrame(() => {
        popoverEl?.showPopover?.();
      });
    });
  } else {
    popoverEl?.hidePopover?.();
  }

  onDestroy(() => {
    clearTimer();
    popoverEl?.hidePopover?.();
  });
</script>

<div
  bind:this={popoverEl}
  class="snackbar-popover {themeClass}"
  popover="manual"
  aria-label="Notification"
  role="status"
  use:usePortalBody
>
  {#if entry}
    <div
      class="snackbar snackbar--{entry.type}"
      role="alert"
      aria-live="polite"
    >
      <div
        class="snackbar__line"
        style="animation-duration: {durationMs}ms"
        aria-hidden="true"
      ></div>
      <div class="snackbar__body">
        <p class="snackbar__message">{entry.message}</p>
        <Button
          variant="outline-accent"
          label="Dismiss"
          title="Dismiss"
          ariaLabel="Dismiss"
          class="snackbar__dismiss"
          onclick={dismiss}
        />
      </div>
    </div>
  {/if}
</div>

<style>
  /* Popover: top-layer but only the size of the card, so it doesn't block the dialog underneath */
  .snackbar-popover {
    position: fixed;
    top: 1rem;
    right: 1rem;
    left: auto;
    bottom: auto;
    margin: 0;
    padding: 0;
    border: none;
    background: transparent;
    width: max-content;
    max-width: min(420px, calc(100vw - 2rem));
    height: fit-content;
    min-height: 0;
    box-sizing: border-box;
  }

  /* Define theme vars so they're in scope in top layer */
  .snackbar-popover.theme-light {
    --snackbar-bg-base: #ffffff;
    --text-primary: #0f172a;
    --text-secondary: #334155;
    --text-muted: #64748b;
    --bg-hover: #e6ebf2;
  }

  .snackbar-popover.theme-dark {
    --snackbar-bg-base: #161c23;
    --text-primary: #e2e8f0;
    --text-secondary: #cbd5e1;
    --text-muted: #94a3b8;
    --bg-hover: #243041;
  }

  .snackbar {
    position: relative;
    width: 100%;
    min-width: 280px;
    max-width: 420px;
    padding: 0;
    border-radius: 0.5rem;
    box-shadow: 0 10px 25px -5px rgb(0 0 0 / 0.2), 0 4px 6px -4px rgb(0 0 0 / 0.1);
    overflow: hidden;
    background: var(--snackbar-bg);
    border: 1px solid var(--snackbar-border);
    color: var(--snackbar-text);
    box-sizing: border-box;
  }

  .snackbar__line {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: var(--snackbar-line);
    transform-origin: left;
    animation: snackbar-shrink linear forwards;
    border-radius: 0.5rem 0.5rem 0 0;
  }

  .snackbar__body {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    padding-top: 0.5rem;
    background: var(--snackbar-bg);
  }

  .snackbar__message {
    margin: 0;
    font-size: 0.875rem;
    line-height: 1.4;
    flex: 1;
    min-width: 0;
  }

  .snackbar__dismiss {
    flex-shrink: 0;
    cursor: pointer;
    border-radius: 0.375rem;
    border: 1px solid var(--snackbar-dismiss-border);
    padding: 0.125rem 0.5rem;
    font-size: 11px;
    background: var(--snackbar-dismiss-bg);
    color: var(--snackbar-dismiss-text);
  }

  .snackbar__dismiss:hover {
    background: var(--snackbar-dismiss-hover);
  }

  /* Theme-aligned: mix accent with --snackbar-bg-base (no self-reference so var() resolves) */
  .snackbar--success {
    --snackbar-line: #22c55e;
    --snackbar-bg: color-mix(in srgb, #22c55e 18%, var(--snackbar-bg-base));
    --snackbar-border: color-mix(in srgb, #22c55e 50%, var(--text-muted));
    --snackbar-text: var(--text-primary);
    --snackbar-dismiss-border: color-mix(in srgb, #22c55e 45%, var(--text-muted));
    --snackbar-dismiss-bg: color-mix(in srgb, #22c55e 12%, var(--snackbar-bg-base));
    --snackbar-dismiss-text: var(--text-secondary);
    --snackbar-dismiss-hover: var(--bg-hover);
  }

  .snackbar--warning {
    --snackbar-line: #eab308;
    --snackbar-bg: color-mix(in srgb, #eab308 18%, var(--snackbar-bg-base));
    --snackbar-border: color-mix(in srgb, #eab308 50%, var(--text-muted));
    --snackbar-text: var(--text-primary);
    --snackbar-dismiss-border: color-mix(in srgb, #eab308 45%, var(--text-muted));
    --snackbar-dismiss-bg: color-mix(in srgb, #eab308 12%, var(--snackbar-bg-base));
    --snackbar-dismiss-text: var(--text-secondary);
    --snackbar-dismiss-hover: var(--bg-hover);
  }

  .snackbar--error {
    --snackbar-line: #ef4444;
    --snackbar-bg: color-mix(in srgb, #ef4444 18%, var(--snackbar-bg-base));
    --snackbar-border: color-mix(in srgb, #ef4444 50%, var(--text-muted));
    --snackbar-text: var(--text-primary);
    --snackbar-dismiss-border: color-mix(in srgb, #ef4444 45%, var(--text-muted));
    --snackbar-dismiss-bg: color-mix(in srgb, #ef4444 12%, var(--snackbar-bg-base));
    --snackbar-dismiss-text: var(--text-secondary);
    --snackbar-dismiss-hover: var(--bg-hover);
  }

  @keyframes snackbar-shrink {
    from {
      transform: scaleX(1);
    }
    to {
      transform: scaleX(0);
    }
  }
</style>
