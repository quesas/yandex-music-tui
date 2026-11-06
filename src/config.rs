use crate::theme::Theme;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quality {
    #[default]
    Lossless,

    High,

    Medium,

    Low,
}

impl Quality {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lossless => "Превосходное",
            Self::High => "Высокое",
            Self::Medium => "Среднее",
            Self::Low => "Низкое",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Lossless => Self::High,
            Self::High => Self::Medium,
            Self::Medium => Self::Low,
            Self::Low => Self::Lossless,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Lossless => Self::Low,
            Self::High => Self::Lossless,
            Self::Medium => Self::High,
            Self::Low => Self::Medium,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub oauth_token: Option<String>,

    pub login: Option<String>,

    pub uid: Option<u64>,

    pub has_plus: bool,

    pub audio_quality: Quality,

    pub volume: u8,

    pub theme: Theme,

    pub wave_context: Option<String>,

    pub wave_settings: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            oauth_token: None,
            login: None,
            uid: None,
            has_plus: false,
            audio_quality: Quality::default(),
            volume: 70,
            theme: Theme::default(),
            wave_context: None,
            wave_settings: HashMap::new(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("yandex-music-tui")
            .join("config.toml")
    }

    pub fn pending_token_path() -> PathBuf {
        Self::path().with_file_name("pending_token")
    }

    pub fn store_pending_token(token: &str) -> anyhow::Result<()> {
        let path = Self::pending_token_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, token)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn take_pending_token() -> Option<String> {
        let path = Self::pending_token_path();
        let token = std::fs::read_to_string(&path).ok()?;
        let _ = std::fs::remove_file(&path);
        let token = token.trim().to_string();
        (!token.is_empty()).then_some(token)
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn apply_account(&mut self, account: &crate::api::AccountInfo) {
        self.login = account.login.clone();
        self.uid = account.uid;
        self.has_plus = account.plus;
    }

    pub fn logout(&mut self) {
        self.oauth_token = None;
        self.login = None;
        self.uid = None;
        self.has_plus = false;
        self.wave_context = None;
        self.wave_settings.clear();
    }
}
