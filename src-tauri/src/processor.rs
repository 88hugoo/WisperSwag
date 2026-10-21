//! Transcripción y procesamiento de texto (motor de voz → reglas locales → IA).

use crate::platform::Foreground;
use crate::state::AppState;
use std::sync::Arc;
use std::time::Duration;
use wispr_core::appctx::{self, AppDecision};
use wispr_core::audio::Clip;
use wispr_core::llm::prompts::{self, AiAction, PromptContext};
use wispr_core::llm::{self, LlmError};
use wispr_core::settings::{EngineKind, Settings};
use wispr_core::stt::catalog::{self, InstalledModel};
use wispr_core::stt::{cloud, SttError, SttOptions, SttResult};
use wispr_core::text::commands::CommandAction;
use wispr_core::text::modes::WritingMode;
use wispr_core::text::pipeline::{self, LocalContext, LocalOutcome};
use wispr_core::text::vocab;

/// Modelo local a usar: el elegido o, si no está instalado, el primero disponible.
pub fn local_model(state: &AppState, settings: &Settings) -> Option<InstalledModel> {
    let dir = state.paths.models_dir();
    catalog::find_installed(&dir, &settings.engine.local_model).or_else(|| catalog::installed(&dir).into_iter().next())
}

fn stt_options(settings: &Settings, previous: Option<String>) -> SttOptions {
    let hints = if settings.engine.use_hotwords {
        vocab::hint_terms(settings.vocabulary.active_entries(), 120)
    } else {
        Vec::new()
    };
    SttOptions { language: settings.engine.language.clone(), hints, previous_text: previous }
}

/// Programa la liberación del modelo tras el tiempo de inactividad configurado.
pub fn schedule_unload(state: &Arc<AppState>) {
    let minutes = state.settings.read().engine.keep_loaded_minutes;
    let engine = state.engine.clone();
    if minutes == 0 {
        engine.unload();
        return;
    }
    if minutes >= 24 * 60 {
        return; // «siempre cargado»
    }
    let keep = Duration::from_secs(minutes as u64 * 60);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(keep + Duration::from_secs(1)).await;
        engine.unload_if_idle(keep);
    });
}

/// Carga el modelo local en segundo plano (p. ej. mientras el usuario habla).
pub fn preload(state: &Arc<AppState>) {
    let settings = state.settings();
    if settings.engine.kind != EngineKind::Local {
        return;
    }
    let Some(model) = local_model(state, &settings) else { return };
    let engine = state.engine.clone();
    let lang = settings.engine.language.clone();
    let threads = effective_threads(&settings);
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = engine.ensure_loaded(&model, &lang, threads) {
            tracing::warn!("no se pudo precargar el modelo: {e}");
        }
    });
}

pub fn effective_threads(settings: &Settings) -> u32 {
    let t = settings.engine.threads;
    if settings.performance.low_power {
        return 2;
    }
    t
}

async fn transcribe_local(state: &Arc<AppState>, settings: &Settings, clip: Clip, previous: Option<String>) -> Result<SttResult, SttError> {
    let model = local_model(state, settings).ok_or_else(|| SttError::ModelMissing(settings.engine.local_model.clone()))?;
    let engine = state.engine.clone();
    let opts = stt_options(settings, previous);
    let threads = effective_threads(settings);
    let r = tauri::async_runtime::spawn_blocking(move || engine.transcribe(&model, &clip, &opts, threads))
        .await
        .map_err(|e| SttError::Other(e.to_string()))?;
    schedule_unload(state);
    r
}

/// Transcribe un fragmento con el motor configurado (y respaldo local si procede).
pub async fn transcribe(state: &Arc<AppState>, clip: Clip, previous: Option<String>) -> Result<SttResult, SttError> {
    let settings = state.settings();
    match settings.engine.kind {
        EngineKind::Local => transcribe_local(state, &settings, clip, previous).await,
        EngineKind::Cloud => {
            let key = state.secret(settings.engine.cloud.provider.secret_key());
            let opts = stt_options(&settings, previous.clone());
            match cloud::transcribe(&settings.engine.cloud, key.as_deref(), &clip, &opts).await {
                Ok(r) => Ok(r),
                Err(e) if settings.engine.fallback_to_local && (e.is_transient() || e == SttError::MissingKey) => {
                    if local_model(state, &settings).is_some() {
                        tracing::warn!("la nube falló ({e}); se usa el motor local");
                        transcribe_local(state, &settings, clip, previous).await
                    } else {
                        Err(e)
                    }
                }
                Err(e) => Err(e),
            }
        }
    }
}

