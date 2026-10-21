// Backend simulado para previsualizar la interfaz en un navegador (sin Tauri).
// No se usa en la aplicación real.
import defaults from "./defaults.json";
import type { HistoryEntry, Settings } from "./types";

type Handler = (payload: any) => void;
const listeners = new Map<string, Set<Handler>>();

function emit(event: string, payload: unknown) {
  listeners.get(event)?.forEach((h) => h(payload));
}

export function listen(event: string, handler: Handler) {
  if (!listeners.has(event)) listeners.set(event, new Set());
  listeners.get(event)!.add(handler);
  return () => listeners.get(event)?.delete(handler);
}

const params = new URLSearchParams(location.search);
let settings: Settings = structuredClone(defaults) as Settings;
settings.onboarding_done = params.get("firstRun") !== "1";
settings.vocabulary.entries = [
  { id: "1", term: "Kubernetes", variants: ["cubernetes"], list: "Programación", enabled: true, note: "" },
  { id: "2", term: "Ximena Vélez", variants: [], list: "Trabajo", enabled: true, note: "Directora de producto" },
  { id: "3", term: "ChatGPT", variants: ["chat gpt"], list: "Informática", enabled: true, note: "" },
];
settings.vocabulary.replacements = [
  { id: "r1", from: "visual estudio code", to: "Visual Studio Code", whole_word: true, case_sensitive: false, regex: false, list: "Programación", enabled: true },
];
settings.snippets = [
  { id: "s1", trigger: "mi dirección", content: "Calle Mayor 14, 3.º B, 28013 Madrid", enabled: true, whole_utterance_only: false },
  { id: "s2", trigger: "firma del trabajo", content: "Un saludo,\nAna García\nProducto · ACME", enabled: true, whole_utterance_only: true },
];

const now = Date.now();
const texts = [
  ["Hola Marta,\n\nTe confirmo la reunión del jueves a las 10:30. Llevaré el informe trimestral.\n\nUn saludo,\nAna", "email", "outlook.exe", true],
  ["Llego en cinco minutos", "messages", "whatsapp.exe", false],
  ["Despliega el servicio en Kubernetes y revisa los logs de GitHub antes de la demo.", "natural", "slack.exe", false],
  ["- Revisar el presupuesto\n- Llamar a Juan\n- Preparar la demo del viernes", "notes", "obsidian.exe", false],
  ["const totalDeVentas = calcularVentas(pedidos)", "code", "code.exe", false],
  ["Ayer estuvimos revisando el presupuesto del proyecto con todo el equipo. Por otro lado, el cliente ha pedido adelantar la entrega dos semanas.", "professional", "winword.exe", true],
] as const;
let history: HistoryEntry[] = texts.map(([t, mode, app, ai], i) => ({
  id: i + 1,
  created_at: new Date(now - i * 3_600_000 * 5).toISOString(),
  raw_text: t.toLowerCase(),
  final_text: t,
  mode,
  app,
  language: "es",
  engine: "local",
  model: "parakeet-tdt-0.6b-v3",
  duration_ms: 4000 + i * 900,
  processing_ms: 420 + i * 40,
  words: t.split(/\s+/).length,
  favorite: i === 0,
  edited: false,
  ai_used: ai,
  audio_path: null,
}));

let phase: "idle" | "recording" | "processing" = "idle";

function catalog() {
  const installed = params.get("noModel") === "1" ? [] : ["parakeet-tdt-0.6b-v3"];
  const base = [
    ["parakeet-tdt-0.6b-v3", "Parakeet V3 (recomendado)", "parakeet", "Muy preciso y rápido en CPU. 25 idiomas europeos (español incluido) con detección automática, puntuación y mayúsculas. No inventa texto en los silencios.", 465, 900, 5, 5, true],
    ["whisper-turbo", "Whisper Turbo", "whisper", "Whisper large-v3-turbo: 99 idiomas, muy buena precisión. Más pesado y lento en CPU que Parakeet.", 538, 1600, 2, 5, false],
    ["whisper-small", "Whisper Small", "whisper", "Equilibrio entre tamaño y precisión para los 99 idiomas de Whisper.", 610, 700, 3, 3, false],
    ["whisper-base", "Whisper Base", "whisper", "Ligero (160 MB). Precisión limitada; útil en equipos modestos.", 198, 350, 4, 2, false],
  ] as const;
  return base.map(([id, name, family, description, dl, ram, speed, acc, rec]) => ({
    id, name, family, description, archive: "", download_mb: dl, ram_mb: ram, speed, accuracy_es: acc,
    languages: family === "parakeet" ? ["bg", "hr", "cs", "da", "nl", "en", "et", "fi", "fr", "de", "el", "hu", "it", "lv", "lt", "mt", "pl", "pt", "ro", "sk", "sl", "es", "sv", "ru", "uk"] : null,
    auto_language: true, recommended: rec, files: [], installed: installed.includes(id), disk_mb: Math.round(dl * 1.4),
  }));
}

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));

