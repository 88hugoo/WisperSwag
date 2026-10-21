import { app } from "./store.svelte";

export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 ? 0 : 1).replace(".", app.lang === "es" ? "," : ".")} ${units[i]}`;
}

export function number(n: number): string {
  return new Intl.NumberFormat(app.lang === "en" ? "en-US" : "es-ES").format(Math.round(n));
}

export function relativeDate(iso: string): string {
  const d = new Date(iso);
  const now = new Date();
  const locale = app.lang === "en" ? "en-US" : "es-ES";
  const time = d.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" });
  const sameDay = d.toDateString() === now.toDateString();
  const yesterday = new Date(now.getTime() - 86_400_000).toDateString() === d.toDateString();
  if (sameDay) return (app.lang === "en" ? "Today, " : "Hoy, ") + time;
  if (yesterday) return (app.lang === "en" ? "Yesterday, " : "Ayer, ") + time;
  return d.toLocaleDateString(locale, { day: "numeric", month: "short", year: d.getFullYear() === now.getFullYear() ? undefined : "numeric" }) + ", " + time;
}

export function duration(ms: number): string {
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min ${s % 60 ? (s % 60) + " s" : ""}`.trim();
  return `${Math.floor(m / 60)} h ${m % 60} min`;
}

export function minutes(min: number): string {
  if (min < 60) return `${Math.round(min)} min`;
  const h = Math.floor(min / 60);
  return `${h} h ${Math.round(min % 60)} min`;
}

/** Nombre legible de un ejecutable («outlook.exe» → «Outlook»). */
export function appName(exe: string | null | undefined): string {
  if (!exe) return "";
  const known: Record<string, string> = {
    "code.exe": "VS Code",
    "winword.exe": "Word",
    "outlook.exe": "Outlook",
    "olk.exe": "Outlook",
    "msedge.exe": "Edge",
    "chrome.exe": "Chrome",
    "firefox.exe": "Firefox",
    "whatsapp.exe": "WhatsApp",
    "whatsapp.root.exe": "WhatsApp",
    "ms-teams.exe": "Teams",
    "windowsterminal.exe": "Terminal",
    "devenv.exe": "Visual Studio",
    "notepad.exe": "Bloc de notas",
  };
  if (known[exe]) return known[exe];
  const base = exe.replace(/\.exe$/i, "");
  return base.charAt(0).toUpperCase() + base.slice(1);
}
