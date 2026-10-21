<script lang="ts">
  // Medidor de nivel: 24 segmentos monocromos.
  let { db = -90, active = false }: { db?: number; active?: boolean } = $props();
  const level = $derived(Math.max(0, Math.min(1, (db + 60) / 54)));
  const SEGMENTS = 24;
</script>

<div class="meter" class:active aria-label="Nivel de entrada" role="meter" aria-valuenow={Math.round(level * 100)} aria-valuemin={0} aria-valuemax={100}>
  {#each Array(SEGMENTS) as _, i}
    <span class:on={active && i / SEGMENTS < level}></span>
  {/each}
</div>

<style>
  .meter {
    display: flex;
    gap: 3px;
    align-items: center;
    height: 18px;
  }
  span {
    flex: 1;
    height: 100%;
    border-radius: 2px;
    background: var(--surface-3);
    transition: background 60ms linear;
  }
  span.on {
    background: var(--text);
  }
</style>
