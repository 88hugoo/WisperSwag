<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { number } from "../../lib/format";
  import { app, tr } from "../../lib/store.svelte";
  import type { AiProviderKind, AiUsage } from "../../lib/types";
  import Button from "../../components/Button.svelte";
  import Dialog from "../../components/Dialog.svelte";
  import Icon from "../../components/Icon.svelte";
  import Input from "../../components/Input.svelte";
  import Row from "../../components/Row.svelte";
  import Section from "../../components/Section.svelte";
  import Select from "../../components/Select.svelte";
  import Toggle from "../../components/Toggle.svelte";

  interface ProviderInfo {
    value: AiProviderKind;
    label: string;
    url: string;
    model: string;
    local: boolean;
    key: "required" | "optional" | "none";
  }

  const providers: ProviderInfo[] = [
    { value: "open_ai", label: "OpenAI", url: "https://api.openai.com/v1", model: "gpt-5-mini", local: false, key: "required" },
    { value: "anthropic", label: "Anthropic (Claude)", url: "https://api.anthropic.com/v1", model: "claude-opus-5-5", local: false, key: "required" },
    { value: "gemini", label: "Google Gemini", url: "https://generativelanguage.googleapis.com/v1beta/openai", model: "gemini-2.5-flash", local: false, key: "required" },
    { value: "groq", label: "Groq", url: "https://api.groq.com/openai/v1", model: "llama-3.3-70b-versatile", local: false, key: "required" },
    { value: "mistral", label: "Mistral", url: "https://api.mistral.ai/v1", model: "mistral-small-latest", local: false, key: "required" },
    { value: "open_router", label: "OpenRouter", url: "https://openrouter.ai/api/v1", model: "openai/gpt-5-mini", local: false, key: "required" },
    { value: "ollama", label: tr("Ollama (en tu equipo)", "Ollama (on your computer)"), url: "http://localhost:11434/v1", model: "llama3.2", local: true, key: "none" },
    { value: "lm_studio", label: tr("LM Studio (en tu equipo)", "LM Studio (on your computer)"), url: "http://localhost:1234/v1", model: "local-model", local: true, key: "none" },
    { value: "custom", label: tr("Compatible con OpenAI (otro)", "OpenAI-compatible (other)"), url: "http://localhost:8080/v1", model: "", local: false, key: "optional" },
  ];

  const ai = $derived(app.settings.ai);
  const p = $derived(providers.find((x) => x.value === ai.provider) ?? providers[0]);
  const secretKey = $derived(`ai:${ai.provider}`);
  let secrets = $state<Record<string, string>>({});
  let keyDraft = $state("");
  let usage = $state<AiUsage | null>(null);
  let testing = $state(false);
  let test = $state<{ ok: boolean; text: string } | null>(null);
  let models = $state<string[]>([]);
  let listing = $state(false);
  let consentOpen = $state(false);
  let resetOpen = $state(false);

  const savedKey = $derived(secrets[secretKey]);
  const ready = $derived(ai.enabled && ai.consent_given && (p.key !== "required" || !!savedKey));
  const isLocalUrl = $derived(/^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(:|\/|$)/i.test(ai.base_url.trim() || p.url));

  onMount(() => {
    loadSecrets();
    loadUsage();
  });

  async function loadSecrets() {
    secrets = await api.secretStatus();
  }

  async function loadUsage() {
    usage = await api.aiUsage();
  }

  function setEnabled(v: boolean) {
    if (v && !ai.consent_given) {
      app.settings.ai.enabled = false;
      consentOpen = true;
      return;
    }
    app.settings.ai.enabled = v;
  }

  function accept() {
    app.settings.ai.consent_given = true;
    app.settings.ai.enabled = true;
    consentOpen = false;
  }

  function revoke() {
    app.settings.ai.consent_given = false;
    app.settings.ai.enabled = false;
  }

  function changeProvider(v: AiProviderKind) {
    app.settings.ai.provider = v;
    app.settings.ai.base_url = "";
    app.settings.ai.model = "";
    models = [];
    test = null;
    keyDraft = "";
  }

  async function saveKey() {
    try {
      await api.setSecret(secretKey, keyDraft);
      keyDraft = "";
      await loadSecrets();
      app.toast(tr("Clave guardada de forma segura", "Key stored securely"), "success");
    } catch (e) {
      app.toast(errorText(e), "error");
    }
  }

  async function deleteKey() {
    await api.deleteSecret(secretKey);
    await loadSecrets();
  }

  async function runTest() {
    testing = true;
    test = null;
    try {
      await app.saveNow();
      const r = await api.aiTest();
      test = { ok: true, text: tr(`Responde ${r.model} en ${(r.elapsed_ms / 1000).toFixed(1)} s.`, `${r.model} answered in ${(r.elapsed_ms / 1000).toFixed(1)} s.`) };
      loadUsage();
    } catch (e) {
      test = { ok: false, text: errorText(e) };
    } finally {
      testing = false;
    }
  }

  async function listModels() {
    listing = true;
    try {
      await app.saveNow();
      models = await api.aiListModels();
      if (models.length === 0) app.toast(tr("El proveedor no devolvió modelos", "The provider returned no models"), "info");
    } catch (e) {
      app.toast(errorText(e), "error", 6000);
    } finally {
      listing = false;
    }
  }

  async function resetUsage() {
    await api.aiUsageReset();
    resetOpen = false;
    loadUsage();
  }

  const money = (v: number) => new Intl.NumberFormat(app.lang === "en" ? "en-US" : "es-ES", { style: "currency", currency: "USD", maximumFractionDigits: v < 1 ? 3 : 2 }).format(v);
  const priceText = (v: number) => (Number.isFinite(v) ? String(v) : "0");
  function setPrice(which: "price_input_per_mtok" | "price_output_per_mtok", raw: string) {
    const v = Number(raw.replace(",", "."));
    app.settings.ai[which] = Number.isFinite(v) && v >= 0 ? v : 0;
  }
