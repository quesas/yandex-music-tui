use crate::theme::Theme;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quality {
    /// FLAC / lossless — максимальное качество (нужен Я.Плюс)
    #[default]
    Lossless,
    /// MP3 320 kbps
    High,
    /// AAC ~192 kbps
    Medium,
    /// AAC ~64 kbps
    Low,
}

impl Quality {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lossless => "FLAC",
            Self::High => "MP3 320",
            Self::Medium => "AAC 192",
            Self::Low => "AAC 64",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// OAuth-токен Яндекса, полученный через браузер или введённый вручную.
    pub oauth_token: Option<String>,
    /// Логин пользователя (`ynd-login` в Murglar).
    pub login: Option<String>,
    /// UID пользователя (`ynd-user-id` в Murglar).
    pub uid: Option<u64>,
    /// Активен ли Яндекс Плюс (`ynd-has-premium` в Murglar).
    pub has_plus: bool,
    /// Желаемое качество звука
    pub audio_quality: Quality,
    /// Громкость 0..=100
    pub volume: u8,
    /// Тема интерфейса
    pub theme: Theme,
    /// Показывать подсказки по клавишам
    pub show_hints: bool,
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
            show_hints: true,
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

    /// Файл, через который браузерный OAuth-callback передаёт токен запущенному TUI.
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

    /// Сохранить данные аккаунта, полученные из `/account/about`.
    pub fn apply_account(&mut self, account: &crate::api::AccountInfo) {
        self.login = account.login.clone();
        self.uid = account.uid;
        self.has_plus = account.plus;
    }

    /// Полный выход: токен и данные аккаунта удаляются (аналог `logout()` в Murglar).
    pub fn logout(&mut self) {
        self.oauth_token = None;
        self.login = None;
        self.uid = None;
        self.has_plus = false;
    }
}