/// Contexto de la aplicación de destino.
pub fn decide(settings: &Settings, target: &Foreground) -> AppDecision {
    appctx::resolve(settings, &target.app)
}

/// Modo de escritura para un dictado.
pub fn mode_for(settings: &Settings, decision: &AppDecision, override_mode: Option<&str>) -> WritingMode {
    override_mode
        .and_then(|id| settings.writing.mode(id).cloned())
        .or_else(|| decision.mode_id.as_deref().and_then(|id| settings.writing.mode(id).cloned()))
        .unwrap_or_else(|| settings.writing.active())
}

/// Idioma del texto para las reglas locales.
pub fn text_lang(settings: &Settings, detected: Option<&str>) -> String {
    match detected {
        Some(l) if !l.is_empty() => l.to_string(),
        _ if settings.engine.language != "auto" => settings.engine.language.clone(),
        _ => "es".into(),
    }
}

pub struct TextResult {
    pub outcome: Outcome,
    pub mode_id: String,
    pub ai_used: bool,
    pub ai_error: Option<LlmError>,
}

pub enum Outcome {
    Empty,
    Command(CommandAction),
    Text { text: String, press_enter: bool },
}

fn prompt_context(settings: &Settings, decision: &AppDecision) -> PromptContext {
    PromptContext {
        app_hint: settings.ai.send_app_context.then(|| decision.category.ai_hint().to_string()),
        app_extra: decision.ai_context.clone(),
        vocabulary: if settings.ai.send_vocabulary {
            vocab::hint_terms(settings.vocabulary.active_entries(), 150)
        } else {
            Vec::new()
        },
    }
}

/// Envía una solicitud de IA respetando límites y registrando el uso.
pub async fn ai_request(state: &Arc<AppState>, req: llm::LlmRequest) -> Result<String, LlmError> {
    let settings = state.settings();
    let key = state.secret(&settings.ai.provider.secret_key());
    llm::check_ready(&settings.ai, key.as_deref())?;
    state.usage.lock().data.check(&settings.ai)?;
    match llm::complete(&settings.ai, key.as_deref(), &req).await {
        Ok(r) => {
            {
                let mut u = state.usage.lock();
                u.data.record(&settings.ai, &r);
                u.save();
            }
            Ok(r.text)
        }
        Err(e) => {
            let mut u = state.usage.lock();
            u.data.record_failure();
            u.save();
            Err(e)
        }
    }
}

/// ¿Se aplicará IA a este modo?
pub fn ai_applies(settings: &Settings, mode: &WritingMode) -> bool {
    settings.ai.usable() && mode.ai_enabled && !(mode.ai_instructions.trim().is_empty() && mode.id == "custom")
}

