//! Contexto de la aplicación activa.
//!
//! Solo se usa el **nombre del ejecutable** de la ventana en primer plano
//! (p. ej. `outlook.exe`). El título de la ventana únicamente se consulta si el
//! usuario lo activa, se procesa en memoria y nunca se guarda ni se envía.

use crate::settings::{InsertMethod, Settings};
use crate::text::normalize::strip_accents;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AppCategory {
    Code,
    Terminal,
    Email,
    Chat,
    Browser,
    Documents,
    Notes,
    Other,
}

impl AppCategory {
    pub fn all() -> [AppCategory; 8] {
        [
            AppCategory::Code,
            AppCategory::Terminal,
            AppCategory::Email,
            AppCategory::Chat,
            AppCategory::Browser,
            AppCategory::Documents,
            AppCategory::Notes,
            AppCategory::Other,
        ]
    }

    /// Descripción breve que se puede enviar a la IA (sin datos personales).
    pub fn ai_hint(self) -> &'static str {
        match self {
            AppCategory::Code => "un editor de código o IDE",
            AppCategory::Terminal => "una terminal de comandos",
            AppCategory::Email => "un cliente de correo electrónico",
            AppCategory::Chat => "una aplicación de mensajería o chat",
            AppCategory::Browser => "un navegador web",
            AppCategory::Documents => "un procesador de textos o documento",
            AppCategory::Notes => "una aplicación de notas",
            AppCategory::Other => "una aplicación de escritorio",
        }
    }
}

/// Ejecutables conocidos → categoría.
const KNOWN_APPS: &[(&str, AppCategory)] = &[
    ("code.exe", AppCategory::Code),
    ("code - insiders.exe", AppCategory::Code),
    ("cursor.exe", AppCategory::Code),
    ("windsurf.exe", AppCategory::Code),
    ("zed.exe", AppCategory::Code),
    ("devenv.exe", AppCategory::Code),
    ("idea64.exe", AppCategory::Code),
    ("pycharm64.exe", AppCategory::Code),
    ("webstorm64.exe", AppCategory::Code),
    ("rider64.exe", AppCategory::Code),
    ("clion64.exe", AppCategory::Code),
    ("goland64.exe", AppCategory::Code),
    ("rustrover64.exe", AppCategory::Code),
    ("phpstorm64.exe", AppCategory::Code),
    ("datagrip64.exe", AppCategory::Code),
    ("studio64.exe", AppCategory::Code),
    ("sublime_text.exe", AppCategory::Code),
    ("notepad++.exe", AppCategory::Code),
    ("eclipse.exe", AppCategory::Code),
    ("fleet.exe", AppCategory::Code),
    ("windowsterminal.exe", AppCategory::Terminal),
    ("wt.exe", AppCategory::Terminal),
    ("cmd.exe", AppCategory::Terminal),
    ("powershell.exe", AppCategory::Terminal),
    ("pwsh.exe", AppCategory::Terminal),
    ("conhost.exe", AppCategory::Terminal),
    ("wezterm-gui.exe", AppCategory::Terminal),
    ("alacritty.exe", AppCategory::Terminal),
    ("mintty.exe", AppCategory::Terminal),
    ("putty.exe", AppCategory::Terminal),
    ("tabby.exe", AppCategory::Terminal),
    ("warp.exe", AppCategory::Terminal),
    ("outlook.exe", AppCategory::Email),
    ("olk.exe", AppCategory::Email),
    ("hxoutlook.exe", AppCategory::Email),
    ("thunderbird.exe", AppCategory::Email),
    ("mailbird.exe", AppCategory::Email),
    ("emclient.exe", AppCategory::Email),
    ("spark.exe", AppCategory::Email),
    ("slack.exe", AppCategory::Chat),
    ("discord.exe", AppCategory::Chat),
    ("teams.exe", AppCategory::Chat),
    ("ms-teams.exe", AppCategory::Chat),
    ("whatsapp.exe", AppCategory::Chat),
    ("whatsapp.root.exe", AppCategory::Chat),
    ("telegram.exe", AppCategory::Chat),
    ("signal.exe", AppCategory::Chat),
    ("messenger.exe", AppCategory::Chat),
    ("element.exe", AppCategory::Chat),
    ("zoom.exe", AppCategory::Chat),
    ("skype.exe", AppCategory::Chat),
    ("chrome.exe", AppCategory::Browser),
    ("msedge.exe", AppCategory::Browser),
    ("firefox.exe", AppCategory::Browser),
    ("brave.exe", AppCategory::Browser),
    ("opera.exe", AppCategory::Browser),
    ("opera_gx.exe", AppCategory::Browser),
    ("vivaldi.exe", AppCategory::Browser),
    ("arc.exe", AppCategory::Browser),
    ("zen.exe", AppCategory::Browser),
    ("librewolf.exe", AppCategory::Browser),
    ("winword.exe", AppCategory::Documents),
    ("soffice.bin", AppCategory::Documents),
    ("swriter.exe", AppCategory::Documents),
    ("wordpad.exe", AppCategory::Documents),
    ("powerpnt.exe", AppCategory::Documents),
    ("excel.exe", AppCategory::Documents),
    ("acrobat.exe", AppCategory::Documents),
    ("notepad.exe", AppCategory::Notes),
    ("obsidian.exe", AppCategory::Notes),
    ("notion.exe", AppCategory::Notes),
    ("onenote.exe", AppCategory::Notes),
    ("onenoteim.exe", AppCategory::Notes),
    ("evernote.exe", AppCategory::Notes),
    ("joplin.exe", AppCategory::Notes),
    ("logseq.exe", AppCategory::Notes),
    ("typora.exe", AppCategory::Notes),
    ("stickynot.exe", AppCategory::Notes),
];

