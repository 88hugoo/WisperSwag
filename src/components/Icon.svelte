<script lang="ts" module>
  // Iconos vectoriales minimalistas (trazo 1,6 px, extremos redondeados).
  const ICONS: Record<string, string> = {
    home: '<path d="M4 10.5 12 4l8 6.5V19a1 1 0 0 1-1 1h-4.5v-5.5h-5V20H5a1 1 0 0 1-1-1z"/>',
    history: '<path d="M4 12a8 8 0 1 0 2.4-5.7"/><path d="M4 4.5v3.8h3.8"/><path d="M12 8v4.2l2.8 1.8"/>',
    book: '<path d="M5 4.5h9.5A2.5 2.5 0 0 1 17 7v12.5H7.5A2.5 2.5 0 0 1 5 17z"/><path d="M5 17a2.5 2.5 0 0 1 2.5-2.5H17"/><path d="M9 8.5h4.5"/>',
    snippet: '<path d="M8.5 6 4 12l4.5 6"/><path d="M15.5 6 20 12l-4.5 6"/><path d="M13 5l-2 14"/>',
    modes: '<path d="M4 7h9"/><path d="M17 7h3"/><circle cx="15" cy="7" r="2"/><path d="M4 17h3"/><path d="M11 17h9"/><circle cx="9" cy="17" r="2"/>',
    apps: '<rect x="4" y="4" width="6.5" height="6.5" rx="1.6"/><rect x="13.5" y="4" width="6.5" height="6.5" rx="1.6"/><rect x="4" y="13.5" width="6.5" height="6.5" rx="1.6"/><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1.6"/>',
    command: '<path d="m5 8 4 4-4 4"/><path d="M12 16.5h7"/>',
    settings: '<circle cx="12" cy="12" r="3"/><path d="M12 3.5v2.2M12 18.3v2.2M20.5 12h-2.2M5.7 12H3.5M18 6l-1.6 1.6M7.6 16.4 6 18M18 18l-1.6-1.6M7.6 7.6 6 6"/>',
    mic: '<rect x="9" y="3.5" width="6" height="11" rx="3"/><path d="M5.5 11.5a6.5 6.5 0 0 0 13 0"/><path d="M12 18v2.5"/>',
    stop: '<rect x="7" y="7" width="10" height="10" rx="2.2"/>',
    x: '<path d="m6.5 6.5 11 11M17.5 6.5l-11 11"/>',
    check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
    search: '<circle cx="11" cy="11" r="6.5"/><path d="m16 16 4 4"/>',
    star: '<path d="m12 4 2.4 5 5.4.7-4 3.7 1 5.4L12 16.2 7.2 18.8l1-5.4-4-3.7 5.4-.7z"/>',
    copy: '<rect x="8.5" y="8.5" width="11" height="11" rx="2"/><path d="M15.5 8.5V6a1.5 1.5 0 0 0-1.5-1.5H6A1.5 1.5 0 0 0 4.5 6v8A1.5 1.5 0 0 0 6 15.5h2.5"/>',
    edit: '<path d="M14.5 5.5 18.5 9.5"/><path d="M5 19l1-4.5L15.8 4.7a1.8 1.8 0 0 1 2.5 0l1 1a1.8 1.8 0 0 1 0 2.5L9.5 18z"/>',
    trash: '<path d="M4.5 7h15"/><path d="M9.5 7V5a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1v2"/><path d="M6.5 7l.8 11.2A2 2 0 0 0 9.3 20h5.4a2 2 0 0 0 2-1.8L17.5 7"/>',
    refresh: '<path d="M19.5 12a7.5 7.5 0 0 1-13.4 4.6"/><path d="M4.5 12A7.5 7.5 0 0 1 17.9 7.4"/><path d="M18.5 3.8v3.9h-3.9"/><path d="M5.5 20.2v-3.9h3.9"/>',
    download: '<path d="M12 4v11"/><path d="m7.5 10.5 4.5 4.5 4.5-4.5"/><path d="M5 19.5h14"/>',
    upload: '<path d="M12 15V4"/><path d="m7.5 8.5 4.5-4.5 4.5 4.5"/><path d="M5 19.5h14"/>',
    play: '<path d="M8 5.5v13l10.5-6.5z"/>',
    pause: '<path d="M8.5 5.5v13M15.5 5.5v13"/>',
    "chevron-right": '<path d="m9.5 6 6 6-6 6"/>',
    "chevron-down": '<path d="m6 9.5 6 6 6-6"/>',
    "chevron-left": '<path d="m14.5 6-6 6 6 6"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    minus: '<path d="M5 12h14"/>',
    external: '<path d="M13.5 4.5h6v6"/><path d="m19.5 4.5-8 8"/><path d="M17.5 13.5v4.5a1.5 1.5 0 0 1-1.5 1.5H6A1.5 1.5 0 0 1 4.5 18V8A1.5 1.5 0 0 1 6 6.5h4.5"/>',
    info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5"/><path d="M12 7.8v.2"/>',
    alert: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5v5.5"/><path d="M12 16.2v.2"/>',
    sparkle: '<path d="M12 3.5c.6 4.4 2.6 6.9 8 8.5-5.4 1.6-7.4 4.1-8 8.5-.6-4.4-2.6-6.9-8-8.5 5.4-1.6 7.4-4.1 8-8.5z"/>',
    keyboard: '<rect x="3.5" y="6.5" width="17" height="11" rx="2"/><path d="M7 10h.01M10 10h.01M13 10h.01M16 10h.01M8 14h8"/>',
    globe: '<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17"/><path d="M12 3.5c2.4 2.4 3.5 5.2 3.5 8.5s-1.1 6.1-3.5 8.5c-2.4-2.4-3.5-5.2-3.5-8.5s1.1-6.1 3.5-8.5z"/>',
    lock: '<rect x="5.5" y="10.5" width="13" height="9.5" rx="2"/><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"/>',
    shield: '<path d="M12 3.5 19 6v5.5c0 4.3-2.9 7.6-7 9-4.1-1.4-7-4.7-7-9V6z"/><path d="m9 12 2.2 2.2L15.5 10"/>',
    cpu: '<rect x="6.5" y="6.5" width="11" height="11" rx="2"/><path d="M10 10h4v4h-4z"/><path d="M9.5 3.5v3M14.5 3.5v3M9.5 17.5v3M14.5 17.5v3M3.5 9.5h3M3.5 14.5h3M17.5 9.5h3M17.5 14.5h3"/>',
    zap: '<path d="M13 3.5 5.5 13.5H12l-1 7 7.5-10H12z"/>',
    clock: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>',
    sun: '<circle cx="12" cy="12" r="3.8"/><path d="M12 3v1.8M12 19.2V21M21 12h-1.8M4.8 12H3M18.4 5.6l-1.3 1.3M6.9 17.1l-1.3 1.3M18.4 18.4l-1.3-1.3M6.9 6.9 5.6 5.6"/>',
    moon: '<path d="M19.5 14.5A8 8 0 0 1 9.5 4.5a8 8 0 1 0 10 10z"/>',
    monitor: '<rect x="3.5" y="4.5" width="17" height="11.5" rx="2"/><path d="M9 20h6M12 16v4"/>',
    minimize: '<path d="M6 12h12"/>',
    maximize: '<rect x="6.5" y="6.5" width="11" height="11" rx="1.5"/>',
    close: '<path d="m7 7 10 10M17 7 7 17"/>',
    folder: '<path d="M3.5 7.5A1.5 1.5 0 0 1 5 6h4l2 2h8a1.5 1.5 0 0 1 1.5 1.5v8A1.5 1.5 0 0 1 19 19H5a1.5 1.5 0 0 1-1.5-1.5z"/>',
    file: '<path d="M7 3.5h7l4.5 4.5v11A1.5 1.5 0 0 1 17 20.5H7A1.5 1.5 0 0 1 5.5 19V5A1.5 1.5 0 0 1 7 3.5z"/><path d="M13.5 3.5V8.5h5"/>',
    dots: '<path d="M6 12h.01M12 12h.01M18 12h.01"/>',
    wave: '<path d="M4 10v4M7.2 7v10M10.4 4.5v15M13.6 8v8M16.8 6v12M20 10v4"/>',
    power: '<path d="M12 3.5v8"/><path d="M7 6.5a7.5 7.5 0 1 0 10 0"/>',
    eye: '<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="2.8"/>',
    "eye-off": '<path d="M4 4l16 16"/><path d="M9.6 6c.8-.3 1.6-.5 2.4-.5 6 0 9.5 6.5 9.5 6.5a17 17 0 0 1-2.4 3.2M6.4 7.2C4 9 2.5 12 2.5 12s3.5 6.5 9.5 6.5c1.4 0 2.6-.3 3.7-.8"/>',
    bell: '<path d="M6.5 16.5V11a5.5 5.5 0 0 1 11 0v5.5l1.5 1.5H5z"/><path d="M10 20.5h4"/>',
    volume: '<path d="M4.5 9.5h3.5l4.5-4v13l-4.5-4H4.5z"/><path d="M16 9a4.5 4.5 0 0 1 0 6M18.5 6.5a8 8 0 0 1 0 11"/>',
    sort: '<path d="M8 4.5v15M4.5 16 8 19.5 11.5 16"/><path d="M16 19.5v-15M12.5 8 16 4.5 19.5 8"/>',
    filter: '<path d="M4.5 6h15l-6 7v5.5l-3 1.5V13z"/>',
    language: '<path d="M4 6h9M8.5 4v2M6 6c.8 3.5 3.2 6 6.5 7.5"/><path d="M11 6c-.8 3.5-3.2 6-6.5 7.5"/><path d="m12.5 20 3.5-8 3.5 8M13.6 17.5h4.8"/>',
    target: '<circle cx="12" cy="12" r="8.5"/><circle cx="12" cy="12" r="4.5"/><circle cx="12" cy="12" r=".6"/>',
    arrow: '<path d="M5 12h14"/><path d="m13 6 6 6-6 6"/>',
    "arrow-left": '<path d="M19 12H5"/><path d="m11 6-6 6 6 6"/>',
    grip: '<path d="M9 6h.01M15 6h.01M9 12h.01M15 12h.01M9 18h.01M15 18h.01"/>',
    hash: '<path d="M5 9h14M5 15h14M10 4l-2 16M16 4l-2 16"/>',
    message: '<path d="M4.5 6.5A2 2 0 0 1 6.5 4.5h11a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H10l-4.5 3.5v-3.5h-1z"/>',
    mail: '<rect x="3.5" y="5.5" width="17" height="13" rx="2"/><path d="m4 7 8 6 8-6"/>',
    code: '<path d="m8.5 7-5 5 5 5"/><path d="m15.5 7 5 5-5 5"/>',
    note: '<path d="M6 3.5h12A1.5 1.5 0 0 1 19.5 5v10.5L15 20.5H6A1.5 1.5 0 0 1 4.5 19V5A1.5 1.5 0 0 1 6 3.5z"/><path d="M15 20.5V16a.5.5 0 0 1 .5-.5h4"/><path d="M8.5 8.5h7M8.5 12h4"/>',
    list: '<path d="M9 6.5h11M9 12h11M9 17.5h11"/><path d="M4.5 6.5h.01M4.5 12h.01M4.5 17.5h.01"/>',
  };

  export type IconName = keyof typeof ICONS;
</script>

<script lang="ts">
  let { name, size = 16, stroke = 1.6, fill = false }: { name: string; size?: number; stroke?: number; fill?: boolean } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={fill ? "currentColor" : "none"}
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {@html ICONS[name] ?? ""}
</svg>

<style>
  svg {
    flex: none;
    display: block;
  }
</style>
