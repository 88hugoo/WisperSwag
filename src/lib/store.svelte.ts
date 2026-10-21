// Estado global de la interfaz (Svelte 5, runas).
import { api, errorText, on } from "./api";
import type { Bootstrap, Settings, Status } from "./types";

export interface Toast {
  id: number;
  text: string;
  kind: "info" | "success" | "error";
}

class AppStore {
  boot = $state<Bootstrap | null>(null);
  settings = $state<Settings>(null as unknown as Settings);
  status = $state<Status>({ phase: "idle", hands_free: false, command: false, paused: false, queue: 0 });
  lang = $state<"es" | "en">("es");
  ready = $state(false);
  toasts = $state<Toast[]>([]);
  saveError = $state<string | null>(null);
  #lastSaved = "";
  #timer: ReturnType<typeof setTimeout> | null = null;
  #toastId = 0;

  async init() {
    const boot = await api.bootstrap();
    this.boot = boot;
    this.settings = boot.settings;
    this.#lastSaved = JSON.stringify(boot.settings);
    this.status = boot.status;
    this.lang = boot.ui_lang;
    this.applyTheme();
    this.ready = true;
    on("status", (s: Status) => (this.status = s));
    on("settings-changed", () => this.reload());
    // Guardado automático: cualquier cambio en los ajustes se persiste.
    $effect.root(() => {
      $effect(() => {
        const json = JSON.stringify(this.settings);
        if (json !== this.#lastSaved) this.#schedule();
        this.applyTheme();
      });
    });
    matchMedia("(prefers-color-scheme: light)").addEventListener("change", () => this.applyTheme());
  }

  async reload() {
    const boot = await api.bootstrap();
    this.boot = boot;
    this.#lastSaved = JSON.stringify(boot.settings);
    this.settings = boot.settings;
    this.lang = boot.ui_lang;
  }

  #schedule() {
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = setTimeout(() => this.saveNow(), 280);
  }

  async saveNow() {
    const json = JSON.stringify(this.settings);
    if (json === this.#lastSaved) return;
    try {
      const saved = await api.saveSettings(JSON.parse(json));
      this.#lastSaved = JSON.stringify(saved);
      this.saveError = null;
      // Si el backend ha corregido algún valor (rangos), se refleja.
      if (this.#lastSaved !== json) this.settings = saved;
      const ui = saved.general.ui_language;
      this.lang = ui === "auto" ? (navigator.language.toLowerCase().startsWith("en") ? "en" : "es") : ui;
    } catch (e) {
      this.saveError = errorText(e);
      this.toast(errorText(e), "error");
    }
  }

  applyTheme() {
    if (!this.settings) return;
    const t = this.settings.appearance.theme;
    const light = t === "light" || (t === "system" && matchMedia("(prefers-color-scheme: light)").matches);
    document.documentElement.dataset.theme = light ? "light" : "dark";
    document.documentElement.dataset.motion = this.settings.appearance.animations ? "on" : "off";
    document.documentElement.lang = this.lang;
  }

  toast(text: string, kind: Toast["kind"] = "info", ms = 3200) {
    const id = ++this.#toastId;
    this.toasts = [...this.toasts, { id, text, kind }];
    setTimeout(() => (this.toasts = this.toasts.filter((t) => t.id !== id)), ms);
  }
}

export const app = new AppStore();

/** Traducción en línea: texto en español e inglés. */
export function tr(es: string, en: string): string {
  return app.lang === "en" ? en : es;
}