/// Sitios reconocibles por el título de la pestaña del navegador.
const SITE_HINTS: &[(&str, AppCategory)] = &[
    ("gmail", AppCategory::Email),
    ("outlook", AppCategory::Email),
    ("correo", AppCategory::Email),
    ("proton mail", AppCategory::Email),
    ("yahoo mail", AppCategory::Email),
    ("slack", AppCategory::Chat),
    ("whatsapp", AppCategory::Chat),
    ("discord", AppCategory::Chat),
    ("telegram", AppCategory::Chat),
    ("messenger", AppCategory::Chat),
    ("microsoft teams", AppCategory::Chat),
    ("google chat", AppCategory::Chat),
    ("google docs", AppCategory::Documents),
    ("documentos de google", AppCategory::Documents),
    ("word", AppCategory::Documents),
    ("notion", AppCategory::Notes),
    ("keep", AppCategory::Notes),
    ("codespaces", AppCategory::Code),
    ("vscode.dev", AppCategory::Code),
    ("replit", AppCategory::Code),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppProfile {
    pub id: String,
    pub name: String,
    /// Ejecutables (sin ruta), p. ej. `slack.exe`.
    pub exe: Vec<String>,
    /// Texto que debe contener el título (solo si se permite leer títulos).
    pub title_contains: Vec<String>,
    pub mode: Option<String>,
    pub insert_method: Option<InsertMethod>,
    /// Indicaciones extra para la IA en esta aplicación.
    pub ai_context: String,
    pub enabled: bool,
}

impl Default for AppProfile {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            exe: Vec::new(),
            title_contains: Vec::new(),
            mode: None,
            insert_method: None,
            ai_context: String::new(),
            enabled: true,
        }
    }
}

/// Información mínima de la ventana activa.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ForegroundApp {
    /// Nombre del ejecutable en minúsculas (`outlook.exe`).
    pub exe: String,
    /// Título de la ventana; solo presente si el usuario lo ha permitido.
    pub title: Option<String>,
    /// La ventana pertenece a un proceso con privilegios elevados.
    pub elevated: bool,
}

/// Decisión de contexto para un dictado.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppDecision {
    pub category: AppCategory,
    pub mode_id: Option<String>,
    pub insert_method: Option<InsertMethod>,
    pub ai_context: Option<String>,
    pub profile: Option<String>,
}

