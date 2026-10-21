// Tipos que reflejan las estructuras de Rust (serde). Mantener sincronizados con
// crates/wispr-core/src/settings.rs y src-tauri/src/commands.rs.

export type UiLanguage = "auto" | "es" | "en";
export type CloseBehavior = "release_memory" | "hide_to_tray" | "quit";
export type Theme = "system" | "dark" | "light";
export type PillSize = "small" | "medium" | "large";
export type PillAnchor = "bottom_center" | "bottom_left" | "bottom_right" | "top_center" | "custom";
export type MonitorChoice = "active" | "cursor" | "primary";
export type InfoLevel = "minimal" | "standard" | "detailed";
export type ActivationMode = "hybrid" | "toggle" | "hold";
export type InsertMethod = "paste" | "paste_shift_insert" | "type" | "clipboard_only";
export type EngineKind = "local" | "cloud";
export type CloudSttProvider = "open_ai" | "groq" | "custom";
export type AiProviderKind =
  | "open_ai"
  | "anthropic"
  | "groq"
  | "open_router"
  | "mistral"
  | "gemini"
  | "ollama"
  | "lm_studio"
  | "custom";
export type FinalPeriod = "keep" | "remove_if_single_sentence" | "remove";
export type Structure = "plain" | "paragraphs" | "bullets" | "checklist" | "email" | "code";
export type AppCategory = "code" | "terminal" | "email" | "chat" | "browser" | "documents" | "notes" | "other";

export interface GeneralSettings {
  ui_language: UiLanguage;
  launch_at_startup: boolean;
  start_minimized: boolean;
  close_behavior: CloseBehavior;
  system_notifications: boolean;
  sounds: boolean;
  sound_volume: number;
  check_updates: boolean;
  auto_backup: boolean;
}

export interface AppearanceSettings {
  theme: Theme;
  pill_size: PillSize;
  pill_opacity: number;
  pill_anchor: PillAnchor;
  pill_custom_x: number;
  pill_custom_y: number;
  pill_monitor: MonitorChoice;
  show_idle_pill: boolean;
  animations: boolean;
  info_level: InfoLevel;
  hide_in_fullscreen: boolean;
  live_preview: boolean;
}

export interface AudioSettings {
  input_device: string | null;
  input_device_name: string | null;
  auto_gain: boolean;
  high_pass: boolean;
  max_recording_secs: number;
  mute_system_audio: boolean;
}

export interface HotkeySettings {
  dictate: string | null;
  command: string | null;
  cancel: string | null;
  toggle_bar: string | null;
  paste_last: string | null;
  rewrite_selection: string | null;
}

export interface DictationSettings {
  hotkeys: HotkeySettings;
  activation: ActivationMode;
  hold_threshold_ms: number;
  auto_insert: boolean;
  insert_method: InsertMethod;
  restore_clipboard: boolean;
  smart_spacing: boolean;
  also_copy: boolean;
  min_duration_ms: number;
  hotkeys_paused: boolean;
}

export interface CloudSttSettings {
  provider: CloudSttProvider;
  base_url: string;
  model: string;
  timeout_secs: number;
  send_vocabulary_hint: boolean;
}

export interface EngineSettings {
  kind: EngineKind;
  local_model: string;
  language: string;
  threads: number;
  keep_loaded_minutes: number;
  preload_on_start: boolean;
  cloud: CloudSttSettings;
  fallback_to_local: boolean;
  use_hotwords: boolean;
}

export interface CleanupRules {
  remove_fillers: boolean;
  fillers: string[];
  collapse_repetitions: boolean;
  remove_false_starts: boolean;
  spoken_punctuation: boolean;
  auto_capitalize: boolean;
  fix_spacing: boolean;
  format_numbers: boolean;
  hallucination_filter: boolean;
}

export interface ModeLocal {
  final_period: FinalPeriod;
  lowercase_start: boolean;
  remove_discourse_markers: boolean;
  structure: Structure;
  capitalize: boolean;
}

export interface WritingMode {
  id: string;
  name: string;
  description: string;
  builtin: boolean;
  local: ModeLocal;
  template: string;
  ai_enabled: boolean;
  ai_instructions: string;
  temperature: number;
}