/// Procesa el texto reconocido. `on_ai` se llama justo antes de usar la IA (para la interfaz).
#[allow(clippy::too_many_arguments)]
pub async fn process_text(
    state: &Arc<AppState>,
    raw: &str,
    target: &Foreground,
    lang: &str,
    override_mode: Option<&str>,
    audio_had_speech: bool,
    allow_ai: bool,
    on_ai: impl FnOnce(),
) -> TextResult {
    let settings = state.settings();
    let decision = decide(&settings, target);
    let mode = mode_for(&settings, &decision, override_mode);
    let ctx = LocalContext { settings: &settings, mode: &mode, lang, audio_had_speech };
    let prepared = match pipeline::prepare(raw, &ctx) {
        LocalOutcome::Empty => return TextResult { outcome: Outcome::Empty, mode_id: mode.id.clone(), ai_used: false, ai_error: None },
        LocalOutcome::Command(c) => {
            return TextResult { outcome: Outcome::Command(c), mode_id: mode.id.clone(), ai_used: false, ai_error: None }
        }
        LocalOutcome::Text(p) => p,
    };
    let mut ai_used = false;
    let mut ai_error = None;
    let mut final_text = None;
    if allow_ai && ai_applies(&settings, &mode) && !prepared.base.trim().is_empty() {
        on_ai();
        let input = prepared.slots.to_ai_markers(&prepared.base);
        let action = AiAction::Mode { mode_id: mode.id.clone() };
        let req = prompts::build_rewrite(&input, &action, Some(&mode), &prompt_context(&settings, &decision));
        match ai_request(state, req).await {
            Ok(out) => {
                let clean = prompts::sanitize_output(&out, mode.local.structure == wispr_core::text::modes::Structure::Code);
                let restored = if prepared.slots.is_empty() { Some(clean.clone()) } else { prepared.slots.from_ai_markers(&clean) };
                match restored {
                    Some(t) if prompts::plausible_rewrite(&input, &t, &action) => {
                        final_text = Some(pipeline::finalize(&t, true, &prepared, &ctx));
                        ai_used = true;
                    }
                    _ => {
                        ai_error = Some(LlmError::BadResponse("la respuesta no parecía una edición del dictado".into()));
                    }
                }
            }
            Err(e) => {
                tracing::warn!("IA no disponible: {e}");
                ai_error = Some(e);
            }
        }
    }
    let text = final_text.unwrap_or_else(|| pipeline::finalize(&prepared.local, false, &prepared, &ctx));
    TextResult {
        outcome: if text.trim().is_empty() {
            Outcome::Empty
        } else {
            Outcome::Text { text, press_enter: prepared.press_enter }
        },
        mode_id: mode.id.clone(),
        ai_used,
        ai_error,
    }
}

/// Reescribe un texto existente con una acción de IA (historial, «reformular selección»).
/// Sin IA disponible, aplica la limpieza local del modo.
pub async fn rewrite(state: &Arc<AppState>, text: &str, action: AiAction, target: &Foreground) -> Result<(String, bool), String> {
    let settings = state.settings();
    let decision = decide(&settings, target);
    let mode = match &action {
        AiAction::Mode { mode_id } => settings.writing.mode(mode_id).cloned().unwrap_or_else(|| settings.writing.active()),
        _ => mode_for(&settings, &decision, None),
    };
    if settings.ai.usable() {
        let req = prompts::build_rewrite(text, &action, Some(&mode), &prompt_context(&settings, &decision));
        match ai_request(state, req).await {
            Ok(out) => {
                let clean = prompts::sanitize_output(&out, false);
                if prompts::plausible_rewrite(text, &clean, &action) {
                    let fixed = pipeline::apply_vocabulary(&clean, &settings);
                    return Ok((wispr_core::text::format::apply_final_period(&fixed, mode.local.final_period), true));
                }
                return Err("La respuesta de la IA no parecía una edición del texto.".into());
            }
            Err(e) => {
                if !matches!(action, AiAction::Mode { .. }) {
                    return Err(e.to_string());
                }
                tracing::warn!("IA no disponible, se usa limpieza local: {e}");
            }
        }
    } else if !matches!(action, AiAction::Mode { .. }) {
        return Err("Esta acción necesita un proveedor de IA configurado (Ajustes › IA).".into());
    }
    let lang = text_lang(&settings, None);
    let ctx = LocalContext { settings: &settings, mode: &mode, lang: &lang, audio_had_speech: true };
    let cleaned = pipeline::clean(text, &ctx);
    Ok((wispr_core::text::format::apply_mode_format(&cleaned, &mode.local), false))
}

/// Modo comando: instrucción dictada aplicada a la selección.
pub async fn command(state: &Arc<AppState>, instruction: &str, selection: Option<&str>, target: &Foreground) -> Result<String, LlmError> {
    let settings = state.settings();
    let decision = decide(&settings, target);
    let req = prompts::build_command(instruction, selection, &prompt_context(&settings, &decision));
    let out = ai_request(state, req).await?;
    let clean = prompts::sanitize_output(&out, decision.category == appctx::AppCategory::Code);
    if clean.trim().is_empty() {
        return Err(LlmError::BadResponse("respuesta vacía".into()));
    }
    Ok(pipeline::apply_vocabulary(&clean, &settings))
}
