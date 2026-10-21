//! Historial local de dictados y estadísticas (SQLite, sin conexión).
//!
//! Las estadísticas diarias se guardan aparte y solo contienen recuentos, de modo
//! que siguen disponibles aunque el usuario desactive o borre el historial.

use crate::text::normalize::search_key;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HistoryEntry {
    pub id: i64,
    /// Fecha ISO 8601 local.
    pub created_at: String,
    pub raw_text: String,
    pub final_text: String,
    pub mode: String,
    pub app: Option<String>,
    pub language: Option<String>,
    pub engine: String,
    pub model: String,
    pub duration_ms: i64,
    pub processing_ms: i64,
    pub words: i64,
    pub favorite: bool,
    pub edited: bool,
    pub ai_used: bool,
    pub audio_path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HistoryQuery {
    pub search: String,
    pub favorites_only: bool,
    /// `true` = más antiguos primero.
    pub oldest_first: bool,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct DayStat {
    pub date: String,
    pub words: u64,
    pub dictations: u64,
    pub audio_ms: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Stats {
    pub total_words: u64,
    pub total_dictations: u64,
    pub total_audio_ms: u64,
    pub today_words: u64,
    pub week_words: u64,
    /// Minutos ahorrados frente a escribir a `typing_wpm`.
    pub minutes_saved: f64,
    /// Palabras por minuto al hablar.
    pub speaking_wpm: f64,
    /// Días consecutivos con al menos un dictado.
    pub streak_days: u32,
    pub last_30_days: Vec<DayStat>,
    pub top_apps: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Txt,
    Markdown,
    Csv,
    Json,
}

pub struct History {
    conn: Connection,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL,
    raw_text TEXT NOT NULL DEFAULT '',
    final_text TEXT NOT NULL,
    search TEXT NOT NULL DEFAULT '',
    mode TEXT NOT NULL DEFAULT '',
    app TEXT,
    language TEXT,
    engine TEXT NOT NULL DEFAULT '',
    model TEXT NOT NULL DEFAULT '',
    duration_ms INTEGER NOT NULL DEFAULT 0,
    processing_ms INTEGER NOT NULL DEFAULT 0,
    words INTEGER NOT NULL DEFAULT 0,
    favorite INTEGER NOT NULL DEFAULT 0,
    edited INTEGER NOT NULL DEFAULT 0,
    ai_used INTEGER NOT NULL DEFAULT 0,
    audio_path TEXT
);
CREATE INDEX IF NOT EXISTS idx_entries_created ON entries(created_at);
CREATE TABLE IF NOT EXISTS daily_stats (
    date TEXT PRIMARY KEY,
    words INTEGER NOT NULL DEFAULT 0,
    dictations INTEGER NOT NULL DEFAULT 0,
    audio_ms INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS app_stats (
    app TEXT PRIMARY KEY,
    dictations INTEGER NOT NULL DEFAULT 0
);
PRAGMA user_version = 1;
"#;

fn row_to_entry(r: &rusqlite::Row) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: r.get("id")?,
        created_at: r.get("created_at")?,
        raw_text: r.get("raw_text")?,
        final_text: r.get("final_text")?,
        mode: r.get("mode")?,
        app: r.get("app")?,
        language: r.get("language")?,
        engine: r.get("engine")?,
        model: r.get("model")?,
        duration_ms: r.get("duration_ms")?,
        processing_ms: r.get("processing_ms")?,
        words: r.get("words")?,
        favorite: r.get::<_, i64>("favorite")? != 0,
        edited: r.get::<_, i64>("edited")? != 0,
        ai_used: r.get::<_, i64>("ai_used")? != 0,
        audio_path: r.get("audio_path")?,
    })
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

impl History {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(p) = path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        // Memoria acotada: caché de 512 KiB.
        conn.pragma_update(None, "cache_size", -512)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Guarda un dictado. Devuelve su identificador.
    pub fn insert(&self, e: &HistoryEntry) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO entries (created_at, raw_text, final_text, search, mode, app, language, engine, model,
                duration_ms, processing_ms, words, favorite, edited, ai_used, audio_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0, ?14, ?15)",
            params![
                e.created_at,
                e.raw_text,
                e.final_text,
                search_key(&format!("{} {}", e.final_text, e.raw_text)),
                e.mode,
                e.app,
                e.language,
                e.engine,
                e.model,
                e.duration_ms,
                e.processing_ms,
                e.words,
                e.favorite as i64,
                e.ai_used as i64,
                e.audio_path,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Suma un dictado a las estadísticas (siempre, aunque no se guarde el texto).
    pub fn record_stats(&self, date: &str, words: u64, audio_ms: u64, app: Option<&str>) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO daily_stats (date, words, dictations, audio_ms) VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(date) DO UPDATE SET words = words + ?2, dictations = dictations + 1, audio_ms = audio_ms + ?3",
            params![date, words as i64, audio_ms as i64],
        )?;
        if let Some(app) = app.filter(|a| !a.is_empty()) {
            self.conn.execute(
                "INSERT INTO app_stats (app, dictations) VALUES (?1, 1)
                 ON CONFLICT(app) DO UPDATE SET dictations = dictations + 1",
                params![app],
            )?;
        }
        Ok(())
    }

    pub fn get(&self, id: i64) -> rusqlite::Result<Option<HistoryEntry>> {
        self.conn.query_row("SELECT * FROM entries WHERE id = ?1", params![id], row_to_entry).optional()
    }

    pub fn query(&self, q: &HistoryQuery) -> rusqlite::Result<HistoryPage> {
        let mut where_sql = String::from("WHERE 1=1");
        let mut args: Vec<String> = Vec::new();
        for word in search_key(&q.search).split_whitespace() {
            args.push(format!("%{}%", escape_like(word)));
            where_sql.push_str(&format!(" AND search LIKE ?{} ESCAPE '\\'", args.len()));
        }
        if q.favorites_only {
            where_sql.push_str(" AND favorite = 1");
        }
        let order = if q.oldest_first { "ASC" } else { "DESC" };
        let limit = if q.limit == 0 { 50 } else { q.limit.min(500) };
        let total: u64 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM entries {where_sql}"),
            rusqlite::params_from_iter(args.iter()),
            |r| r.get::<_, i64>(0),
        )? as u64;
        let mut stmt = self.conn.prepare(&format!(
            "SELECT * FROM entries {where_sql} ORDER BY created_at {order}, id {order} LIMIT {limit} OFFSET {}",
            q.offset
        ))?;
        let entries = stmt
            .query_map(rusqlite::params_from_iter(args.iter()), row_to_entry)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(HistoryPage { entries, total })
    }

    pub fn recent(&self, n: u32) -> rusqlite::Result<Vec<HistoryEntry>> {
        Ok(self.query(&HistoryQuery { limit: n, ..Default::default() })?.entries)
    }

    pub fn update_text(&self, id: i64, text: &str) -> rusqlite::Result<()> {
        let raw: String = self
            .conn
            .query_row("SELECT raw_text FROM entries WHERE id = ?1", params![id], |r| r.get(0))
            .unwrap_or_default();
        self.conn.execute(
            "UPDATE entries SET final_text = ?2, edited = 1, search = ?3, words = ?4 WHERE id = ?1",
            params![
                id,
                text,
                search_key(&format!("{text} {raw}")),
                crate::text::normalize::word_count(text) as i64
            ],
        )?;
        Ok(())
    }

    /// Sustituye el texto final tras reprocesarlo con otro modo.
    pub fn update_processed(&self, id: i64, text: &str, mode: &str, ai_used: bool) -> rusqlite::Result<()> {
        self.update_text(id, text)?;
        self.conn.execute(
            "UPDATE entries SET mode = ?2, ai_used = ?3, edited = 0 WHERE id = ?1",
            params![id, mode, ai_used as i64],
        )?;
        Ok(())
    }

    pub fn set_favorite(&self, id: i64, favorite: bool) -> rusqlite::Result<()> {
        self.conn.execute("UPDATE entries SET favorite = ?2 WHERE id = ?1", params![id, favorite as i64])?;
        Ok(())
    }

    /// Elimina una entrada y devuelve la ruta de su audio (para borrarlo).
    pub fn delete(&self, id: i64) -> rusqlite::Result<Option<String>> {
        let audio: Option<String> = self
            .conn
            .query_row("SELECT audio_path FROM entries WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?
            .flatten();
        self.conn.execute("DELETE FROM entries WHERE id = ?1", params![id])?;
        Ok(audio)
    }

    /// Borra todo el historial (los favoritos también si `keep_favorites` es falso).
    /// Devuelve las rutas de audio asociadas.
    pub fn clear(&self, keep_favorites: bool) -> rusqlite::Result<Vec<String>> {
        let cond = if keep_favorites { "WHERE favorite = 0" } else { "" };
        let mut stmt = self.conn.prepare(&format!("SELECT audio_path FROM entries {cond}"))?;
        let audio: Vec<String> =
            stmt.query_map([], |r| r.get::<_, Option<String>>(0))?.filter_map(|r| r.ok().flatten()).collect();
        self.conn.execute(&format!("DELETE FROM entries {cond}"), [])?;
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")?;
        Ok(audio)
    }

    /// Aplica la retención: borra entradas no favoritas anteriores a `days` días.
    pub fn purge_older_than(&self, days: u32) -> rusqlite::Result<Vec<String>> {
        if days == 0 {
            return Ok(Vec::new());
        }
        let cutoff = (chrono::Local::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        let mut stmt = self.conn.prepare("SELECT audio_path FROM entries WHERE created_at < ?1 AND favorite = 0")?;
        let audio: Vec<String> =
            stmt.query_map(params![cutoff], |r| r.get::<_, Option<String>>(0))?.filter_map(|r| r.ok().flatten()).collect();
        self.conn.execute("DELETE FROM entries WHERE created_at < ?1 AND favorite = 0", params![cutoff])?;
        Ok(audio)
    }

    /// Quita las referencias a audios anteriores a `days` días y devuelve sus rutas.
    pub fn purge_audio_older_than(&self, days: u32) -> rusqlite::Result<Vec<String>> {
        let cutoff = (chrono::Local::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        let mut stmt = self.conn.prepare("SELECT audio_path FROM entries WHERE created_at < ?1 AND audio_path IS NOT NULL")?;
        let audio: Vec<String> =
            stmt.query_map(params![cutoff], |r| r.get::<_, Option<String>>(0))?.filter_map(|r| r.ok().flatten()).collect();
        self.conn.execute("UPDATE entries SET audio_path = NULL WHERE created_at < ?1", params![cutoff])?;
        Ok(audio)
    }

    pub fn clear_stats(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch("DELETE FROM daily_stats; DELETE FROM app_stats;")
    }

    pub fn count(&self) -> rusqlite::Result<u64> {
        self.conn.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get::<_, i64>(0)).map(|n| n as u64)
    }

    pub fn stats(&self, typing_wpm: u32) -> rusqlite::Result<Stats> {
        let mut s = Stats::default();
        let (w, d, a): (i64, i64, i64) = self.conn.query_row(
            "SELECT COALESCE(SUM(words),0), COALESCE(SUM(dictations),0), COALESCE(SUM(audio_ms),0) FROM daily_stats",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        s.total_words = w as u64;
        s.total_dictations = d as u64;
        s.total_audio_ms = a as u64;
        let today = chrono::Local::now().date_naive();
        let mut stmt = self.conn.prepare("SELECT date, words, dictations, audio_ms FROM daily_stats WHERE date >= ?1 ORDER BY date")?;
        let since = (today - chrono::Duration::days(29)).format("%Y-%m-%d").to_string();
        let days: Vec<DayStat> = stmt
            .query_map(params![since], |r| {
                Ok(DayStat {
                    date: r.get(0)?,
                    words: r.get::<_, i64>(1)? as u64,
                    dictations: r.get::<_, i64>(2)? as u64,
                    audio_ms: r.get::<_, i64>(3)? as u64,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let today_s = today.format("%Y-%m-%d").to_string();
        let week_s = (today - chrono::Duration::days(6)).format("%Y-%m-%d").to_string();
        s.today_words = days.iter().filter(|d| d.date == today_s).map(|d| d.words).sum();
        s.week_words = days.iter().filter(|d| d.date >= week_s).map(|d| d.words).sum();
        // Racha: días consecutivos hasta hoy (o ayer si hoy aún no se ha dictado).
        let mut all_dates: Vec<String> = self
            .conn
            .prepare("SELECT date FROM daily_stats WHERE dictations > 0 ORDER BY date DESC LIMIT 400")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<_>>()?;
        all_dates.dedup();
        let mut cursor = today;
        if all_dates.first().map(String::as_str) != Some(today_s.as_str()) {
            cursor = today - chrono::Duration::days(1);
        }
        for d in &all_dates {
            if *d == cursor.format("%Y-%m-%d").to_string() {
                s.streak_days += 1;
                cursor -= chrono::Duration::days(1);
            } else if d.as_str() < cursor.format("%Y-%m-%d").to_string().as_str() {
                break;
            }
        }
        // Completa los últimos 30 días (con ceros) para el gráfico.
        s.last_30_days = (0..30)
            .rev()
            .map(|i| {
                let date = (today - chrono::Duration::days(i)).format("%Y-%m-%d").to_string();
                days.iter().find(|d| d.date == date).cloned().unwrap_or(DayStat { date, ..Default::default() })
            })
            .collect();
        let speaking_minutes = s.total_audio_ms as f64 / 60_000.0;
        s.speaking_wpm = if speaking_minutes > 0.05 { s.total_words as f64 / speaking_minutes } else { 0.0 };
        let typing_minutes = s.total_words as f64 / typing_wpm.max(1) as f64;
        s.minutes_saved = (typing_minutes - speaking_minutes).max(0.0);
        s.top_apps = self
            .conn
            .prepare("SELECT app, dictations FROM app_stats ORDER BY dictations DESC LIMIT 5")?
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64)))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(s)
    }

    /// Exporta las entradas seleccionadas (o todas si `ids` está vacío).
    pub fn export(&self, ids: &[i64], format: ExportFormat) -> rusqlite::Result<String> {
        let entries: Vec<HistoryEntry> = if ids.is_empty() {
            self.query(&HistoryQuery { limit: 500, oldest_first: true, ..Default::default() })?.entries
        } else {
            ids.iter().filter_map(|id| self.get(*id).ok().flatten()).collect()
        };
        Ok(match format {
            ExportFormat::Txt => entries
                .iter()
                .map(|e| format!("[{}]\n{}\n", e.created_at[..16.min(e.created_at.len())].replace('T', " "), e.final_text))
                .collect::<Vec<_>>()
                .join("\n"),
            ExportFormat::Markdown => {
                let mut out = String::from("# Dictados de Wispr Swag\n\n");
                for e in &entries {
                    out.push_str(&format!(
                        "## {}\n\n{}\n\n",
                        e.created_at[..16.min(e.created_at.len())].replace('T', " "),
                        e.final_text
                    ));
                }
                out
            }
            ExportFormat::Csv => {
                let q = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
                let mut out = String::from("fecha,texto,modo,aplicacion,idioma,palabras,favorito\n");
                for e in &entries {
                    out.push_str(&format!(
                        "{},{},{},{},{},{},{}\n",
                        q(&e.created_at),
                        q(&e.final_text),
                        q(&e.mode),
                        q(e.app.as_deref().unwrap_or("")),
                        q(e.language.as_deref().unwrap_or("")),
                        e.words,
                        e.favorite
                    ));
                }
                out
            }
            ExportFormat::Json => serde_json::to_string_pretty(&entries).unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(text: &str) -> HistoryEntry {
        HistoryEntry {
            created_at: chrono::Local::now().to_rfc3339(),
            raw_text: text.to_lowercase(),
            final_text: text.into(),
            mode: "natural".into(),
            engine: "local".into(),
            words: crate::text::normalize::word_count(text) as i64,
            duration_ms: 3000,
            ..Default::default()
        }
    }

    #[test]
    fn insert_search_edit_delete() {
        let h = History::open_in_memory().unwrap();
        let a = h.insert(&entry("La canción de mañana")).unwrap();
        let _b = h.insert(&entry("Informe trimestral 50%")).unwrap();
        assert_eq!(h.count().unwrap(), 2);
        // Búsqueda sin tildes ni mayúsculas.
        let r = h.query(&HistoryQuery { search: "CANCION manana".into(), ..Default::default() }).unwrap();
        assert_eq!(r.total, 1);
        assert_eq!(r.entries[0].id, a);
        // El % se busca literalmente.
        assert_eq!(h.query(&HistoryQuery { search: "50%".into(), ..Default::default() }).unwrap().total, 1);
        assert_eq!(h.query(&HistoryQuery { search: "%".into(), ..Default::default() }).unwrap().total, 1);
        h.update_text(a, "Texto corregido").unwrap();
        let e = h.get(a).unwrap().unwrap();
        assert!(e.edited);
        assert_eq!(e.words, 2);
        h.set_favorite(a, true).unwrap();
        assert_eq!(h.query(&HistoryQuery { favorites_only: true, ..Default::default() }).unwrap().total, 1);
        h.delete(a).unwrap();
        assert_eq!(h.count().unwrap(), 1);
    }

    #[test]
    fn ordering_and_clear() {
        let h = History::open_in_memory().unwrap();
        for i in 0..5 {
            let mut e = entry(&format!("Entrada {i}"));
            e.created_at = format!("2026-01-0{}T10:00:00+01:00", i + 1);
            h.insert(&e).unwrap();
        }
        let newest = h.query(&HistoryQuery::default()).unwrap();
        assert_eq!(newest.entries[0].final_text, "Entrada 4");
        let oldest = h.query(&HistoryQuery { oldest_first: true, limit: 2, ..Default::default() }).unwrap();
        assert_eq!(oldest.entries.len(), 2);
        assert_eq!(oldest.entries[0].final_text, "Entrada 0");
        h.set_favorite(oldest.entries[0].id, true).unwrap();
        h.purge_older_than(30).unwrap();
        assert_eq!(h.count().unwrap(), 1, "solo queda el favorito");
        h.clear(false).unwrap();
        assert_eq!(h.count().unwrap(), 0);
    }

    #[test]
    fn stats_and_streak() {
        let h = History::open_in_memory().unwrap();
        let today = chrono::Local::now().date_naive();
        for back in 0..3 {
            let d = (today - chrono::Duration::days(back)).format("%Y-%m-%d").to_string();
            h.record_stats(&d, 100, 30_000, Some("slack.exe")).unwrap();
        }
        let s = h.stats(40).unwrap();
        assert_eq!(s.total_words, 300);
        assert_eq!(s.streak_days, 3);
        assert_eq!(s.today_words, 100);
        assert_eq!(s.last_30_days.len(), 30);
        assert!((s.speaking_wpm - 200.0).abs() < 0.1);
        // 300 palabras a 40 ppm = 7,5 min; habladas en 1,5 min → 6 min ahorrados.
        assert!((s.minutes_saved - 6.0).abs() < 0.01);
        assert_eq!(s.top_apps[0], ("slack.exe".to_string(), 3));
    }

    #[test]
    fn export_formats() {
        let h = History::open_in_memory().unwrap();
        h.insert(&entry("Hola, \"mundo\"")).unwrap();
        assert!(h.export(&[], ExportFormat::Csv).unwrap().contains("\"Hola, \"\"mundo\"\"\""));
        assert!(h.export(&[], ExportFormat::Markdown).unwrap().starts_with("# Dictados"));
        let json = h.export(&[], ExportFormat::Json).unwrap();
        let back: Vec<HistoryEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 1);
    }

    #[test]
    fn persists_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("h.sqlite3");
        {
            let h = History::open(&p).unwrap();
            h.insert(&entry("persistente")).unwrap();
        }
        let h = History::open(&p).unwrap();
        assert_eq!(h.count().unwrap(), 1);
    }
}
