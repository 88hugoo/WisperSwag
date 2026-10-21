<script lang="ts">
  let { value = 0, indeterminate = false }: { value?: number; indeterminate?: boolean } = $props();
</script>

<div class="track" class:ind={indeterminate}>
  <div class="bar" style:width="{indeterminate ? 30 : Math.max(0, Math.min(1, value)) * 100}%"></div>
</div>

<style>
  .track {
    height: 4px;
    border-radius: 4px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar {
    height: 100%;
    border-radius: 4px;
    background: var(--text);
    transition: width 200ms var(--ease);
  }
  .ind .bar {
    animation: slide 1.1s var(--ease) infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
</style>