</script>

<h2 class="h">{tr("IA e integraciones", "AI & integrations")}</h2>

<div class="intro">
  <Icon name="sparkle" size={18} />
  <p>
    {tr(
      "Opcional. Sin IA, Wispr Swag limpia y da formato al texto con reglas locales (instantáneas y privadas). Con IA, cada modo de escritura puede reescribir el dictado con más criterio y se activan el modo comando y «Reescribir la selección».",
      "Optional. Without AI, Wispr Swag cleans and formats text with local rules (instant and private). With AI, each writing mode can rewrite the dictation more smartly, and command mode and “Rewrite selection” become available.",
    )}
  </p>
</div>

<Section>
  <Row title={tr("Procesar con IA", "Process with AI")} description={!ai.enabled
    ? tr("Desactivada: nada sale de tu equipo salvo que uses la transcripción en la nube.", "Off: nothing leaves your computer unless you use cloud transcription.")
    : ready
      ? tr(`Activa con ${p.label}. Si falla o tarda más de ${ai.timeout_secs} s, se usa el texto local.`, `On with ${p.label}. If it fails or takes over ${ai.timeout_secs} s, the local text is used.`)
      : tr("Activa, pero falta la clave de API del proveedor.", "On, but the provider's API key is missing.")}>
    {#if ai.enabled && !ready}<span class="warn"><Icon name="alert" size={15} /></span>{/if}
    <Toggle checked={ai.enabled} controlled onchange={setEnabled} />
  </Row>
  {#if ai.consent_given}
    <Row title={tr("Consentimiento", "Consent")} description={tr("Aceptaste el envío de texto descrito al activar la IA.", "You accepted the text sharing described when enabling AI.")}>
      <Button size="sm" variant="ghost" onclick={() => (consentOpen = true)}>{tr("Ver detalles", "View details")}</Button>
      <Button size="sm" variant="ghost" onclick={revoke}>{tr("Retirar", "Revoke")}</Button>
    </Row>
  {/if}
</Section>

<Section title={tr("Proveedor", "Provider")} description={p.local || isLocalUrl
  ? tr("Se ejecuta en tu equipo: el texto no sale del ordenador.", "Runs on your computer: text doesn't leave it.")
  : tr("El texto se envía directamente al proveedor con tu clave. Revisa su política de uso de datos.", "Text is sent straight to the provider with your key. Review its data usage policy.")}>
  <Row title={tr("Servicio", "Service")}>
    <Select value={ai.provider} options={providers.map((x) => ({ value: x.value, label: x.label }))} width="260px" onchange={changeProvider} />
  </Row>
  {#if p.key !== "none"}
    <Row title={tr("Clave de API", "API key")} description={savedKey
      ? `${tr("Guardada en", "Stored in")} ${app.boot?.secrets_backend ?? ""}: ${savedKey}`
      : p.key === "optional" ? tr("Opcional, según el servidor.", "Optional, depending on the server.") : tr("Se guarda cifrada en el Administrador de credenciales de Windows, nunca en los ajustes ni en los registros.", "Stored encrypted in Windows Credential Manager, never in settings or logs.")}>
      <Input bind:value={keyDraft} type="password" mono width="220px" placeholder={savedKey ? tr("Reemplazar clave", "Replace key") : tr("Pega tu clave", "Paste your key")} onkeydown={(e) => e.key === "Enter" && keyDraft.trim() && saveKey()} />
      <Button disabled={!keyDraft.trim()} onclick={saveKey}>{tr("Guardar", "Save")}</Button>
      {#if savedKey}<Button variant="ghost" icon="trash" title={tr("Borrar clave", "Delete key")} onclick={deleteKey} />{/if}
    </Row>
  {/if}
  <Row title={tr("Modelo", "Model")} description={tr("Vacío = recomendado para este proveedor.", "Empty = recommended for this provider.")}>
    {#if models.length}
      <Select value={ai.model || p.model} options={[...new Set([...(ai.model ? [ai.model] : []), ...models])].map((m) => ({ value: m, label: m }))} width="260px" onchange={(v) => (app.settings.ai.model = v)} />
      <Button variant="ghost" icon="edit" title={tr("Escribir a mano", "Type manually")} onclick={() => (models = [])} />
    {:else}
      <Input bind:value={app.settings.ai.model} placeholder={p.model || tr("nombre-del-modelo", "model-name")} mono width="260px" />
      <Button variant="ghost" icon="list" loading={listing} title={tr("Ver modelos disponibles", "List available models")} onclick={listModels} />
    {/if}
  </Row>
  <Row title={tr("Dirección del servicio", "Service URL")} description={tr("Vacío = dirección oficial. Útil para proxies corporativos o servidores propios.", "Empty = official URL. Useful for corporate proxies or own servers.")}>
    <Input bind:value={app.settings.ai.base_url} placeholder={p.url} mono width="320px" />
  </Row>
  <Row title={tr("Probar conexión", "Test connection")} description={test?.text ?? tr("Envía una frase de prueba y mide el tiempo de respuesta.", "Sends a test sentence and measures response time.")}>
    {#if test}<span class="res" class:bad={!test.ok}><Icon name={test.ok ? "check" : "alert"} size={15} /></span>{/if}
    <Button icon="refresh" loading={testing} disabled={!ai.consent_given} title={!ai.consent_given ? tr("Activa la IA primero", "Enable AI first") : undefined} onclick={runTest}>{tr("Probar", "Test")}</Button>
  </Row>
</Section>

<Section title={tr("Qué se envía", "What is sent")}>
  <Row title={tr("Tipo de aplicación", "App type")} description={tr("Una descripción genérica («un cliente de correo»), nunca el título de la ventana ni el nombre del documento.", "A generic description (“an email client”), never the window title or document name.")}>
    <Toggle bind:checked={app.settings.ai.send_app_context} />
  </Row>
  <Row title={tr("Términos del vocabulario", "Vocabulary terms")} description={tr("Para que respete la ortografía de nombres y tecnicismos.", "So it keeps the spelling of names and jargon.")}>
    <Toggle bind:checked={app.settings.ai.send_vocabulary} />
  </Row>
  <Row title={tr("Longitud máxima", "Maximum length")} description={tr("Los dictados más largos se procesan solo en local.", "Longer dictations are processed locally only.")}>
    <Select bind:value={app.settings.ai.max_input_chars} width="170px" options={[2000, 4000, 8000, 16000, 32000].map((v) => ({ value: v, label: `${number(v)} ${tr("caracteres", "chars")}` }))} />
  </Row>
  <Row title={tr("Tiempo máximo de espera", "Maximum wait")} description={tr("Pasado este tiempo se inserta el texto local para no bloquearte.", "After this, the local text is inserted so you're never blocked.")}>
    <Select bind:value={app.settings.ai.timeout_secs} width="120px" options={[5, 8, 12, 20, 30, 60].map((v) => ({ value: v, label: `${v} s` }))} />
  </Row>
</Section>

<Section title={tr("Límites y coste", "Limits & cost")} description={tr("Los límites se comprueban antes de cada solicitud. El coste es una estimación con los precios que indiques.", "Limits are checked before each request. Cost is an estimate using the prices you enter.")}>
  {#snippet actions()}
    <Button size="sm" variant="ghost" icon="refresh" onclick={loadUsage}>{tr("Actualizar", "Refresh")}</Button>
  {/snippet}
  {#if usage}
    <div class="usage">
      <div><span class="num">{number(usage.day_requests)}</span><em>{tr("solicitudes hoy", "requests today")}</em></div>
      <div><span class="num">{number(usage.month_requests)}</span><em>{tr("este mes", "this month")}</em></div>
      <div><span class="num">{number(usage.month_input_tokens + usage.month_output_tokens)}</span><em>{tr("tokens este mes", "tokens this month")}</em></div>
      <div><span class="num">{ai.price_input_per_mtok || ai.price_output_per_mtok ? money(usage.month_cost_usd) : "—"}</span><em>{tr("coste estimado del mes", "estimated monthly cost")}</em></div>
    </div>
    {#if usage.failures > 0}<p class="fails faint">{tr(`${usage.failures} solicitudes fallidas (se usó el texto local).`, `${usage.failures} failed requests (the local text was used).`)}</p>{/if}
  {/if}
  <Row title={tr("Máximo de solicitudes al día", "Max requests per day")}>
    <Select bind:value={app.settings.ai.daily_request_limit} width="150px" options={[
      { value: 50, label: "50" },
      { value: 100, label: "100" },
      { value: 250, label: "250" },
      { value: 500, label: "500" },
      { value: 1000, label: "1.000" },
      { value: 0, label: tr("Sin límite", "No limit") },
    ]} />
  </Row>
  <Row title={tr("Máximo de tokens al mes", "Max tokens per month")}>
    <Select bind:value={app.settings.ai.monthly_token_limit} width="150px" options={[
      { value: 100000, label: "100.000" },
      { value: 500000, label: "500.000" },
      { value: 1000000, label: "1 M" },
      { value: 5000000, label: "5 M" },
      { value: 0, label: tr("Sin límite", "No limit") },
    ]} />
  </Row>
  <Row title={tr("Precio por millón de tokens (USD)", "Price per million tokens (USD)")} description={tr("Entrada / salida, según la tarifa de tu proveedor y modelo.", "Input / output, from your provider's and model's pricing.")}>
    <Input value={priceText(ai.price_input_per_mtok)} width="80px" mono onblur={(e) => setPrice("price_input_per_mtok", (e.target as HTMLInputElement).value)} />
    <span class="faint">/</span>
    <Input value={priceText(ai.price_output_per_mtok)} width="80px" mono onblur={(e) => setPrice("price_output_per_mtok", (e.target as HTMLInputElement).value)} />
  </Row>
  <Row title={tr("Reiniciar contadores", "Reset counters")}>
    <Button variant="ghost" onclick={() => (resetOpen = true)}>{tr("Reiniciar", "Reset")}</Button>
  </Row>
</Section>

<Dialog bind:open={consentOpen} title={tr("Antes de activar la IA", "Before enabling AI")} width="560px">
  <div class="consent">
    <p>{tr("Cuando un dictado use la IA, Wispr Swag enviará al proveedor que elijas:", "When a dictation uses AI, Wispr Swag will send the provider you choose:")}</p>
    <ul>
      <li><Icon name="check" size={13} />{tr("El texto transcrito (ya limpiado en local) y las instrucciones del modo de escritura.", "The transcribed text (already cleaned locally) and the writing mode instructions.")}</li>
      <li><Icon name="check" size={13} />{tr("Si lo permites: el tipo de aplicación («correo», «chat») y los términos de tu vocabulario.", "If allowed: the app type (“email”, “chat”) and your vocabulary terms.")}</li>
      <li><Icon name="check" size={13} />{tr("En el modo comando y al reescribir, también el texto que tengas seleccionado.", "In command mode and when rewriting, also the text you have selected.")}</li>
    </ul>
    <p>{tr("Nunca se envía:", "Never sent:")}</p>
    <ul>
      <li><Icon name="x" size={13} />{tr("El audio (salvo que actives aparte la transcripción en la nube).", "Audio (unless you separately enable cloud transcription).")}</li>
      <li><Icon name="x" size={13} />{tr("Tu historial, títulos de ventanas, nombres de archivos ni otros datos del equipo.", "Your history, window titles, file names or other computer data.")}</li>
    </ul>
    <p class="faint">{tr(
      "La conexión va directa de tu equipo al proveedor (Wispr Swag no tiene servidores). El proveedor trata los datos según sus condiciones; revisa si los conserva o los usa para entrenar. Puedes retirar este consentimiento cuando quieras.",
      "The connection goes straight from your computer to the provider (Wispr Swag has no servers). The provider handles data under its own terms; check whether it retains it or trains on it. You can revoke this consent at any time.",
    )}</p>
  </div>
  {#snippet actions()}
    {#if ai.consent_given}
      <Button variant="primary" onclick={() => (consentOpen = false)}>{tr("Entendido", "Got it")}</Button>
    {:else}
      <Button variant="ghost" onclick={() => (consentOpen = false)}>{tr("Ahora no", "Not now")}</Button>
      <Button variant="primary" onclick={accept}>{tr("Acepto y activar", "Accept and enable")}</Button>
    {/if}
  {/snippet}
</Dialog>

<Dialog bind:open={resetOpen} title={tr("Reiniciar contadores", "Reset counters")}>
  <p class="dlg-text">{tr("Se pondrán a cero las solicitudes, los tokens y el coste estimado. Los límites vuelven a contar desde ahora.", "Requests, tokens and estimated cost go back to zero. Limits count again from now.")}</p>
  {#snippet actions()}
    <Button variant="ghost" onclick={() => (resetOpen = false)}>{tr("Cancelar", "Cancel")}</Button>
    <Button variant="danger" onclick={resetUsage}>{tr("Reiniciar", "Reset")}</Button>
  {/snippet}
</Dialog>

<style>
  .h {
    margin-bottom: 14px;
  }
  .intro {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    margin-bottom: 22px;
    color: var(--text-2);
    font-size: var(--text-sm);
    line-height: 1.55;
    max-width: 720px;
  }
  .intro p {
    margin: 0;
  }
  .intro :global(svg) {
    flex-shrink: 0;
    margin-top: 1px;
    color: var(--text);
  }
  .warn {
    display: inline-flex;
  }
  .res {
    display: inline-flex;
  }
  .res.bad {
    color: var(--text-2);
  }
  .usage {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    border-bottom: 1px solid var(--border);
  }
  .usage div {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px 18px;
  }
  .usage div + div {
    border-left: 1px solid var(--border);
  }
  .usage .num {
    font-size: var(--text-xl);
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .usage em {
    font-style: normal;
    font-size: var(--text-xs);
    color: var(--text-3);
  }
  .fails {
    margin: 0;
    padding: 8px 18px;
    font-size: var(--text-xs);
    border-bottom: 1px solid var(--border);
  }
  .consent {
    font-size: var(--text-sm);
    line-height: 1.55;
  }
  .consent p {
    margin: 0 0 8px;
  }
  .consent ul {
    list-style: none;
    margin: 0 0 14px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: var(--text-2);
  }
  .consent li {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .consent li :global(svg) {
    flex-shrink: 0;
    margin-top: 3px;
    color: var(--text);
  }
  .dlg-text {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-2);
    line-height: 1.55;
  }
</style>
