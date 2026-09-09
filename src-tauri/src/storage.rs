use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub style: String,
    pub shortcut: String,
    pub activation: String,
    pub microphone: String,
    pub icon_position: String,
    pub history_enabled: bool,
    pub auto_insert: bool,
    pub launch_at_login: bool,
    pub vocabulary: Vec<String>,
    pub snippets: Vec<Snippet>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            language: "auto".into(),
            style: "clean".into(),
            shortcut: "Control+Shift+Space".into(),
            activation: "hold".into(),
            microphone: String::new(),
            icon_position: "bottom-center".into(),
            history_enabled: false,
            auto_insert: true,
            launch_at_login: false,
            vocabulary: vec![],
            snippets: vec![],
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Snippet {
    pub trigger: String,
    pub text: String,
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        let languages: serde_json::Value =
            serde_json::from_str(include_str!("../../src/languages.json"))
                .expect("Bundled language list must be valid JSON");
        if languages.get(&self.language).is_none() {
            return Err("Choose Auto-detect or a supported dictation language.".into());
        }
        if !["light", "dark", "system"].contains(&self.theme.as_str()) {
            return Err("Choose Light, Dark, or System appearance.".into());
        }
        if ![
            "top-left",
            "top-center",
            "top-right",
            "center-left",
            "center",
            "center-right",
            "bottom-left",
            "bottom-center",
            "bottom-right",
        ]
        .contains(&self.icon_position.as_str())
        {
            return Err("Choose a supported icon position.".into());
        }
        if !["clean", "verbatim"].contains(&self.style.as_str())
            || !["hold", "toggle"].contains(&self.activation.as_str())
        {
            return Err("Choose a writing style and recording mode from Settings.".into());
        }
        if self.vocabulary.len() > 100
            || self
                .vocabulary
                .iter()
                .any(|v| v.trim().is_empty() || v.len() > 100)
        {
            return Err(
                "Use up to 100 short dictionary entries. Shorten long words or phrases and try again.".into(),
            );
        }
        if self.snippets.len() > 100
            || self.snippets.iter().any(|s| {
                s.trigger.trim().is_empty()
                    || s.trigger.len() > 100
                    || s.text.is_empty()
                    || s.text.len() > 5000
            })
        {
            return Err("Use up to 100 snippets. Each needs a short phrase and saved text. Shorten long entries and try again.".into());
        }
        let mut seen = std::collections::HashSet::new();
        if self
            .snippets
            .iter()
            .any(|s| !seen.insert(normalize_trigger(&s.trigger)))
        {
            return Err("Each snippet needs a unique spoken phrase.".into());
        }
        Ok(())
    }
}
pub fn normalize_trigger(text: &str) -> String {
    text.trim()
        .trim_end_matches(['.', '!', '?', '।'])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
pub fn expand_snippet(text: &str, snippets: &[Snippet]) -> String {
    let normalized = normalize_trigger(text);
    snippets
        .iter()
        .find(|s| normalize_trigger(&s.trigger) == normalized)
        .map(|s| s.text.clone())
        .unwrap_or_else(|| text.to_owned())
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: i64,
    pub created_at: u64,
    pub text: String,
    pub language: String,
    pub seconds: f64,
    pub cost: Option<f64>,
    pub latency_ms: u64,
}
pub struct Storage {
    connection: Connection,
    pub directory: PathBuf,
}
impl Storage {
    pub fn open(directory: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&directory).map_err(|_| "Could not create Bol's data folder.")?;
        let connection = Connection::open(directory.join("bol.sqlite"))
            .map_err(|_| "Could not open Bol’s saved data. Try reopening the app.")?;
        connection.execute_batch("PRAGMA secure_delete=ON; CREATE TABLE IF NOT EXISTS preferences (id INTEGER PRIMARY KEY CHECK(id=1), value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS history (id INTEGER PRIMARY KEY, created_at INTEGER NOT NULL, text TEXT NOT NULL, language TEXT NOT NULL, seconds REAL NOT NULL, cost REAL, latency_ms INTEGER NOT NULL);").map_err(|_| "Could not prepare Bol’s saved data. Try reopening the app.")?;
        let store = Self {
            connection,
            directory,
        };
        Ok(store)
    }
    pub fn settings(&self) -> Settings {
        self.connection
            .query_row("SELECT value FROM preferences WHERE id=1", [], |r| {
                r.get::<_, String>(0)
            })
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        let json = serde_json::to_string(settings)
            .map_err(|_| "Could not prepare your settings for saving. Please try again.")?;
        self.connection.execute("INSERT INTO preferences (id,value) VALUES (1,?1) ON CONFLICT(id) DO UPDATE SET value=excluded.value", [json]).map_err(|_| "Could not save your settings. Please try again.")?;
        Ok(())
    }
    pub fn history(&self) -> Result<Vec<HistoryItem>, String> {
        let mut stmt = self.connection.prepare("SELECT id,created_at,text,language,seconds,cost,latency_ms FROM history ORDER BY id DESC LIMIT 500").map_err(|_| "Could not load your dictation history. Try reopening Bol.")?;
        let rows = stmt
            .query_map([], |r| {
                Ok(HistoryItem {
                    id: r.get(0)?,
                    created_at: r.get(1)?,
                    text: r.get(2)?,
                    language: r.get(3)?,
                    seconds: r.get(4)?,
                    cost: r.get(5)?,
                    latency_ms: r.get(6)?,
                })
            })
            .map_err(|_| "Could not load your dictation history. Try reopening Bol.")?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Could not load your dictation history. Try reopening Bol.".into())
    }
    pub fn add(
        &self,
        text: &str,
        language: &str,
        seconds: f64,
        cost: Option<f64>,
        latency: u64,
    ) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.connection.execute("INSERT INTO history (created_at,text,language,seconds,cost,latency_ms) VALUES (?1,?2,?3,?4,?5,?6)", params![now,text,language,seconds,cost,latency]).map_err(|_| "Your transcript is ready, but it could not be saved to history. Copy it before starting another dictation.")?;
        self.connection.execute("DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT 500)", []).map_err(|_| "The dictation was saved, but Bol could not remove older history entries.")?;
        Ok(())
    }
    pub fn delete(&self, id: Option<i64>) -> Result<(), String> {
        if let Some(id) = id {
            self.connection
                .execute("DELETE FROM history WHERE id=?1", [id])
        } else {
            self.connection.execute("DELETE FROM history", [])
        }
        .map_err(|_| "Could not delete the selected history. Please try again.")?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snippets_only_expand_complete_utterances() {
        let snippets = vec![Snippet {
            trigger: "my email".into(),
            text: "hello@example.com".into(),
        }];
        assert_eq!(
            expand_snippet(" My email! ", &snippets),
            "hello@example.com"
        );
        assert_eq!(
            expand_snippet("send this to my email", &snippets),
            "send this to my email"
        );
    }
    #[test]
    fn rejects_duplicate_triggers() {
        let s = Settings {
            snippets: vec![
                Snippet {
                    trigger: "Hello".into(),
                    text: "one".into(),
                },
                Snippet {
                    trigger: "hello!".into(),
                    text: "two".into(),
                },
            ],
            ..Settings::default()
        };
        assert!(s.validate().is_err());
    }
    #[test]
    fn saved_language_loads_without_losing_preferences() {
        let directory =
            std::env::temp_dir().join(format!("bol-language-migration-{}", std::process::id()));
        {
            let store = Storage::open(directory.clone()).unwrap();
            let settings = Settings {
                vocabulary: vec!["Ananya".into()],
                shortcut: "Control+Alt+B".into(),
                ..Settings::default()
            };
            let mut legacy = serde_json::to_value(&settings).unwrap();
            legacy["language"] = serde_json::json!("hi");
            store
                .connection
                .execute(
                    "INSERT INTO preferences (id,value) VALUES (1,?1)",
                    [legacy.to_string()],
                )
                .unwrap();
        }
        let store = Storage::open(directory.clone()).unwrap();
        let settings = store.settings();
        assert_eq!(settings.language, "hi");
        assert_eq!(settings.icon_position, "bottom-center");
        assert_eq!(settings.shortcut, "Control+Alt+B");
        assert_eq!(settings.vocabulary, ["Ananya"]);
        settings.validate().unwrap();
        drop(store);
        std::fs::remove_file(directory.join("bol.sqlite")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn missing_language_defaults_to_auto_and_invalid_values_are_rejected() {
        let settings: Settings =
            serde_json::from_str(r#"{"theme":"dark","style":"verbatim"}"#).unwrap();
        assert_eq!(settings.language, "auto");
        assert_eq!(settings.theme, "dark");
        assert_eq!(settings.style, "verbatim");
        let languages: serde_json::Value =
            serde_json::from_str(include_str!("../../src/languages.json")).unwrap();
        assert_eq!(languages.as_object().unwrap().len(), 61);
        for language in languages.as_object().unwrap().keys() {
            let settings = Settings {
                language: language.into(),
                ..Settings::default()
            };
            settings.validate().unwrap();
            let round_trip: Settings =
                serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
            assert_eq!(&round_trip.language, language);
        }
        assert!(Settings {
            language: "unknown".into(),
            ..Settings::default()
        }
        .validate()
        .is_err());
    }
    #[test]
    fn history_is_opt_in() {
        assert!(!Settings::default().history_enabled);
    }
    #[test]
    fn history_retention_and_deletion() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE history (id INTEGER PRIMARY KEY, created_at INTEGER, text TEXT, language TEXT, seconds REAL, cost REAL, latency_ms INTEGER)").unwrap();
        let store = Storage {
            connection,
            directory: PathBuf::new(),
        };
        for i in 0..505 {
            store
                .add(&format!("entry {i}"), "auto", 1.0, Some(0.001), 10)
                .unwrap();
        }
        let history = store.history().unwrap();
        assert_eq!(history.len(), 500);
        assert_eq!(history.first().unwrap().text, "entry 504");
        assert_eq!(history.last().unwrap().text, "entry 5");
        store.delete(Some(history[0].id)).unwrap();
        assert_eq!(store.history().unwrap().len(), 499);
        store.delete(None).unwrap();
        assert!(store.history().unwrap().is_empty());
    }
}
