<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "./lib/api";
  import { router } from "./lib/router.svelte";
  import { app } from "./lib/store.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Titlebar from "./components/Titlebar.svelte";
  import Toasts from "./components/Toasts.svelte";
  import Home from "./pages/Home.svelte";
  import History from "./pages/History.svelte";
  import Vocabulary from "./pages/Vocabulary.svelte";
  import Snippets from "./pages/Snippets.svelte";
  import Modes from "./pages/Modes.svelte";
  import Apps from "./pages/Apps.svelte";
  import Commands from "./pages/Commands.svelte";
  import Settings from "./pages/settings/Settings.svelte";
  import Welcome from "./pages/Welcome.svelte";

  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      await app.init();
      if (app.boot?.first_run && router.section !== "welcome") router.go("welcome");
    } catch (e) {
      error = errorText(e);
    } finally {
      // La ventana se muestra cuando la interfaz ya está pintada (sin destellos).
      requestAnimationFrame(() => api.frontendReady());
    }
  });

  // Evita el menú contextual del navegador fuera de los campos de texto.
  function oncontextmenu(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (!t.closest("input, textarea")) e.preventDefault();
  }
</script>

<svelte:window {oncontextmenu} />

<div class="shell">
  <Titlebar />
  {#if error}
    <div class="fatal">
      <h2>No se pudo cargar Wispr Swag</h2>
      <p class="muted">{error}</p>
    </div>
  {:else if app.ready}
    {#if router.section === "welcome"}
      <Welcome />
    {:else}
      <div class="body">
        <Sidebar />
        <main class="scroll">
          <div class="page" class:wide={router.section === "settings"}>
            {#key router.section}
              {#if router.section === ""}<Home />
              {:else if router.section === "history"}<History />
              {:else if router.section === "vocabulary"}<Vocabulary />
              {:else if router.section === "snippets"}<Snippets />
              {:else if router.section === "modes"}<Modes />
              {:else if router.section === "apps"}<Apps />
              {:else if router.section === "commands"}<Commands />
              {:else if router.section === "settings"}<Settings />
              {:else}<Home />{/if}
            {/key}
          </div>
        </main>
      </div>
    {/if}
  {/if}
  <Toasts />
</div>

<style>
  .shell {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  main {
    flex: 1;
    min-width: 0;
    background: var(--surface);
    border-top-left-radius: var(--radius-lg);
    border-top: 1px solid var(--border);
    border-left: 1px solid var(--border);
  }
  :global([data-theme="light"]) main {
    background: var(--surface);
  }
  .page {
    max-width: 880px;
    margin: 0 auto;
    padding: 40px 44px 64px;
    animation: enter 220ms var(--ease);
  }
  .page.wide {
    max-width: 1040px;
    padding-top: 28px;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  .fatal {
    margin: auto;
    text-align: center;
  }
  .fatal p {
    margin-top: 8px;
  }
</style>
