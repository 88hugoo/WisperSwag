// Acceso al backend. Dentro de Tauri usa IPC; en un navegador normal (desarrollo
// del diseño) usa un backend simulado para poder previsualizar la interfaz.
import type {
  AiAction,
  AiUsage,
  Bootstrap,
  Diagnostics,
  HistoryEntry,
  HistoryPage,
  HistoryQuery,
  HotkeyCheck,
  InputDevice,
  InstalledModel,
  ModelsView,
  PillPreview,
  ProcessPreview,
  RecordTest,
  Settings,
  Stats,
  Status,
  StorageInfo,
  UpdateInfo,
} from "./types";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Handler = (payload: any) => void;
let mockModule: typeof import("./mock") | null = null;

async function mock() {
  if (!mockModule) mockModule = await import("./mock");
  return mockModule;
}

export async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return (await mock()).invoke(cmd, args) as Promise<T>;
}

export async function on(event: string, handler: Handler): Promise<() => void> {
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen(event, (e) => handler(e.payload));
  }
  return (await mock()).listen(event, handler);
}

/** Mensaje legible de un error de IPC. */
export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as any).message);
  return String(e);
}

export const api = {
  bootstrap: () => call<Bootstrap>("get_bootstrap"),
  frontendReady: () => call<void>("frontend_ready"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  completeOnboarding: () => call<void>("complete_onboarding"),
  status: () => call<Status>("get_status"),
  toggleDictation: () => call<void>("toggle_dictation"),
  cancelDictation: () => call<void>("cancel_dictation"),
  togglePause: () => call<void>("toggle_pause"),
  processText: (text: string, modeId: string | null, useAi: boolean) =>
    call<ProcessPreview>("process_text", { text, modeId, useAi }),

  listDevices: () => call<InputDevice[]>("list_devices"),
  micTestStart: (device: string | null) => call<string>("mic_test_start", { device }),
  micTestStop: () => call<void>("mic_test_stop"),
  micRecordTest: (seconds: number) => call<RecordTest>("mic_record_test", { seconds }),

  listModels: () => call<ModelsView>("list_models"),
  downloadModel: (id: string) => call<InstalledModel>("download_model", { id }),
  cancelDownload: (id: string) => call<void>("cancel_download", { id }),
  deleteModel: (id: string) => call<void>("delete_model", { id }),
  importModel: (folder: string, name: string) => call<InstalledModel>("import_model", { folder, name }),
  unloadModel: () => call<void>("unload_model"),
  loadModelNow: () => call<number>("load_model_now"),
  testCloudStt: () => call<string>("test_cloud_stt"),

  setSecret: (key: string, value: string) => call<void>("set_secret", { key, value }),
  deleteSecret: (key: string) => call<void>("delete_secret", { key }),
  secretStatus: () => call<Record<string, string>>("secret_status"),

  aiTest: () => call<{ model: string; elapsed_ms: number; reply: string }>("ai_test"),
  aiListModels: () => call<string[]>("ai_list_models"),
  aiUsage: () => call<AiUsage>("ai_usage"),
  aiUsageReset: () => call<void>("ai_usage_reset"),

  historyQuery: (query: HistoryQuery) => call<HistoryPage>("history_query", { query }),
  historyUpdate: (id: number, text: string) => call<void>("history_update", { id, text }),
  historyFavorite: (id: number, favorite: boolean) => call<void>("history_favorite", { id, favorite }),
  historyDelete: (id: number) => call<void>("history_delete", { id }),
  historyClear: (keepFavorites: boolean) => call<void>("history_clear", { keepFavorites }),
  historyExport: (ids: number[], format: string, path: string) => call<string>("history_export", { ids, format, path }),
  historyCopy: (id: number) => call<void>("history_copy", { id }),
  historyReuse: (id: number) => call<void>("history_reuse", { id }),
  historyReprocess: (id: number, action: AiAction) =>
    call<{ entry: HistoryEntry | null; ai_used: boolean }>("history_reprocess", { id, action }),
  stats: () => call<Stats>("stats"),

  vocabImport: (path: string, list: string) => call<{ entries: number; replacements: number }>("vocab_import", { path, list }),
  vocabExport: (path: string, format: string) => call<void>("vocab_export", { path, format }),
  validateRegex: (pattern: string) => call<void>("validate_regex", { pattern }),

  hotkeyCaptureStart: () => call<void>("hotkey_capture_start"),
  hotkeyCaptureStop: () => call<void>("hotkey_capture_stop"),
  hotkeyCheck: (action: string, hotkey: string) => call<HotkeyCheck>("hotkey_check", { action, hotkey }),
  hotkeyDisplay: (hotkey: string) => call<string>("hotkey_display_cmd", { hotkey }),
  hotkeysRestart: () => call<void>("hotkeys_restart"),

  settingsExport: (path: string) => call<void>("settings_export", { path }),
  settingsImport: (path: string) => call<Settings>("settings_import", { path }),
  settingsBackupNow: () => call<string>("settings_backup_now"),
  settingsReset: () => call<Settings>("settings_reset"),
  openFolder: (kind: string) => call<void>("open_folder", { kind }),
  openSystemSettings: (page: string) => call<void>("open_system_settings", { page }),

  storageInfo: () => call<StorageInfo>("storage_info"),
  deleteData: (options: Record<string, boolean>) => call<void>("delete_data", { options }),
  diagnostics: () => call<Diagnostics>("diagnostics"),
  checkUpdates: () => call<UpdateInfo>("check_updates"),
  foregroundIn: (seconds: number) => call<string>("foreground_in", { seconds }),
  pillPreview: (phase: string, theme: string, scale: number) => call<PillPreview>("pill_preview", { phase, theme, scale }),
  quit: () => call<void>("quit_app"),
};

/** Diálogos nativos (guardar/abrir). En el navegador devuelven una ruta simulada. */
export async function saveDialog(defaultPath: string, filters: { name: string; extensions: string[] }[]) {
  if (!inTauri) return `C:\\Users\\demo\\Documents\\${defaultPath}`;
  const { save } = await import("@tauri-apps/plugin-dialog");
  return save({ defaultPath, filters });
}

export async function openDialog(opts: { directory?: boolean; filters?: { name: string; extensions: string[] }[] }) {
  if (!inTauri) return opts.directory ? "C:\\modelos\\mi-modelo" : "C:\\Users\\demo\\Documents\\archivo.json";
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ multiple: false, directory: !!opts.directory, filters: opts.filters });
  return typeof r === "string" ? r : null;
}

export async function openUrl(url: string) {
  if (!inTauri) {
    window.open(url, "_blank");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}

export async function windowAction(action: "minimize" | "maximize" | "close" | "drag") {
  if (!inTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  if (action === "minimize") await w.minimize();
  else if (action === "maximize") await w.toggleMaximize();
  else if (action === "close") await w.close();
  else await w.startDragging();
}
