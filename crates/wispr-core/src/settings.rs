//! Configuración persistente de Wispr Swag.
//!
//! Todos los campos tienen valores por defecto (`#[serde(default)]`), de modo que un
//! archivo antiguo o incompleto se carga sin errores y las claves nuevas toman su
//! valor inicial. Las credenciales **nunca** se guardan aquí (ver [`crate::secrets`]).

use crate::appctx::{AppCategory, AppProfile};
use crate::hotkey::{Hotkey, Mods};
use crate::paths::{write_atomic, AppPaths};
use crate::text::commands::{default_commands, VoiceCommand};
use crate::text::modes::{builtin_modes, WritingMode};
use crate::text::snippets::Snippet;
use crate::text::vocab::{Replacement, VocabEntry};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub onboarding_done: bool,
    pub general: GeneralSettings,
    pub appearance: AppearanceSettings,
    pub audio: AudioSettings,
    pub dictation: DictationSettings,
    pub engine: EngineSettings,
    pub writing: WritingSettings,
    pub vocabulary: VocabularySettings,
    pub snippets: Vec<Snippet>,
    pub commands: VoiceCommandSettings,
    pub apps: AppContextSettings,
    pub ai: AiSettings,
    pub privacy: PrivacySettings,
    pub performance: PerformanceSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            onboarding_done: false,
            general: Default::default(),
            appearance: Default::default(),
            audio: Default::default(),
            dictation: Default::default(),
            engine: Default::default(),
            writing: Default::default(),
            vocabulary: Default::default(),
            snippets: Vec::new(),
            commands: Default::default(),
            apps: Default::default(),
            ai: Default::default(),
            privacy: Default::default(),
            performance: Default::default(),
        }
    }
}

// ---------------------------------------------------------------- General

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum UiLanguage {
    #[default]
    Auto,
    Es,
    En,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehavior {
    /// Cierra la ventana y libera su memoria; la app sigue en la bandeja.
    #[default]
    ReleaseMemory,
    /// Oculta la ventana (se reabre al instante, pero ocupa memoria).
    HideToTray,
    /// Sale por completo de la aplicación.
    Quit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GeneralSettings {
    pub ui_language: UiLanguage,
    pub launch_at_startup: bool,
    /// Al iniciar, no abrir la ventana principal (solo bandeja y barra).
    pub start_minimized: bool,
    pub close_behavior: CloseBehavior,
    /// Notificaciones del sistema (además de los avisos en la barra).
    pub system_notifications: bool,
    pub sounds: bool,
    /// 0.0–1.0
    pub sound_volume: f32,
    /// Comprobar si hay versiones nuevas (consulta GitHub Releases).
    pub check_updates: bool,
    /// Copia de seguridad automática diaria de los ajustes.
    pub auto_backup: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            ui_language: UiLanguage::Auto,
            launch_at_startup: true,
            start_minimized: true,
            close_behavior: CloseBehavior::ReleaseMemory,
            system_notifications: false,
            sounds: true,
            sound_volume: 0.35,
            check_updates: false,
            auto_backup: true,
        }
    }
}

