<script lang="ts">
  import { app } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}">
      <Icon name={t.kind === "error" ? "alert" : t.kind === "success" ? "check" : "info"} size={15} />
      <span>{t.text}</span>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 60;
    pointer-events: none;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 520px;
    padding: 9px 16px 9px 13px;
    border-radius: 999px;
    background: var(--text);
    color: var(--bg);
    font-size: var(--text-sm);
    font-weight: 520;
    box-shadow: var(--shadow-lg);
    animation: up var(--normal) var(--ease);
  }
  @keyframes up {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
