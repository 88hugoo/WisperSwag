<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let {
    variant = "secondary",
    size = "md",
    icon,
    iconRight,
    disabled = false,
    loading = false,
    title,
    full = false,
    type = "button",
    onclick,
    children,
  }: {
    variant?: "primary" | "secondary" | "ghost" | "danger" | "outline";
    size?: "sm" | "md" | "lg";
    icon?: string;
    iconRight?: string;
    disabled?: boolean;
    loading?: boolean;
    title?: string;
    full?: boolean;
    type?: "button" | "submit";
    onclick?: (e: MouseEvent) => void;
    children?: Snippet;
  } = $props();
</script>

<button
  {type}
  class="btn {variant} {size}"
  class:full
  class:icon-only={!children}
  disabled={disabled || loading}
  {title}
  aria-label={title}
  {onclick}
>
  {#if loading}
    <span class="spin" aria-hidden="true"></span>
  {:else if icon}
    <Icon name={icon} size={size === "sm" ? 14 : 16} />
  {/if}
  {#if children}<span class="label">{@render children()}</span>{/if}
  {#if iconRight}<Icon name={iconRight} size={14} />{/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: 32px;
    padding: 0 13px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    font-size: var(--text-sm);
    font-weight: 550;
    letter-spacing: -0.005em;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background var(--fast) var(--ease),
      border-color var(--fast) var(--ease),
      opacity var(--fast) var(--ease),
      transform var(--fast) var(--ease);
  }
  .btn:active:not(:disabled) {
    transform: translateY(0.5px) scale(0.99);
  }
  .btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .sm {
    height: 28px;
    padding: 0 10px;
    font-size: var(--text-xs);
    gap: 6px;
  }
  .lg {
    height: 40px;
    padding: 0 18px;
    font-size: var(--text-md);
    border-radius: var(--radius);
  }
  .icon-only {
    padding: 0;
    width: 32px;
  }
  .icon-only.sm {
    width: 28px;
  }
  .full {
    width: 100%;
  }
  .primary {
    background: var(--accent);
    color: var(--accent-fg);
  }
  .primary:hover:not(:disabled) {
    opacity: 0.88;
  }
  .secondary {
    background: var(--surface-3);
    color: var(--text);
    border-color: var(--border);
  }
  .secondary:hover:not(:disabled) {
    border-color: var(--border-strong);
    background: var(--active);
  }
  .outline {
    background: transparent;
    border-color: var(--border-strong);
    color: var(--text);
  }
  .outline:hover:not(:disabled) {
    background: var(--hover);
  }
  .ghost {
    background: transparent;
    color: var(--text-2);
  }
  .ghost:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .danger {
    background: transparent;
    color: var(--text);
    border-color: var(--border-strong);
  }
  .danger:hover:not(:disabled) {
    background: var(--text);
    color: var(--bg);
  }
  .label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .spin {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    border: 1.6px solid currentColor;
    border-right-color: transparent;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
