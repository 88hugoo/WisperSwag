<script lang="ts">
  import { api, errorText } from "../../lib/api";
  import defaults from "../../lib/defaults.json";
  import { app, tr } from "../../lib/store.svelte";
  import type { HotkeySettings } from "../../lib/types";
  import Button from "../../components/Button.svelte";
  import HotkeyField from "../../components/HotkeyField.svelte";
  import Icon from "../../components/Icon.svelte";
  import Input from "../../components/Input.svelte";
  import Row from "../../components/Row.svelte";
  import Section from "../../components/Section.svelte";
  import Segmented from "../../components/Segmented.svelte";
  import Select from "../../components/Select.svelte";
  import Toggle from "../../components/Toggle.svelte";

  const d = $derived(app.settings.dictation);
  const c = $derived(app.settings.writing.cleanup);
  const fallback = defaults.dictation.hotkeys as HotkeySettings;
  let restarting = $state(false);
  let fillerDraft = $state("");

  const shortcuts = $derived<{ key: keyof HotkeySettings; title: string; description: string; clearable: boolean }[]>([
    {
      key: "dictate",
      title: tr("Dictar", "Dictate"),
      description:
        d.activation === "hybrid"
          ? tr("Mantén pulsado mientras hablas, o pulsa una vez para dictar sin manos y otra para terminar.", "Hold while you speak, or tap once for hands-free and again to finish.")
          : d.activation === "toggle"
            ? tr("Pulsa para empezar y otra vez para terminar.", "Press to start and again to finish.")
            : tr("Mantén pulsado mientras hablas.", "Hold while you speak."),
      clearable: false,
    },
    {
      key: "command",
      title: tr("Modo comando (IA)", "Command mode (AI)"),
      description: tr("Da una instrucción por voz: se aplica al texto seleccionado («hazlo más corto») o genera uno nuevo. Requiere la IA.", "Speak an instruction: it's applied to the selected text (“make it shorter”) or generates new text. Requires AI."),
      clearable: true,
    },
    {
      key: "cancel",
      title: tr("Cancelar dictado", "Cancel dictation"),
      description: tr("Solo se intercepta mientras grabas o procesas; el resto del tiempo la tecla funciona con normalidad.", "Only intercepted while recording or processing; otherwise the key works normally."),
      clearable: true,
    },
    {
      key: "paste_last",
      title: tr("Pegar el último dictado", "Paste last dictation"),
      description: tr("Vuelve a insertar la última transcripción donde esté el cursor.", "Inserts the last transcript again at the cursor."),
      clearable: true,
    },
    {
      key: "rewrite_selection",
      title: tr("Reescribir la selección (IA)", "Rewrite selection (AI)"),
      description: tr("Aplica el modo de escritura activo al texto seleccionado en cualquier aplicación.", "Applies the active writing mode to the selected text in any app."),
      clearable: true,
    },
    {
      key: "toggle_bar",
      title: tr("Mostrar u ocultar la barra", "Show or hide the bar"),
      description: tr("Oculta la barra en reposo; los atajos siguen funcionando.", "Hides the idle bar; shortcuts keep working."),
      clearable: true,
    },
  ]);

  async function restartHotkeys() {
    restarting = true;
    try {
      await api.hotkeysRestart();
      await app.reload();
      app.toast(tr("Atajos reiniciados", "Shortcuts restarted"), "success");
    } catch (e) {
      app.toast(errorText(e), "error");
    } finally {
      restarting = false;
    }
  }

  function addFiller() {
    const words = fillerDraft.split(",").map((w) => w.trim().toLowerCase()).filter(Boolean);
    const set = new Set(c.fillers);
    for (const w of words) set.add(w);
    app.settings.writing.cleanup.fillers = [...set];
    fillerDraft = "";
  }
</script>

<h2 class="h">{tr("Dictado y atajos", "Dictation & shortcuts")}</h2>