export interface WritingSettings {
  active_mode: string;
  modes: WritingMode[];
  cleanup: CleanupRules;
}

export interface VocabEntry {
  id: string;
  term: string;
  variants: string[];
  list: string;
  enabled: boolean;
  note: string;
}

export interface Replacement {
  id: string;
  from: string;
  to: string;
  whole_word: boolean;
  case_sensitive: boolean;
  regex: boolean;
  list: string;
  enabled: boolean;
}

export interface VocabularySettings {
  entries: VocabEntry[];
  replacements: Replacement[];
  lists: string[];
  disabled_lists: string[];
  phonetic_matching: boolean;
}

export interface Snippet {
  id: string;
  trigger: string;
  content: string;
  enabled: boolean;
  whole_utterance_only: boolean;
}

export type CommandAction =
  | { type: "delete_last" }
  | { type: "undo" }
  | { type: "press_enter" }
  | { type: "press_tab" }
  | { type: "new_line" }
  | { type: "new_paragraph" }
  | { type: "select_all" }
  | { type: "copy" }
  | { type: "paste" }
  | { type: "paste_last" }
  | { type: "keys"; combo: string }
  | { type: "insert_text"; text: string };

export interface VoiceCommand {
  id: string;
  phrases: string[];
  action: CommandAction;
  enabled: boolean;
  builtin: boolean;
}

export interface VoiceCommandSettings {
  enabled: boolean;
  commands: VoiceCommand[];
  send_suffix: boolean;
  send_phrases: string[];
}

export interface AppProfile {
  id: string;
  name: string;
  exe: string[];
  title_contains: string[];
  mode: string | null;
  insert_method: InsertMethod | null;
  ai_context: string;
  enabled: boolean;
}

export interface AppContextSettings {
  enabled: boolean;
  use_window_titles: boolean;
  category_modes: Partial<Record<AppCategory, string>>;
  profiles: AppProfile[];
}

export interface AiSettings {
  enabled: boolean;
  provider: AiProviderKind;
  base_url: string;
  model: string;
  timeout_secs: number;
  max_input_chars: number;
  daily_request_limit: number;
  monthly_token_limit: number;
  send_app_context: boolean;
  send_vocabulary: boolean;
  consent_given: boolean;
  price_input_per_mtok: number;
  price_output_per_mtok: number;
}

export interface PrivacySettings {
  save_history: boolean;
  history_retention_days: number;
  store_app_name: boolean;
  save_audio: boolean;
  audio_retention_days: number;
  store_raw_text: boolean;
}

export interface PerformanceSettings {
  incremental_transcription: boolean;
  segment_pause_ms: number;
  pill_fps: number;
  low_power: boolean;
  typing_wpm: number;
}

export interface Settings {
  version: number;
  onboarding_done: boolean;
  general: GeneralSettings;
  appearance: AppearanceSettings;
  audio: AudioSettings;
  dictation: DictationSettings;
  engine: EngineSettings;
  writing: WritingSettings;
  vocabulary: VocabularySettings;
  snippets: Snippet[];
  commands: VoiceCommandSettings;
  apps: AppContextSettings;
  ai: AiSettings;
  privacy: PrivacySettings;
  performance: PerformanceSettings;
}

export type PhaseName = "idle" | "recording" | "processing";

export interface Status {
  phase: PhaseName;
  hands_free: boolean;
  command: boolean;
  paused: boolean;
  queue: number;
}

export interface Bootstrap {
  settings: Settings;
  version: string;
  platform: string;
  ui_lang: "es" | "en";
  first_run: boolean;
  status: Status;
  hotkeys_available: boolean;
  hotkey_error: string | null;
  elevated: boolean;
  secrets_backend: string;
  paths: Record<string, string>;
  languages: [string, string][];
}

export interface InputDevice {
  id: string;
  name: string;
  is_default: boolean;
}

export interface ModelFile {
  archive_name: string;
  local_name: string;
  size: number;
  sha256: string | null;
}