function keyLabel(hotkey: string): string {
  const label = String(hotkey).replaceAll("+", " + ");
  return params.get("lang") === "en" ? label : label.replace("Shift", "Mayús").replace("Space", "Espacio");
}

export async function invoke(cmd: string, args: Record<string, any>): Promise<unknown> {
  await delay(60);
  switch (cmd) {
    case "get_bootstrap":
      return {
        settings: structuredClone(settings),
        version: "0.1.0",
        platform: "windows",
        ui_lang: params.get("lang") === "en" ? "en" : "es",
        first_run: !settings.onboarding_done,
        status: { phase, hands_free: false, command: false, paused: false, queue: 0 },
        hotkeys_available: true,
        hotkey_error: null,
        elevated: false,
        secrets_backend: "Administrador de credenciales de Windows",
        paths: { config: "C:\\Users\\demo\\AppData\\Roaming\\WisprSwag", data: "C:\\Users\\demo\\AppData\\Local\\WisprSwag" },
        languages: [["auto", "Detección automática"], ["es", "Español"], ["en", "Inglés"], ["ca", "Catalán"], ["fr", "Francés"], ["pt", "Portugués"], ["de", "Alemán"], ["it", "Italiano"]],
      };
    case "save_settings":
      settings = structuredClone(args.settings);
      return structuredClone(settings);
    case "complete_onboarding":
      settings.onboarding_done = true;
      return null;
    case "toggle_dictation":
      phase = phase === "recording" ? "processing" : "recording";
      emit("status", { phase, hands_free: true, command: false, paused: false, queue: 0 });
      if (phase === "processing") {
        setTimeout(() => {
          phase = "idle";
          emit("status", { phase, hands_free: false, command: false, paused: false, queue: 0 });
          emit("dictation-result", { text: "Esto es una prueba de dictado con Wispr Swag.", inserted: true });
        }, 900);
      }
      return null;
    case "list_devices":
      return [
        { id: "wasapi:1", name: "Micrófono (Realtek High Definition Audio)", is_default: true },
        { id: "wasapi:2", name: "Auriculares (Jabra Evolve2 65)", is_default: false },
      ];
    case "mic_test_start": {
      let t = 0;
      const id = setInterval(() => {
        t += 1;
        const rms = Math.abs(Math.sin(t / 3)) * 0.08 + 0.004;
        emit("mic-level", { rms, peak: rms * 2.5, db: 20 * Math.log10(rms) });
        if (t > 400) clearInterval(id);
      }, 50);
      (window as any).__micTimer = id;
      return "Micrófono (Realtek High Definition Audio)";
    }
    case "mic_test_stop":
      clearInterval((window as any).__micTimer);
      emit("mic-level", { stopped: true });
      return null;
    case "mic_record_test":
      await delay(1500);
      return { text: "Hola, esto es una prueba del micrófono.", duration_ms: 5000, peak_db: -9.5, noise_db: -58, speech_ms: 2600, engine_ms: 640, device: "Micrófono (Realtek)" };
    case "list_models":
      return { catalog: catalog(), installed: catalog().filter((c) => c.installed).map((c) => ({ id: c.id, name: c.name, family: c.family, dir: "", size_bytes: 670_000_000, custom: false })), loaded: null, downloading: [], recommended_threads: 4 };
    case "download_model": {
      const total = 487_000_000;
      for (let i = 1; i <= 20; i++) {
        await delay(120);
        emit("model-progress", { model_id: args.id, phase: i < 16 ? "downloading" : "extracting", done_bytes: (total * i) / 20, total_bytes: total, message: null });
      }
      emit("model-progress", { model_id: args.id, phase: "done", done_bytes: total, total_bytes: total, message: null });
      return { id: args.id };
    }
    case "secret_status":
      return { "ai:anthropic": "sk-…9f2a" };
    case "ai_usage":
      return { day: "", day_requests: 12, month: "", month_requests: 214, month_input_tokens: 48210, month_output_tokens: 20130, total_requests: 980, total_input_tokens: 210000, total_output_tokens: 90000, month_cost_usd: 0.42, failures: 1 };
    case "ai_test":
      await delay(700);
      return { model: "claude-opus-5-5", elapsed_ms: 812, reply: "correcto" };
    case "ai_list_models":
      return ["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"];
    case "history_query": {
      const q = args.query;
      let list = history.filter((h) => !q.favorites_only || h.favorite);
      if (q.search) list = list.filter((h) => h.final_text.toLowerCase().includes(q.search.toLowerCase()));
      if (q.oldest_first) list = [...list].reverse();
      return { entries: list.slice(q.offset, q.offset + (q.limit || 50)), total: list.length };
    }
    case "history_favorite":
      history = history.map((h) => (h.id === args.id ? { ...h, favorite: args.favorite } : h));
      return null;
    case "history_delete":
      history = history.filter((h) => h.id !== args.id);
      return null;
    case "history_update":
      history = history.map((h) => (h.id === args.id ? { ...h, final_text: args.text, edited: true } : h));
      return null;
    case "history_reprocess": {
      const e = history.find((h) => h.id === args.id)!;
      return { entry: { ...e, final_text: e.final_text + " (reprocesado)" }, ai_used: true };
    }
    case "stats": {
      const days = Array.from({ length: 30 }, (_, i) => ({
        date: new Date(now - (29 - i) * 86_400_000).toISOString().slice(0, 10),
        words: Math.round(Math.max(0, Math.sin(i / 2.3) * 400 + 380 + (i % 4) * 60)),
        dictations: 10 + (i % 7),
        audio_ms: 300_000,
      }));
      return { total_words: 18452, total_dictations: 812, total_audio_ms: 7_400_000, today_words: 642, week_words: 3910, minutes_saved: 338, speaking_wpm: 149, streak_days: 12, last_30_days: days, top_apps: [["slack.exe", 210], ["outlook.exe", 160], ["code.exe", 98]] };
    }
    case "hotkey_check":
      return { valid: true, display: keyLabel(args.hotkey), conflicts: args.hotkey === "Win+L" ? [{ severity: "error", message: "Reservado por Windows (bloquear el equipo)." }] : [] };
    case "hotkey_display_cmd":
      return keyLabel(args.hotkey);
    case "hotkey_capture_start":
      setTimeout(() => emit("hotkey-captured", { hotkey: "Ctrl+Win", display: "Ctrl + Win" }), 1500);
      return null;
    case "process_text":
      return { text: String(args.text).trim().replace(/^./, (c: string) => c.toUpperCase()), command: null, ai_used: !!args.useAi, ai_error: null };
    case "storage_info":
      return { history_entries: history.length, history_bytes: 182_000, audio_bytes: 0, models_bytes: 670_000_000, logs_bytes: 48_000, config_bytes: 22_000 };
    case "diagnostics":
      return { version: "0.1.0", os: "windows x86_64", uptime_secs: 7342, process: { working_set: 24_600_000, private_bytes: 15_100_000, cpu_time_ms: 3100, threads: 21 }, cpu_percent: 0.0, engine: { loaded_model: null, load_ms: null, idle_secs: 812, threads: 4 }, engine_kind: "local", model_installed: true, hotkeys_running: true, hotkey_error: null, elevated: false, devices: 2, network_features: [], hub_open: true, threads_available: 8 };
    case "check_updates":
      return { current: "0.1.0", latest: "v0.1.0", newer: false, url: "https://github.com/88hugoo/WisperSwag/releases" };
    case "foreground_in":
      await delay(args.seconds * 1000);
      return "slack.exe";
    case "pill_preview":
      return { width: 0, height: 0, rgba: "" };
    case "settings_backup_now":
      return "C:\\Users\\demo\\AppData\\Roaming\\WisprSwag\\backups\\settings-20261020.json";
    default:
      return null;
  }
}
