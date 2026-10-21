//! Textos breves de la capa nativa (bandeja, barra flotante y avisos) en español e inglés.
//! La interfaz de ajustes tiene su propio diccionario.

use crate::settings::UiLanguage;

/// Resuelve el idioma efectivo (`es`/`en`) a partir del ajuste y del idioma del sistema.
pub fn resolve(setting: UiLanguage, system_locale: Option<&str>) -> &'static str {
    match setting {
        UiLanguage::Es => "es",
        UiLanguage::En => "en",
        UiLanguage::Auto => match system_locale {
            Some(l) if l.to_lowercase().starts_with("en") => "en",
            _ => "es",
        },
    }
}

/// Traduce una clave. Las claves desconocidas se devuelven tal cual.
pub fn t(lang: &str, key: &'static str) -> &'static str {
    let en = lang.starts_with("en");
    let pair = match key {
        "tray.tooltip" => ("Wispr Swag — dictado por voz", "Wispr Swag — voice dictation"),
        "tray.dictate" => ("Iniciar dictado", "Start dictation"),
        "tray.stop" => ("Detener dictado", "Stop dictation"),
        "tray.paste_last" => ("Pegar última transcripción", "Paste last transcript"),
        "tray.recent" => ("Dictados recientes", "Recent dictations"),
        "tray.no_recent" => ("Sin dictados todavía", "No dictations yet"),
        "tray.mode" => ("Modo de escritura", "Writing mode"),
        "tray.language" => ("Idioma de dictado", "Dictation language"),
        "tray.open" => ("Abrir Wispr Swag", "Open Wispr Swag"),
        "tray.history" => ("Historial", "History"),
        "tray.settings" => ("Ajustes", "Settings"),
        "tray.pause" => ("Pausar atajos", "Pause shortcuts"),
        "tray.toggle_bar" => ("Mostrar barra en reposo", "Show idle bar"),
        "tray.quit" => ("Salir", "Quit"),
        "pill.listening" => ("Escuchando", "Listening"),
        "pill.processing" => ("Transcribiendo…", "Transcribing…"),
        "pill.polishing" => ("Puliendo con IA…", "Polishing with AI…"),
        "pill.copied" => ("Copiado: pulsa Ctrl+V", "Copied: press Ctrl+V"),
        "pill.cancelled" => ("Cancelado", "Cancelled"),
        "pill.no_speech" => ("No se detectó voz", "No speech detected"),
        "pill.command" => ("Comando", "Command"),
        "pill.paused" => ("Atajos en pausa", "Shortcuts paused"),
        "pill.resumed" => ("Atajos activos", "Shortcuts active"),
        "pill.loading_model" => ("Cargando modelo…", "Loading model…"),
        "pill.retry" => ("Reintentar", "Retry"),
        "pill.hint" => ("Pulsa para dictar", "Click to dictate"),
        "pill.elevated" => ("Ventana de administrador: copiado", "Admin window: copied"),
        "pill.deleted" => ("Borrado", "Deleted"),
        "pill.ai_fallback" => ("IA no disponible: texto local", "AI unavailable: local text"),
        "pill.too_short" => ("Mantén pulsado para dictar", "Hold to dictate"),
        "pill.mic_fallback" => ("Usando el micrófono predeterminado", "Using default microphone"),
        _ => (key, key),
    };
    if en {
        pair.1
    } else {
        pair.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_and_translates() {
        assert_eq!(resolve(UiLanguage::Auto, Some("en-US")), "en");
        assert_eq!(resolve(UiLanguage::Auto, Some("es-ES")), "es");
        assert_eq!(resolve(UiLanguage::Auto, None), "es");
        assert_eq!(t("es", "tray.quit"), "Salir");
        assert_eq!(t("en", "tray.quit"), "Quit");
        assert_eq!(t("es", "desconocida"), "desconocida");
    }
}