export interface CatalogItem {
  id: string;
  name: string;
  family: "parakeet" | "whisper";
  description: string;
  archive: string;
  download_mb: number;
  ram_mb: number;
  speed: number;
  accuracy_es: number;
  languages: string[] | null;
  auto_language: boolean;
  recommended: boolean;
  files: ModelFile[];
  installed: boolean;
  disk_mb: number;
}

export interface InstalledModel {
  id: string;
  name: string;
  family: "parakeet" | "whisper";
  dir: string;
  size_bytes: number;
  custom: boolean;
}

export interface ModelsView {
  catalog: CatalogItem[];
  installed: InstalledModel[];
  loaded: string | null;
  downloading: string[];
  recommended_threads: number;
}

export interface DownloadProgress {
  model_id: string;
  phase: "downloading" | "extracting" | "done" | "failed" | "cancelled";
  done_bytes: number;
  total_bytes: number;
  message: string | null;
}

export interface HistoryEntry {
  id: number;
  created_at: string;
  raw_text: string;
  final_text: string;
  mode: string;
  app: string | null;
  language: string | null;
  engine: string;
  model: string;
  duration_ms: number;
  processing_ms: number;
  words: number;
  favorite: boolean;
  edited: boolean;
  ai_used: boolean;
  audio_path: string | null;
}

export interface HistoryQuery {
  search: string;
  favorites_only: boolean;
  oldest_first: boolean;
  limit: number;
  offset: number;
}

export interface HistoryPage {
  entries: HistoryEntry[];
  total: number;
}

export interface DayStat {
  date: string;
  words: number;
  dictations: number;
  audio_ms: number;
}

export interface Stats {
  total_words: number;
  total_dictations: number;
  total_audio_ms: number;
  today_words: number;
  week_words: number;
  minutes_saved: number;
  speaking_wpm: number;
  streak_days: number;
  last_30_days: DayStat[];
  top_apps: [string, number][];
}

export type ConflictSeverity = "error" | "warning";

export interface HotkeyConflict {
  severity: ConflictSeverity;
  message: string;
}

export interface HotkeyCheck {
  valid: boolean;
  display: string;
  conflicts: HotkeyConflict[];
}

export type AiAction =
  | { type: "mode"; mode_id: string }
  | { type: "fix_grammar" }
  | { type: "summarize" }
  | { type: "professional" }
  | { type: "structure" }
  | { type: "tasks" }
  | { type: "tone"; tone: string }
  | { type: "translate"; language: string }
  | { type: "command"; instruction: string };

export interface AiUsage {
  day: string;
  day_requests: number;
  month: string;
  month_requests: number;
  month_input_tokens: number;
  month_output_tokens: number;
  total_requests: number;
  total_input_tokens: number;
  total_output_tokens: number;
  month_cost_usd: number;
  failures: number;
}

export interface ProcessUsage {
  working_set: number;
  private_bytes: number;
  cpu_time_ms: number;
  threads: number;
}

export interface Diagnostics {
  version: string;
  os: string;
  uptime_secs: number;
  process: ProcessUsage;
  cpu_percent: number;
  engine: { loaded_model: string | null; load_ms: number | null; idle_secs: number; threads: number };
  engine_kind: string;
  model_installed: boolean;
  hotkeys_running: boolean;
  hotkey_error: string | null;
  elevated: boolean;
  devices: number;
  network_features: string[];
  hub_open: boolean;
  threads_available: number;
}

export interface StorageInfo {
  history_entries: number;
  history_bytes: number;
  audio_bytes: number;
  models_bytes: number;
  logs_bytes: number;
  config_bytes: number;
}

export interface RecordTest {
  text: string;
  duration_ms: number;
  peak_db: number;
  noise_db: number;
  speech_ms: number;
  engine_ms: number;
  device: string;
}

export interface ProcessPreview {
  text: string;
  command: string | null;
  ai_used: boolean;
  ai_error: string | null;
}

export interface UpdateInfo {
  current: string;
  latest: string | null;
  newer: boolean;
  url: string;
}

export interface PillPreview {
  width: number;
  height: number;
  rgba: string;
}