// ------------------------------------------------------------- Apariencia

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PillSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl PillSize {
    pub fn scale(self) -> f32 {
        match self {
            PillSize::Small => 0.86,
            PillSize::Medium => 1.0,
            PillSize::Large => 1.2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PillAnchor {
    #[default]
    BottomCenter,
    BottomLeft,
    BottomRight,
    TopCenter,
    /// Posición elegida arrastrando la barra.
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MonitorChoice {
    /// Monitor de la ventana activa.
    #[default]
    Active,
    /// Monitor donde está el puntero.
    Cursor,
    Primary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum InfoLevel {
    /// Solo onda de voz.
    Minimal,
    /// Onda + botones de detener/cancelar.
    #[default]
    Standard,
    /// Además: temporizador, modo e idioma.
    Detailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: Theme,
    pub pill_size: PillSize,
    /// 0.5–1.0
    pub pill_opacity: f32,
    pub pill_anchor: PillAnchor,
    /// Posición personalizada, relativa al área de trabajo del monitor (0–1).
    pub pill_custom_x: f32,
    pub pill_custom_y: f32,
    pub pill_monitor: MonitorChoice,
    /// Mostrar una mini barra en reposo (clic para dictar).
    pub show_idle_pill: bool,
    pub animations: bool,
    pub info_level: InfoLevel,
    /// Ocultar la barra sobre juegos y aplicaciones a pantalla completa.
    pub hide_in_fullscreen: bool,
    /// Vista previa del texto reconocido durante el dictado.
    pub live_preview: bool,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            pill_size: PillSize::Medium,
            pill_opacity: 0.94,
            pill_anchor: PillAnchor::BottomCenter,
            pill_custom_x: 0.5,
            pill_custom_y: 0.95,
            pill_monitor: MonitorChoice::Active,
            show_idle_pill: true,
            animations: true,
            info_level: InfoLevel::Standard,
            hide_in_fullscreen: true,
            live_preview: false,
        }
    }
}

// ------------------------------------------------------------------ Audio

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AudioSettings {
    /// Identificador estable del micrófono (cpal). `None` = predeterminado del sistema.
    pub input_device: Option<String>,
    /// Nombre visible guardado para mostrarlo aunque el dispositivo no esté conectado.
    pub input_device_name: Option<String>,
    /// Normaliza automáticamente el volumen de la voz antes de transcribir.
    pub auto_gain: bool,
    /// Filtro paso alto para eliminar zumbidos y ruido de baja frecuencia.
    pub high_pass: bool,
    /// Duración máxima de un dictado (segundos).
    pub max_recording_secs: u32,
    /// Silenciar el audio del sistema mientras se dicta (Windows).
    pub mute_system_audio: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            input_device: None,
            input_device_name: None,
            auto_gain: true,
            high_pass: true,
            max_recording_secs: 600,
            mute_system_audio: false,
        }
    }
}

// ---------------------------------------------------------------- Dictado

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ActivationMode {
    /// Mantener = pulsar para hablar; toque corto = manos libres.
    #[default]
    Hybrid,
    /// Pulsar para empezar, pulsar de nuevo para terminar.
    Toggle,
    /// Solo mientras se mantiene pulsado.
    Hold,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum InsertMethod {
    /// Portapapeles + Ctrl+V (el más compatible; restaura el portapapeles).
    #[default]
    Paste,
    /// Portapapeles + Mayús+Insert (terminales antiguos).
    PasteShiftInsert,
    /// Escritura carácter a carácter (SendInput Unicode).
    Type,
    /// Solo copiar al portapapeles.
    ClipboardOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct HotkeySettings {
    pub dictate: Option<Hotkey>,
    pub command: Option<Hotkey>,
    pub cancel: Option<Hotkey>,
    pub toggle_bar: Option<Hotkey>,
    pub paste_last: Option<Hotkey>,
    pub rewrite_selection: Option<Hotkey>,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            dictate: Some(Hotkey::chord(Mods::CTRL | Mods::WIN)),
            command: Some(Hotkey::chord(Mods::CTRL | Mods::WIN | Mods::ALT)),
            cancel: Some(Hotkey::new(Mods::empty(), crate::hotkey::vk::ESCAPE)),
            toggle_bar: Some(Hotkey::new(Mods::ALT | Mods::SHIFT, b'B' as u16)),
            paste_last: Some(Hotkey::new(Mods::ALT | Mods::SHIFT, b'Z' as u16)),
            rewrite_selection: Some(Hotkey::new(Mods::ALT | Mods::SHIFT, b'R' as u16)),
        }
    }
}

impl HotkeySettings {
    pub fn named(&self) -> Vec<(&'static str, Option<Hotkey>)> {
        vec![
            ("dictate", self.dictate),
            ("command", self.command),
            ("cancel", self.cancel),
            ("toggle_bar", self.toggle_bar),
            ("paste_last", self.paste_last),
            ("rewrite_selection", self.rewrite_selection),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DictationSettings {
    pub hotkeys: HotkeySettings,
    pub activation: ActivationMode,
    /// Umbral (ms) que separa «toque» de «mantener» en modo híbrido.
    pub hold_threshold_ms: u32,
    /// Insertar automáticamente el texto en la aplicación activa.
    pub auto_insert: bool,
    pub insert_method: InsertMethod,
    /// Restaurar el contenido previo del portapapeles tras pegar.
    pub restore_clipboard: bool,
    /// Añadir un espacio entre dictados consecutivos en el mismo campo.
    pub smart_spacing: bool,
    /// Copiar también el resultado al portapapeles (además de insertarlo).
    pub also_copy: bool,
    /// Dictados más cortos que esto (ms) se descartan como pulsaciones accidentales.
    pub min_duration_ms: u32,
    /// Atajos en pausa (p. ej. mientras se juega).
    pub hotkeys_paused: bool,
}

impl Default for DictationSettings {
    fn default() -> Self {
        Self {
            hotkeys: HotkeySettings::default(),
            activation: ActivationMode::Hybrid,
            hold_threshold_ms: 350,
            auto_insert: true,
            insert_method: InsertMethod::Paste,
            restore_clipboard: true,
            smart_spacing: true,
            also_copy: false,
            min_duration_ms: 300,
            hotkeys_paused: false,
        }
    }
}

// ----------------------------------------------------------------- Motor

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    /// Reconocimiento en el propio equipo (sin conexión).
    #[default]
    Local,
    /// Servicio en la nube compatible con la API de OpenAI.
    Cloud,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CloudSttProvider {
    #[default]
    OpenAi,
    Groq,
    /// Cualquier servidor con `/v1/audio/transcriptions` (p. ej. speaches, LocalAI).
    Custom,
}

impl CloudSttProvider {
    pub fn default_base_url(self) -> &'static str {
        match self {
            CloudSttProvider::OpenAi => "https://api.openai.com/v1",
            CloudSttProvider::Groq => "https://api.groq.com/openai/v1",
            CloudSttProvider::Custom => "http://localhost:8000/v1",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            CloudSttProvider::OpenAi => "gpt-4o-transcribe",
            CloudSttProvider::Groq => "whisper-large-v3-turbo",
            CloudSttProvider::Custom => "whisper-1",
        }
    }

    pub fn secret_key(self) -> &'static str {
        match self {
            CloudSttProvider::OpenAi => "stt:openai",
            CloudSttProvider::Groq => "stt:groq",
            CloudSttProvider::Custom => "stt:custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CloudSttSettings {
    pub provider: CloudSttProvider,
    /// Vacío = URL por defecto del proveedor.
    pub base_url: String,
    /// Vacío = modelo por defecto del proveedor.
    pub model: String,
    pub timeout_secs: u32,
    /// Enviar el vocabulario personal como pista de contexto (`prompt`).
    pub send_vocabulary_hint: bool,
}

impl Default for CloudSttSettings {
    fn default() -> Self {
        Self {
            provider: CloudSttProvider::OpenAi,
            base_url: String::new(),
            model: String::new(),
            timeout_secs: 30,
            send_vocabulary_hint: true,
        }
    }
}

impl CloudSttSettings {
    pub fn effective_base_url(&self) -> String {
        if self.base_url.trim().is_empty() {
            self.provider.default_base_url().to_string()
        } else {
            self.base_url.trim().trim_end_matches('/').to_string()
        }
    }

    pub fn effective_model(&self) -> String {
        if self.model.trim().is_empty() {
            self.provider.default_model().to_string()
        } else {
            self.model.trim().to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EngineSettings {
    pub kind: EngineKind,
    /// Identificador del modelo local del catálogo (o importado).
    pub local_model: String,
    /// Código ISO 639-1 (`es`, `en`, …) o `auto`.
    pub language: String,
    /// Hilos de CPU para la inferencia local (0 = automático).
    pub threads: u32,
    /// Minutos que el modelo permanece en memoria tras el último uso (0 = liberar al terminar).
    pub keep_loaded_minutes: u32,
    /// Cargar el modelo al iniciar la aplicación en lugar de al primer dictado.
    pub preload_on_start: bool,
    pub cloud: CloudSttSettings,
    /// Si la nube falla y hay un modelo local instalado, transcribir en local.
    pub fallback_to_local: bool,
    /// Usar el vocabulario como «palabras clave» del motor cuando lo admita.
    pub use_hotwords: bool,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            kind: EngineKind::Local,
            local_model: "parakeet-tdt-0.6b-v3".into(),
            language: "es".into(),
            threads: 0,
            keep_loaded_minutes: 10,
            preload_on_start: false,
            cloud: CloudSttSettings::default(),
            fallback_to_local: true,
            use_hotwords: true,
        }
    }
}

// --------------------------------------------------------------- Escritura

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CleanupRules {
    pub remove_fillers: bool,
    pub fillers: Vec<String>,
    pub collapse_repetitions: bool,
    pub remove_false_starts: bool,
    /// «nueva línea», «punto y aparte», «abre paréntesis»…
    pub spoken_punctuation: bool,
    pub auto_capitalize: bool,
    pub fix_spacing: bool,
    /// «50 por ciento» → «50 %», «3 euros» → «3 €».
    pub format_numbers: bool,
    /// Descarta frases fantasma típicas de los modelos («Subtítulos realizados por…»).
    pub hallucination_filter: bool,
}

pub fn default_fillers() -> Vec<String> {
    ["eh", "ehm", "em", "emm", "mmm", "mm", "hmm", "uh", "um", "umm", "erm"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Default for CleanupRules {
    fn default() -> Self {
        Self {
            remove_fillers: true,
            fillers: default_fillers(),
            collapse_repetitions: true,
            remove_false_starts: true,
            spoken_punctuation: true,
            auto_capitalize: true,
            fix_spacing: true,
            format_numbers: false,
            hallucination_filter: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WritingSettings {
    pub active_mode: String,
    pub modes: Vec<WritingMode>,
    pub cleanup: CleanupRules,
}

impl Default for WritingSettings {
    fn default() -> Self {
        Self {
            active_mode: "natural".into(),
            modes: builtin_modes(),
            cleanup: CleanupRules::default(),
        }
    }
}

impl WritingSettings {
    pub fn mode(&self, id: &str) -> Option<&WritingMode> {
        self.modes.iter().find(|m| m.id == id)
    }

    /// Modo activo (o «natural» si el identificador ya no existe).
    pub fn active(&self) -> WritingMode {
        self.mode(&self.active_mode)
            .or_else(|| self.mode("natural"))
            .cloned()
            .unwrap_or_else(|| builtin_modes().remove(0))
    }
}

// ------------------------------------------------------------- Vocabulario

pub fn default_vocab_lists() -> Vec<String> {
    ["General", "Informática", "Programación", "Trabajo", "Estudios"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct VocabularySettings {
    pub entries: Vec<VocabEntry>,
    pub replacements: Vec<Replacement>,
    pub lists: Vec<String>,
    /// Listas desactivadas (sus entradas no se aplican).
    pub disabled_lists: Vec<String>,
    /// Corrección por similitud fonética (español) hacia los términos del vocabulario.
    pub phonetic_matching: bool,
}

impl Default for VocabularySettings {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            replacements: Vec::new(),
            lists: default_vocab_lists(),
            disabled_lists: Vec::new(),
            phonetic_matching: true,
        }
    }
}

impl VocabularySettings {
    pub fn list_enabled(&self, list: &str) -> bool {
        !self.disabled_lists.iter().any(|l| l == list)
    }

    pub fn active_entries(&self) -> impl Iterator<Item = &VocabEntry> {
        self.entries.iter().filter(|e| e.enabled && self.list_enabled(&e.list))
    }

    pub fn active_replacements(&self) -> impl Iterator<Item = &Replacement> {
        self.replacements.iter().filter(|r| r.enabled && self.list_enabled(&r.list))
    }
}

// ---------------------------------------------------------- Comandos de voz

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct VoiceCommandSettings {
    pub enabled: bool,
    pub commands: Vec<VoiceCommand>,
    /// Si el dictado termina en «pulsa intro», se inserta el texto y se pulsa Intro.
    pub send_suffix: bool,
    pub send_phrases: Vec<String>,
}

impl Default for VoiceCommandSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            commands: default_commands(),
            send_suffix: false,
            send_phrases: ["pulsa intro", "pulsa enter", "press enter"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

// --------------------------------------------------- Contexto de aplicación

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppContextSettings {
    pub enabled: bool,
    /// Usar el título de la ventana del navegador para reconocer sitios (Gmail, Slack…).
    /// Se procesa en memoria y nunca se guarda.
    pub use_window_titles: bool,
    /// Modo de escritura por categoría de aplicación.
    pub category_modes: BTreeMap<AppCategory, String>,
    /// Reglas específicas del usuario (tienen prioridad).
    pub profiles: Vec<AppProfile>,
}

impl Default for AppContextSettings {
    fn default() -> Self {
        let mut category_modes = BTreeMap::new();
        category_modes.insert(AppCategory::Code, "code".to_string());
        category_modes.insert(AppCategory::Terminal, "code".to_string());
        category_modes.insert(AppCategory::Email, "email".to_string());
        category_modes.insert(AppCategory::Chat, "messages".to_string());
        category_modes.insert(AppCategory::Notes, "notes".to_string());
        Self {
            enabled: true,
            use_window_titles: false,
            category_modes,
            profiles: Vec::new(),
        }
    }
}

// --------------------------------------------------------------------- IA

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiProviderKind {
    #[default]
    OpenAi,
    Anthropic,
    Groq,
    OpenRouter,
    Mistral,
    Gemini,
    /// Ollama local (sin conexión, sin clave).
    Ollama,
    /// LM Studio local.
    LmStudio,
    /// Cualquier servidor compatible con `/v1/chat/completions`.
    Custom,
}

impl AiProviderKind {
    pub fn default_base_url(self) -> &'static str {
        match self {
            AiProviderKind::OpenAi => "https://api.openai.com/v1",
            AiProviderKind::Anthropic => "https://api.anthropic.com/v1",
            AiProviderKind::Groq => "https://api.groq.com/openai/v1",
            AiProviderKind::OpenRouter => "https://openrouter.ai/api/v1",
            AiProviderKind::Mistral => "https://api.mistral.ai/v1",
            AiProviderKind::Gemini => "https://generativelanguage.googleapis.com/v1beta/openai",
            AiProviderKind::Ollama => "http://localhost:11434/v1",
            AiProviderKind::LmStudio => "http://localhost:1234/v1",
            AiProviderKind::Custom => "http://localhost:8080/v1",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            AiProviderKind::OpenAi => "gpt-5-mini",
            AiProviderKind::Anthropic => "claude-opus-5-5",
            AiProviderKind::Groq => "llama-3.3-70b-versatile",
            AiProviderKind::OpenRouter => "openai/gpt-5-mini",
            AiProviderKind::Mistral => "mistral-small-latest",
            AiProviderKind::Gemini => "gemini-2.5-flash",
            AiProviderKind::Ollama => "llama3.2",
            AiProviderKind::LmStudio => "local-model",
            AiProviderKind::Custom => "",
        }
    }

    /// `true` si el proveedor se ejecuta en el propio equipo.
    pub fn is_local(self) -> bool {
        matches!(self, AiProviderKind::Ollama | AiProviderKind::LmStudio)
    }

    pub fn needs_key(self) -> bool {
        !self.is_local() && self != AiProviderKind::Custom
    }

    pub fn secret_key(self) -> String {
        format!("ai:{}", serde_json::to_value(self).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AiSettings {
    /// Interruptor general del procesamiento con IA.
    pub enabled: bool,
    pub provider: AiProviderKind,
    /// Vacío = URL por defecto del proveedor.
    pub base_url: String,
    /// Vacío = modelo por defecto del proveedor.
    pub model: String,
    pub timeout_secs: u32,
    /// Límite de caracteres enviados por solicitud.
    pub max_input_chars: u32,
    /// Límite de solicitudes por día (0 = sin límite).
    pub daily_request_limit: u32,
    /// Límite de tokens al mes (0 = sin límite).
    pub monthly_token_limit: u64,
    /// Incluir la categoría de la aplicación activa (p. ej. «correo») en la solicitud.
    pub send_app_context: bool,
    /// Incluir los términos del vocabulario para respetar su ortografía.
    pub send_vocabulary: bool,
    /// El usuario ha leído y aceptado qué datos se envían.
    pub consent_given: bool,
    /// Precio orientativo (USD por millón de tokens) para estimar el gasto.
    pub price_input_per_mtok: f64,
    pub price_output_per_mtok: f64,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: AiProviderKind::OpenAi,
            base_url: String::new(),
            model: String::new(),
            timeout_secs: 12,
            max_input_chars: 8000,
            daily_request_limit: 500,
            monthly_token_limit: 0,
            send_app_context: true,
            send_vocabulary: true,
            consent_given: false,
            price_input_per_mtok: 0.0,
            price_output_per_mtok: 0.0,
        }
    }
}

impl AiSettings {
    pub fn effective_base_url(&self) -> String {
        if self.base_url.trim().is_empty() {
            self.provider.default_base_url().to_string()
        } else {
            self.base_url.trim().trim_end_matches('/').to_string()
        }
    }

    pub fn effective_model(&self) -> String {
        if self.model.trim().is_empty() {
            self.provider.default_model().to_string()
        } else {
            self.model.trim().to_string()
        }
    }

    /// La IA puede usarse: activada y con consentimiento informado.
    pub fn usable(&self) -> bool {
        self.enabled && self.consent_given
    }
}

// --------------------------------------------------------------- Privacidad

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PrivacySettings {
    pub save_history: bool,
    /// Días que se conservan las transcripciones (0 = siempre).
    pub history_retention_days: u32,
    /// Guardar el nombre de la aplicación donde se dictó.
    pub store_app_name: bool,
    /// Guardar el audio de cada dictado (desactivado por defecto).
    pub save_audio: bool,
    pub audio_retention_days: u32,
    /// Guardar el texto original reconocido además del texto final.
    pub store_raw_text: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            save_history: true,
            history_retention_days: 90,
            store_app_name: true,
            save_audio: false,
            audio_retention_days: 7,
            store_raw_text: true,
        }
    }
}

// -------------------------------------------------------------- Rendimiento

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PerformanceSettings {
    /// Transcribir por frases durante la grabación (menos espera en dictados largos).
    pub incremental_transcription: bool,
    /// Pausa (ms) que delimita una frase para la transcripción incremental.
    pub segment_pause_ms: u32,
    /// Fotogramas por segundo de la animación de la barra.
    pub pill_fps: u32,
    /// Modo de bajo consumo: menos hilos y sin animaciones.
    pub low_power: bool,
    /// Velocidad de escritura del usuario (ppm) para estimar el tiempo ahorrado.
    pub typing_wpm: u32,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            incremental_transcription: true,
            segment_pause_ms: 900,
            pill_fps: 30,
            low_power: false,
            typing_wpm: 40,
        }
    }
}

// ------------------------------------------------------------- Persistencia

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("no se pudo leer o escribir los ajustes: {0}")]
    Io(#[from] std::io::Error),
    #[error("el archivo de ajustes no es válido: {0}")]
    Parse(#[from] serde_json::Error),
}

impl Settings {
    /// Carga los ajustes. Si el archivo está dañado, se conserva una copia
    /// (`settings.corrupt.json`) y se devuelven los valores por defecto.
    pub fn load(paths: &AppPaths) -> Settings {
        let file = paths.settings_file();
        match std::fs::read(&file) {
            Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
                Ok(mut s) => {
                    s.migrate();
                    s.sanitize();
                    s
                }
                Err(e) => {
                    tracing::error!("ajustes dañados ({e}); se restauran los valores por defecto");
                    let _ = std::fs::copy(&file, file.with_extension("corrupt.json"));
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, paths: &AppPaths) -> Result<(), SettingsError> {
        let json = serde_json::to_vec_pretty(self)?;
        write_atomic(&paths.settings_file(), &json)?;
        Ok(())
    }

    /// Exporta los ajustes (sin credenciales: nunca están aquí) a un archivo.
    pub fn export_to(&self, path: &Path) -> Result<(), SettingsError> {
        let json = serde_json::to_vec_pretty(self)?;
        write_atomic(path, &json)?;
        Ok(())
    }

    pub fn import_from(path: &Path) -> Result<Settings, SettingsError> {
        let bytes = std::fs::read(path)?;
        let mut s: Settings = serde_json::from_slice(&bytes)?;
        s.migrate();
        s.sanitize();
        Ok(s)
    }

    /// Copia de seguridad con marca de fecha; conserva las `keep` más recientes.
    pub fn backup(&self, paths: &AppPaths, keep: usize) -> Result<std::path::PathBuf, SettingsError> {
        let dir = paths.backups_dir();
        std::fs::create_dir_all(&dir)?;
        let name = format!("settings-{}.json", chrono::Local::now().format("%Y%m%d-%H%M%S"));
        let path = dir.join(name);
        self.export_to(&path)?;
        let mut files: Vec<_> = std::fs::read_dir(&dir)?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        files.sort();
        while files.len() > keep {
            let old = files.remove(0);
            let _ = std::fs::remove_file(old);
        }
        Ok(path)
    }

    /// Fecha de la copia de seguridad más reciente, si existe.
    pub fn last_backup_date(paths: &AppPaths) -> Option<String> {
        let mut files: Vec<_> = std::fs::read_dir(paths.backups_dir())
            .ok()?
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(String::from))
            .filter(|n| n.starts_with("settings-"))
            .collect();
        files.sort();
        files.pop().map(|n| n.trim_start_matches("settings-").chars().take(8).collect())
    }

    fn migrate(&mut self) {
        // Asegura que existan todos los modos integrados (versiones nuevas pueden añadir modos).
        for m in builtin_modes() {
            if !self.writing.modes.iter().any(|x| x.id == m.id) {
                self.writing.modes.push(m);
            }
        }
        for c in default_commands() {
            if c.builtin && !self.commands.commands.iter().any(|x| x.id == c.id) {
                self.commands.commands.push(c);
            }
        }
        self.version = SETTINGS_VERSION;
    }

    /// Acota valores numéricos a rangos razonables.
    pub fn sanitize(&mut self) {
        self.general.sound_volume = self.general.sound_volume.clamp(0.0, 1.0);
        self.appearance.pill_opacity = self.appearance.pill_opacity.clamp(0.5, 1.0);
        self.appearance.pill_custom_x = self.appearance.pill_custom_x.clamp(0.0, 1.0);
        self.appearance.pill_custom_y = self.appearance.pill_custom_y.clamp(0.0, 1.0);
        self.audio.max_recording_secs = self.audio.max_recording_secs.clamp(10, 3600);
        self.dictation.hold_threshold_ms = self.dictation.hold_threshold_ms.clamp(150, 1000);
        self.dictation.min_duration_ms = self.dictation.min_duration_ms.clamp(0, 2000);
        self.engine.threads = self.engine.threads.min(64);
        self.engine.cloud.timeout_secs = self.engine.cloud.timeout_secs.clamp(5, 300);
        self.ai.timeout_secs = self.ai.timeout_secs.clamp(3, 120);
        self.ai.max_input_chars = self.ai.max_input_chars.clamp(200, 100_000);
        self.performance.segment_pause_ms = self.performance.segment_pause_ms.clamp(400, 3000);
        self.performance.pill_fps = self.performance.pill_fps.clamp(10, 60);
        self.performance.typing_wpm = self.performance.typing_wpm.clamp(10, 200);
        if self.engine.language.trim().is_empty() {
            self.engine.language = "es".into();
        }
        if self.writing.mode(&self.writing.active_mode).is_none() {
            self.writing.active_mode = "natural".into();
        }
        for l in default_vocab_lists() {
            if !self.vocabulary.lists.contains(&l) {
                self.vocabulary.lists.push(l);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_roundtrip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn partial_file_gets_defaults() {
        let back: Settings = serde_json::from_str(r#"{"general":{"sounds":false}}"#).unwrap();
        assert!(!back.general.sounds);
        assert_eq!(back.engine.language, "es");
        assert_eq!(back.dictation.hotkeys.dictate.unwrap().to_string(), "Ctrl+Win");
    }

    #[test]
    fn save_load_backup_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::portable(dir.path().to_path_buf());
        paths.ensure().unwrap();
        let mut s = Settings::default();
        s.engine.language = "en".into();
        s.appearance.pill_opacity = 7.0; // fuera de rango
        s.save(&paths).unwrap();
        let loaded = Settings::load(&paths);
        assert_eq!(loaded.engine.language, "en");
        assert_eq!(loaded.appearance.pill_opacity, 1.0);
        for _ in 0..3 {
            s.backup(&paths, 2).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1100));
        }
        let count = std::fs::read_dir(paths.backups_dir()).unwrap().count();
        assert_eq!(count, 2);
        assert!(Settings::last_backup_date(&paths).is_some());
    }

    #[test]
    fn corrupt_file_is_preserved_and_defaults_used() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::portable(dir.path().to_path_buf());
        paths.ensure().unwrap();
        std::fs::write(paths.settings_file(), b"{ no es json").unwrap();
        let s = Settings::load(&paths);
        assert_eq!(s, Settings::default());
        assert!(paths.settings_file().with_extension("corrupt.json").exists());
    }

    #[test]
    fn ai_secret_keys() {
        assert_eq!(AiProviderKind::OpenAi.secret_key(), "ai:open_ai");
        assert_eq!(AiProviderKind::Anthropic.secret_key(), "ai:anthropic");
        assert!(!AiProviderKind::Ollama.needs_key());
    }
}