{#if app.boot && !app.boot.hotkeys_available}
  <div class="banner">
    <Icon name="alert" size={16} />
    <div>
      <strong>{tr("Los atajos globales no están activos", "Global shortcuts aren't active")}</strong>
      <p>{app.boot.hotkey_error ?? tr("No se pudo instalar el gancho de teclado.", "The keyboard hook couldn't be installed.")} {tr("Mientras tanto puedes dictar desde la barra o el icono de la bandeja.", "Meanwhile you can dictate from the bar or the tray icon.")}</p>
    </div>
    <Button size="sm" icon="refresh" loading={restarting} onclick={restartHotkeys}>{tr("Reintentar", "Retry")}</Button>
  </div>
{/if}

<Section title={tr("Atajos de teclado", "Keyboard shortcuts")} description={tr("Funcionan en todo Windows. Se avisa si chocan con atajos del sistema o de aplicaciones conocidas.", "They work across Windows. You're warned if they clash with system or well-known app shortcuts.")}>
  {#snippet actions()}
    <Button size="sm" variant="ghost" icon="refresh" loading={restarting} onclick={restartHotkeys} title={tr("Reinstala el gancho de teclado si algún programa lo ha desactivado.", "Reinstalls the keyboard hook if another program disabled it.")}>{tr("Reiniciar atajos", "Restart shortcuts")}</Button>
  {/snippet}
  {#each shortcuts as a (a.key)}
    <Row title={a.title} description={a.description}>
      <HotkeyField bind:value={app.settings.dictation.hotkeys[a.key]} action={a.key} fallback={fallback[a.key]} clearable={a.clearable} />
    </Row>
  {/each}
  <Row title={tr("Pausar todos los atajos", "Pause all shortcuts")} description={tr("Útil al jugar o en presentaciones. También desde el icono de la bandeja.", "Handy for gaming or presentations. Also from the tray icon.")}>
    <Toggle bind:checked={app.settings.dictation.hotkeys_paused} />
  </Row>
</Section>

<Section title={tr("Grabación", "Recording")}>
  <Row title={tr("Forma de activar", "Activation")} description={tr("Híbrido combina «mantener para hablar» y «manos libres» en el mismo atajo.", "Hybrid combines push-to-talk and hands-free on the same shortcut.")}>
    <Segmented bind:value={app.settings.dictation.activation} options={[
      { value: "hybrid", label: tr("Híbrido", "Hybrid") },
      { value: "hold", label: tr("Mantener", "Hold") },
      { value: "toggle", label: tr("Pulsar", "Toggle") },
    ]} />
  </Row>
  {#if d.activation === "hybrid"}
    <Row title={tr("Pulsación breve", "Short press")} description={tr("Si sueltas antes de este tiempo, el dictado sigue en manos libres.", "Releasing before this time keeps dictating hands-free.")}>
      <Select bind:value={app.settings.dictation.hold_threshold_ms} width="150px" options={[200, 250, 300, 350, 450, 600].map((v) => ({ value: v, label: `${v} ms` }))} />
    </Row>
  {/if}
  <Row title={tr("Ignorar pulsaciones muy cortas", "Ignore very short presses")} description={tr("Evita grabaciones accidentales al rozar el atajo.", "Avoids accidental recordings when brushing the shortcut.")}>
    <Select bind:value={app.settings.dictation.min_duration_ms} width="150px" options={[
      { value: 200, label: "0,2 s" },
      { value: 300, label: "0,3 s" },
      { value: 500, label: "0,5 s" },
      { value: 800, label: "0,8 s" },
    ]} />
  </Row>
</Section>

<Section title={tr("Inserción del texto", "Text insertion")}>
  <Row title={tr("Insertar automáticamente", "Insert automatically")} description={tr("Escribe el resultado en la aplicación activa. Si no hay un campo de texto, se copia al portapapeles y se avisa.", "Types the result into the active app. If there's no text field, it's copied to the clipboard and you're notified.")}>
    <Toggle bind:checked={app.settings.dictation.auto_insert} />
  </Row>
  <Row title={tr("Método", "Method")} disabled={!d.auto_insert} description={d.insert_method === "type"
    ? tr("Escribe carácter a carácter: más lento, pero funciona donde no se puede pegar.", "Types character by character: slower, but works where pasting is blocked.")
    : d.insert_method === "paste_shift_insert"
      ? tr("Para terminales y aplicaciones antiguas que no aceptan Ctrl+V.", "For terminals and legacy apps that don't accept Ctrl+V.")
      : d.insert_method === "clipboard_only"
        ? tr("Solo copia el texto; tú decides dónde pegarlo.", "Only copies the text; you decide where to paste it.")
        : tr("El más rápido y fiable. Los perfiles de aplicación pueden usar otro método.", "Fastest and most reliable. App profiles can use another method.")}>
    <Select bind:value={app.settings.dictation.insert_method} width="230px" options={[
      { value: "paste", label: tr("Pegar (Ctrl+V)", "Paste (Ctrl+V)") },
      { value: "paste_shift_insert", label: tr("Pegar (Mayús+Insert)", "Paste (Shift+Insert)") },
      { value: "type", label: tr("Escribir carácter a carácter", "Type character by character") },
      { value: "clipboard_only", label: tr("Solo copiar", "Copy only") },
    ]} />
  </Row>
  <Row title={tr("Restaurar el portapapeles", "Restore the clipboard")} disabled={!d.auto_insert || d.insert_method === "type" || d.insert_method === "clipboard_only"} description={tr("Tras pegar, recupera lo que tenías copiado. El dictado no queda en el historial del portapapeles de Windows.", "After pasting, restores what you had copied. Dictations don't end up in Windows clipboard history.")}>
    <Toggle bind:checked={app.settings.dictation.restore_clipboard} />
  </Row>
  <Row title={tr("Espaciado inteligente", "Smart spacing")} description={tr("Añade un espacio al encadenar dictados en el mismo campo.", "Adds a space when chaining dictations in the same field.")}>
    <Toggle bind:checked={app.settings.dictation.smart_spacing} />
  </Row>
  <Row title={tr("Copiar también al portapapeles", "Also copy to clipboard")} description={tr("Deja el texto disponible para pegarlo en otro sitio.", "Leaves the text available to paste elsewhere.")}>
    <Toggle bind:checked={app.settings.dictation.also_copy} />
  </Row>
</Section>

<Section title={tr("Limpieza del texto", "Text cleanup")} description={tr("Reglas locales e instantáneas. Nunca eliminan contenido con significado.", "Local, instant rules. They never remove meaningful content.")}>
  <Row title={tr("Quitar muletillas", "Remove fillers")} description={tr("Vocalizaciones como «eh», «ehm» o «mmm». Las palabras con significado («bueno», «o sea») solo se quitan en los modos que lo indican.", "Sounds like “uh”, “um” or “hmm”. Meaningful words are only removed in modes that say so.")}>
    <Toggle bind:checked={app.settings.writing.cleanup.remove_fillers} />
  </Row>
  {#if c.remove_fillers}
    <div class="chips">
      {#each c.fillers as f (f)}
        <span class="chip">{f}<button aria-label={tr("Quitar", "Remove")} onclick={() => (app.settings.writing.cleanup.fillers = c.fillers.filter((x) => x !== f))}><Icon name="x" size={11} /></button></span>
      {/each}
      <span class="add">
        <Input bind:value={fillerDraft} placeholder={tr("Añadir…", "Add…")} width="120px" onkeydown={(e) => e.key === "Enter" && fillerDraft.trim() && addFiller()} />
      </span>
    </div>
  {/if}
  <Row title={tr("Quitar repeticiones", "Remove repetitions")} description={tr("«que que» → «que». Respeta repeticiones intencionadas como «muy muy».", "“the the” → “the”. Keeps intentional ones like “very very”.")}>
    <Toggle bind:checked={app.settings.writing.cleanup.collapse_repetitions} />
  </Row>
  <Row title={tr("Corregir arranques en falso", "Fix false starts")} description={tr("«Quería dec- decirte» → «Quería decirte»; «Quiero… quiero que vengas» → «Quiero que vengas». Las rectificaciones («el lunes, mejor el martes») se conservan.", "“I wan- wanted to” → “I wanted to”; “I want… I want you to come” → “I want you to come”. Rephrasings (“Monday, better Tuesday”) are kept.")}>
    <Toggle bind:checked={app.settings.writing.cleanup.remove_false_starts} />
  </Row>
  <Row title={tr("Puntuación dictada", "Spoken punctuation")} description={tr("«punto y aparte», «nueva línea», «punto y coma», «abre paréntesis»… La puntuación normal ya la pone el motor de voz. Lista completa en Comandos de voz.", "“new paragraph”, “new line”, “open parenthesis”… Regular punctuation is added by the speech engine. Full list in Voice commands.")}>
    <Toggle bind:checked={app.settings.writing.cleanup.spoken_punctuation} />
  </Row>
  <Row title={tr("Mayúsculas automáticas", "Auto capitalization")} description={tr("Al inicio de cada frase.", "At the start of each sentence.")}>
    <Toggle bind:checked={app.settings.writing.cleanup.auto_capitalize} />
  </Row>
  <Row title={tr("Corregir espacios", "Fix spacing")} description={tr("Espacios dobles y alrededor de signos de puntuación.", "Double spaces and spacing around punctuation.")}>
    <Toggle bind:checked={app.settings.writing.cleanup.fix_spacing} />
  </Row>
  <Row title={tr("Símbolos en cantidades", "Symbols in amounts")} description={tr("«20 por ciento» → «20 %», «15 euros» → «15 €». Las cifras ya las escribe el motor de voz.", "“20 percent” → “20%”, “15 euros” → “€15”. The speech engine already writes digits.")}>
    <Toggle bind:checked={app.settings.writing.cleanup.format_numbers} />
  </Row>
  <Row title={tr("Filtrar frases inventadas", "Filter invented phrases")} description={tr("Descarta textos típicos que algunos modelos generan con ruido o silencio («Subtítulos realizados por…», «Gracias por ver»).", "Discards typical phrases some models produce from noise or silence (“Thanks for watching”).")}>
    <Toggle bind:checked={app.settings.writing.cleanup.hallucination_filter} />
  </Row>
</Section>

<style>
  .h {
    margin-bottom: 20px;
  }
  .banner {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 14px 16px;
    border-radius: var(--radius);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    margin-bottom: 22px;
    font-size: var(--text-sm);
  }
  .banner > :global(svg) {
    margin-top: 2px;
    flex-shrink: 0;
  }
  .banner div {
    flex: 1;
  }
  .banner p {
    margin: 2px 0 0;
    color: var(--text-2);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 2px 18px 14px;
    border-bottom: 1px solid var(--border);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 6px 0 10px;
    border-radius: 13px;
    background: var(--surface-3);
    font-size: var(--text-sm);
  }
  .chip button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border: none;
    border-radius: 9px;
    background: none;
    color: var(--text-3);
    cursor: pointer;
    padding: 0;
  }
  .chip button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .add :global(input) {
    height: 26px;
  }
</style>