pub fn classify(exe: &str, title: Option<&str>) -> AppCategory {
    let exe = exe.to_lowercase();
    let base = KNOWN_APPS.iter().find(|(e, _)| *e == exe).map(|(_, c)| *c).unwrap_or(AppCategory::Other);
    if base == AppCategory::Browser {
        if let Some(t) = title {
            let t = strip_accents(&t.to_lowercase());
            if let Some((_, c)) = SITE_HINTS.iter().find(|(k, _)| t.contains(k)) {
                return *c;
            }
        }
    }
    base
}

/// Decide modo, método de inserción y contexto de IA para la aplicación activa.
pub fn resolve(settings: &Settings, app: &ForegroundApp) -> AppDecision {
    let ctx = &settings.apps;
    let title = if ctx.use_window_titles { app.title.as_deref() } else { None };
    let category = classify(&app.exe, title);
    let mut d = AppDecision { category, mode_id: None, insert_method: None, ai_context: None, profile: None };
    if !ctx.enabled {
        return d;
    }
    let exe = app.exe.to_lowercase();
    let title_l = title.map(|t| t.to_lowercase());
    let profile = ctx.profiles.iter().filter(|p| p.enabled).find(|p| {
        let exe_ok = p.exe.is_empty() || p.exe.iter().any(|e| e.trim().to_lowercase() == exe);
        let title_ok = p.title_contains.is_empty()
            || title_l.as_ref().is_some_and(|t| p.title_contains.iter().any(|k| t.contains(&k.to_lowercase())));
        exe_ok && title_ok && !(p.exe.is_empty() && p.title_contains.is_empty())
    });
    if let Some(p) = profile {
        d.mode_id = p.mode.clone();
        d.insert_method = p.insert_method;
        d.ai_context = Some(p.ai_context.clone()).filter(|s| !s.trim().is_empty());
        d.profile = Some(p.name.clone());
    }
    if d.mode_id.is_none() {
        d.mode_id = ctx.category_modes.get(&category).cloned().filter(|m| settings.writing.mode(m).is_some());
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fg(exe: &str, title: Option<&str>) -> ForegroundApp {
        ForegroundApp { exe: exe.into(), title: title.map(String::from), elevated: false }
    }

    #[test]
    fn classifies_known_apps() {
        assert_eq!(classify("Code.exe", None), AppCategory::Code);
        assert_eq!(classify("OUTLOOK.EXE", None), AppCategory::Email);
        assert_eq!(classify("unknown.exe", None), AppCategory::Other);
        assert_eq!(classify("chrome.exe", Some("Recibidos - Gmail")), AppCategory::Email);
        assert_eq!(classify("chrome.exe", None), AppCategory::Browser);
    }

    #[test]
    fn category_defaults_to_modes() {
        let s = Settings::default();
        assert_eq!(resolve(&s, &fg("slack.exe", None)).mode_id.as_deref(), Some("messages"));
        assert_eq!(resolve(&s, &fg("code.exe", None)).mode_id.as_deref(), Some("code"));
        assert_eq!(resolve(&s, &fg("chrome.exe", None)).mode_id, None);
    }

    #[test]
    fn titles_ignored_unless_enabled() {
        let mut s = Settings::default();
        assert_eq!(resolve(&s, &fg("chrome.exe", Some("Gmail"))).category, AppCategory::Browser);
        s.apps.use_window_titles = true;
        assert_eq!(resolve(&s, &fg("chrome.exe", Some("Gmail"))).mode_id.as_deref(), Some("email"));
    }

    #[test]
    fn user_profiles_win() {
        let mut s = Settings::default();
        s.apps.profiles.push(AppProfile {
            name: "Slack trabajo".into(),
            exe: vec!["slack.exe".into()],
            mode: Some("professional".into()),
            insert_method: Some(InsertMethod::Type),
            ai_context: "Canal del equipo".into(),
            ..Default::default()
        });
        let d = resolve(&s, &fg("slack.exe", None));
        assert_eq!(d.mode_id.as_deref(), Some("professional"));
        assert_eq!(d.insert_method, Some(InsertMethod::Type));
        assert_eq!(d.profile.as_deref(), Some("Slack trabajo"));
        s.apps.enabled = false;
        assert_eq!(resolve(&s, &fg("slack.exe", None)).mode_id, None);
    }
}
