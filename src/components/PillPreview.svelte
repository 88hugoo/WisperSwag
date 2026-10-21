<script lang="ts">
  // Vista previa real de la barra: la renderiza el mismo motor nativo que la dibuja en Windows.
  import { api } from "../lib/api";
  import { app } from "../lib/store.svelte";

  let { phase, theme }: { phase: string; theme: "dark" | "light" } = $props();
  let canvas: HTMLCanvasElement | undefined = $state();
  let empty = $state(false);

  $effect(() => {
    // Dependencias: apariencia de la barra.
    const a = app.settings.appearance;
    void [a.pill_size, a.pill_opacity, a.info_level, app.settings.dictation.hotkeys.dictate, app.lang];
    const dpr = window.devicePixelRatio || 1;
    api.pillPreview(phase, theme, dpr).then((p) => {
      if (!canvas) return;
      if (!p.width || !p.rgba) {
        empty = true;
        return;
      }
      empty = false;
      const bin = atob(p.rgba);
      const data = new Uint8ClampedArray(bin.length);
      for (let i = 0; i < bin.length; i++) data[i] = bin.charCodeAt(i);
      canvas.width = p.width;
      canvas.height = p.height;
      canvas.style.width = `${p.width / dpr}px`;
      canvas.style.height = `${p.height / dpr}px`;
      canvas.getContext("2d")?.putImageData(new ImageData(data, p.width, p.height), 0, 0);
    });
  });
</script>

<canvas bind:this={canvas} class:hidden={empty}></canvas>
{#if empty}<div class="fallback"><span></span></div>{/if}

<style>
  canvas {
    display: block;
  }
  .hidden {
    display: none;
  }
  .fallback {
    width: 120px;
    height: 34px;
    border-radius: 999px;
    background: #0e0e10;
    display: grid;
    place-items: center;
  }
  .fallback span {
    width: 60px;
    height: 10px;
    border-radius: 6px;
    background: repeating-linear-gradient(90deg, #f5f5f5 0 3px, transparent 3px 6px);
  }
</style>
