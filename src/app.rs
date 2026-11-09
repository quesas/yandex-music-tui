use crate::api::{
    AccountInfo, Album, Artist, Playlist, SearchFilter, SearchItem, Track, WaveBatch, WaveSettings,
    YandexClient,
};
use crate::config::{Config, Quality};
use crate::cover::CoverImage;
use crate::lyrics::Lyrics;
use crate::mpris::{MprisCommand, MprisSnapshot, MprisTrack};
use crate::player::engine::PlayState;
use crate::player::{RepeatMode, TrackEnd};
use crate::ui;
use anyhow::Context;
use rand::seq::SliceRandom;
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::mpsc;

const TICK_MS: u64 = 250;

const SEEK_STEP: i64 = 5;

const VOLUME_STEP: u8 = 5;

const SETTINGS_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    Quit,
    Logout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Login,
    Wave,
    Library,
    Search,
    TrackList,
    Settings,
    WaveSetup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryTab {
    Tracks,
    Artists,
    Playlists,
}

impl LibraryTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tracks => "Треки",
            Self::Artists => "Артисты",
            Self::Playlists => "Плейлисты",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Tracks => 0,
            Self::Artists => 1,
            Self::Playlists => 2,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Tracks => Self::Artists,
            Self::Artists => Self::Playlists,
            Self::Playlists => Self::Tracks,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Tracks => Self::Playlists,
            Self::Artists => Self::Tracks,
            Self::Playlists => Self::Artists,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackSource {
    Other,
    Wave,
}

#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub input: String,
    pub editing: bool,
    pub filter: SearchFilter,
    pub results: Vec<SearchItem>,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            input: String::new(),
            editing: false,
            filter: SearchFilter::All,
            results: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct LyricsState {
    pub track_id: Option<String>,
    pub loading: bool,
    pub loaded: bool,
    pub error: Option<String>,
    pub lyrics: Lyrics,
}

#[derive(Debug, Default)]
pub struct CoverState {
    pub track_id: Option<String>,
    pub loading: bool,
    pub image: Option<CoverImage>,
}

#[derive(Debug, Clone)]
pub enum TrackListSource {
    Artist(Artist),
    Album(Album),
    Playlist(Playlist),
}

impl TrackListSource {
    fn title(&self) -> &str {
        match self {
            Self::Artist(artist) => &artist.name,
            Self::Album(album) => &album.title,
            Self::Playlist(playlist) => &playlist.title,
        }
    }

    fn subtitle(&self) -> String {
        match self {
            Self::Artist(artist) => artist.subtitle(),
            Self::Album(album) => album.subtitle(),
            Self::Playlist(playlist) => playlist.subtitle(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrackListState {
    pub title: String,
    pub subtitle: String,
    pub tracks: Vec<Track>,
    pub back: Screen,
    pub source: TrackListSource,
}

#[derive(Debug)]
pub struct App {
    pub config: Config,
    pub client: YandexClient,
    pub player: crate::player::Player,
    pub list_state: ListState,
    pub account: Option<AccountInfo>,
    pub status_message: String,
    pub should_exit: bool,
    pub show_help: bool,
    /// Смещение прокрутки окна подсказок (стрелки / PgUp / PgDn).
    pub help_scroll: u16,
    pub confirm: Option<ConfirmAction>,

    pub screen: Screen,
    pub settings_back: Screen,
    pub login_token_input: String,
    pub login_token_editing: bool,
    volume_before_mute: u8,

    pub wave: Vec<Track>,
    pub wave_revealed: usize,
    pub wave_session_id: Option<String>,
    pub wave_batch_id: Option<String>,
    pub wave_queue: Vec<String>,
    pub wave_loading: bool,
    pub pending_wave: bool,
    pub wave_settings: Option<WaveSettings>,
    pub wave_settings_loading: bool,
    pub wave_prefs_dirty: bool,

    pub library_tab: LibraryTab,
    pub library_tracks: Vec<Track>,
    pub library_artists: Vec<Artist>,
    pub library_playlists: Vec<Playlist>,
    pub library_loaded: [bool; 3],

    pub search: SearchState,
    pub track_list: Option<TrackListState>,

    /// Открыта ли правая панель с текстом песни и обложкой.
    pub show_lyrics: bool,
    pub lyrics: LyricsState,
    pub cover: CoverState,
    /// Плавное смещение текста в строках (может быть дробным).
    pub lyrics_scroll: f32,
    /// Высота видимой области текста, обновляется при отрисовке.
    pub lyrics_view_height: u16,

    pub playback_source: PlaybackSource,

    pub liked_ids: HashSet<String>,
    pub disliked_ids: HashSet<String>,
    pub ratings_loaded: bool,

    wave_seq: u64,
    library_seq: u64,
    search_seq: u64,
    tracklist_seq: u64,
    audio_seq: u64,
    ratings_seq: u64,
    rating_seq: u64,
    wave_settings_seq: u64,
    lyrics_seq: u64,
    cover_seq: u64,
}

impl App {
    fn logout(&mut self) -> anyhow::Result<()> {
        self.config.logout();
        self.config.save()?;
        self.client = YandexClient::new(None);
        self.account = None;
        self.wave.clear();
        self.wave_revealed = 0;
        self.wave_session_id = None;
        self.wave_batch_id = None;
        self.wave_queue.clear();
        self.library_tracks.clear();
        self.library_artists.clear();
        self.library_playlists.clear();
        self.library_loaded = [false; 3];
        self.liked_ids.clear();
        self.disliked_ids.clear();
        self.ratings_loaded = false;
        self.pending_wave = false;
        self.wave_settings = None;
        self.wave_settings_loading = false;
        self.wave_prefs_dirty = false;
        self.show_lyrics = false;
        self.lyrics = LyricsState::default();
        self.cover = CoverState::default();
        self.lyrics_scroll = 0.0;
        self.player.stop();
        self.audio_seq += 1;
        self.tracklist_seq += 1;
        self.track_list = None;
        self.screen = Screen::Login;
        self.reset_selection();
        Ok(())
    }

    fn uid(&self) -> Option<u64> {
        self.config
            .uid
            .or_else(|| self.account.as_ref().and_then(|a| a.uid))
    }

    pub fn has_token(&self) -> bool {
        self.client.token.is_some()
    }

    pub fn palette(&self) -> crate::theme::Palette {
        self.config.theme.palette()
    }

    fn open_settings(&mut self) {
        if self.screen != Screen::Settings {
            self.settings_back = self.screen;
        }
        self.screen = Screen::Settings;
        self.reset_selection();
    }

    pub fn row_count(&self) -> usize {
        match self.screen {
            Screen::Login => 0,
            Screen::Wave => self.wave_revealed.min(self.wave.len()),
            Screen::Library => match self.library_tab {
                LibraryTab::Tracks => self.library_tracks.len(),
                LibraryTab::Artists => self.library_artists.len(),
                LibraryTab::Playlists => self.library_playlists.len(),
            },
            Screen::Search => self.search.results.len(),
            Screen::TrackList => self
                .track_list
                .as_ref()
                .map(|list| list.tracks.len())
                .unwrap_or(0),
            Screen::Settings => SETTINGS_COUNT,
            Screen::WaveSetup => self.wave_setup_len(),
        }
    }

    fn wave_setup_len(&self) -> usize {
        match &self.wave_settings {
            Some(settings) => 1 + settings.groups.len() + 2,
            None => 1,
        }
    }

    /// Позиции строк «Применить» и «Сбросить» на экране настроек волны.
    fn wave_setup_actions(&self) -> Option<(usize, usize)> {
        let settings = self.wave_settings.as_ref()?;
        let apply = 1 + settings.groups.len();
        Some((apply, apply + 1))
    }

    pub fn reset_selection(&mut self) {
        let len = self.row_count();
        if len == 0 {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    pub fn selected(&self) -> Option<usize> {
        let len = self.row_count();
        if len == 0 {
            None
        } else {
            Some(self.list_state.selected().unwrap_or(0).min(len - 1))
        }
    }

    pub fn is_liked(&self, track: &Track) -> bool {
        self.liked_ids.contains(&track.id)
    }

    pub fn is_disliked(&self, track: &Track) -> bool {
        self.disliked_ids.contains(&track.id)
    }

    /// Индекс текущей строки синхронизированного текста.
    pub fn current_lyric_line(&self) -> Option<usize> {
        self.lyrics
            .lyrics
            .current_index(self.player.engine.position)
    }

    fn ensure_login(&mut self) -> bool {
        if self.has_token() {
            true
        } else {
            self.screen = Screen::Login;
            self.status_message = "Для этого раздела нужен вход в Яндекс Музыку".into();
            self.reset_selection();
            false
        }
    }

    fn discard_track_list(&mut self) {
        if self.screen == Screen::TrackList {
            self.tracklist_seq += 1;
            self.track_list = None;
        }
    }
}

enum AppAction {
    None,
    TriggerLogin,
    Logout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RatingAction {
    Like,
    Unlike,
    Dislike,
    Undislike,
}

impl RatingAction {
    fn label(self) -> &'static str {
        match self {
            Self::Like => "Лайк поставлен",
            Self::Unlike => "Лайк убран",
            Self::Dislike => "Дизлайк поставлен",
            Self::Undislike => "Дизлайк убран",
        }
    }
}

enum ApiEvent {
    Account {
        result: Result<AccountInfo, String>,
        explicit_login: bool,
    },
    Wave {
        result: Result<WaveBatch, String>,
        append: bool,
        seq: u64,
    },
    WaveSettings {
        result: Result<WaveSettings, String>,
        seq: u64,
    },
    Library {
        result: Result<LibraryData, String>,
        tab: LibraryTab,
        seq: u64,
    },
    Search {
        result: Result<Vec<SearchItem>, String>,
        seq: u64,
    },
    TrackList {
        result: Result<TrackListData, String>,
        seq: u64,
    },
    Audio {
        result: Result<Vec<u8>, String>,
        seq: u64,
    },
    Ratings {
        result: Result<(HashSet<String>, HashSet<String>), String>,
        seq: u64,
    },
    Rating {
        result: Result<(), String>,
        action: RatingAction,
        track: Track,
        seq: u64,
    },
    Lyrics {
        result: Result<String, String>,
        track_id: String,
        seq: u64,
    },
    Cover {
        result: Result<CoverImage, String>,
        track_id: String,
        seq: u64,
    },
}

enum LibraryData {
    Tracks(Vec<Track>),
    Artists(Vec<Artist>),
    Playlists(Vec<Playlist>),
}

struct TrackListData {
    title: String,
    subtitle: String,
    tracks: Vec<Track>,
}

pub async fn run() -> anyhow::Result<()> {
    let config = Config::load();
    let initial_token = Config::take_pending_token().or_else(|| config.oauth_token.clone());
    let has_token = initial_token.is_some();
    let mut app = App {
        client: YandexClient::new(initial_token),
        wave: Vec::new(),
        wave_revealed: 0,
        wave_session_id: None,
        wave_batch_id: None,
        wave_queue: Vec::new(),
        wave_loading: false,
        pending_wave: false,
        wave_settings: None,
        wave_settings_loading: false,
        wave_prefs_dirty: false,
        library_tab: LibraryTab::Tracks,
        library_tracks: Vec::new(),
        library_artists: Vec::new(),
        library_playlists: Vec::new(),
        library_loaded: [false; 3],
        search: SearchState::default(),
        track_list: None,
        show_lyrics: false,
        lyrics: LyricsState::default(),
        cover: CoverState::default(),
        lyrics_scroll: 0.0,
        lyrics_view_height: 0,
        playback_source: PlaybackSource::Other,
        liked_ids: HashSet::new(),
        disliked_ids: HashSet::new(),
        ratings_loaded: false,
        screen: if has_token {
            Screen::Wave
        } else {
            Screen::Login
        },
        settings_back: Screen::Wave,
        login_token_input: String::new(),
        login_token_editing: false,
        volume_before_mute: config.volume.max(1),
        player: crate::player::Player::default(),
        list_state: ListState::default(),
        account: None,
        status_message: if has_token {
            "Проверка авторизации...".into()
        } else {
            "Вход: Enter — через Яндекс, t — вставить токен".into()
        },
        should_exit: false,
        show_help: false,
        help_scroll: 0,
        confirm: None,
        config,
        wave_seq: 0,
        library_seq: 0,
        search_seq: 0,
        tracklist_seq: 0,
        audio_seq: 0,
        ratings_seq: 0,
        rating_seq: 0,
        wave_settings_seq: 0,
        lyrics_seq: 0,
        cover_seq: 0,
    };
    app.player.engine.volume = app.config.volume;
    app.player.repeat = app.config.repeat;
    app.volume_before_mute = app.config.volume.max(1);
    app.reset_selection();

    let (api_tx, mut api_rx) = mpsc::channel::<ApiEvent>(64);
    let (mpris_cmd_tx, mut mpris_cmd_rx) = mpsc::unbounded_channel::<MprisCommand>();
    let mpris_tx = crate::mpris::spawn(mpris_cmd_tx);

    if app.has_token() {
        spawn_account(api_tx.clone(), app.client.clone(), false);
    }

    let mut terminal = ratatui::init();
    let tick_rate = Duration::from_millis(TICK_MS);
    let mut last_tick = tokio::time::Instant::now();

    while !app.should_exit {
        if let Some(token) = Config::take_pending_token() {
            terminal.clear()?;
            app.config.oauth_token = Some(token.clone());
            app.client = YandexClient::new(Some(token));
            app.login_token_editing = false;
            app.status_message = "Токен получен из браузера, проверка профиля...".into();
            spawn_account(api_tx.clone(), app.client.clone(), true);
        }

        while let Ok(command) = mpris_cmd_rx.try_recv() {
            handle_mpris_command(&mut app, command, &api_tx);
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        while let Ok(event) = api_rx.try_recv() {
            handle_api_event(&mut app, event, &api_tx);
        }

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        let ev_poll = tokio::task::spawn_blocking(move || crossterm::event::poll(timeout));
        let ev = ev_poll.await.context("event poll аварийно завершился")??;

        let mut action = AppAction::None;
        if ev {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key) => {
                    action = handle_key(&mut app, key, &api_tx);
                }
                crossterm::event::Event::Paste(text) => handle_paste(&mut app, &text),
                _ => {}
            }
        }

        match action {
            AppAction::TriggerLogin => {
                let handler_result = crate::auth::register_uri_handler();
                let browser_result = crate::auth::open_auth_url();
                app.login_token_editing = true;
                app.login_token_input.clear();
                app.status_message = match (handler_result, browser_result) {
                    (Ok(()), Ok(())) => {
                        "Браузер открыт. После входа нажмите «Открыть приложение» — токен подхватится автоматически"
                            .into()
                    }
                    (Err(e), Ok(())) => format!(
                        "Браузер открыт, но обработчик не установлен: {e}. Скопируйте адрес кнопки «Открыть приложение» и вставьте сюда"
                    ),
                    (_, Err(e)) => format!(
                        "Не удалось открыть браузер: {e}. Откройте страницу входа вручную и вставьте токен"
                    ),
                };
                terminal.clear()?;
            }
            AppAction::Logout => match app.logout() {
                Ok(()) => app.status_message = "Вы вышли из Яндекс Музыки".into(),
                Err(e) => app.status_message = format!("Ошибка сохранения конфига: {e}"),
            },
            AppAction::None => {}
        }

        let now = tokio::time::Instant::now();
        let dt = now - last_tick;
        last_tick = now;
        on_tick(&mut app, dt, &api_tx);
        let _ = mpris_tx.send(mpris_snapshot(&app));
    }

    app.config.volume = app.player.engine.volume;
    let _ = app.config.save();

    ratatui::restore();
    Ok(())
}

fn handle_api_event(app: &mut App, event: ApiEvent, tx: &mpsc::Sender<ApiEvent>) {
    match event {
        ApiEvent::Account {
            result,
            explicit_login,
        } => match result {
            Ok(info) => {
                if app.client.token.is_some() {
                    app.status_message = format!(
                        "Вход: {} (Плюс: {})",
                        info.nickname,
                        if info.plus {
                            "активен"
                        } else {
                            "нет"
                        }
                    );
                    app.config.apply_account(&info);
                    let _ = app.config.save();
                    app.account = Some(info);
                    load_ratings(app, tx);
                    if app.screen == Screen::Login {
                        app.screen = Screen::Wave;
                    }
                    if app.screen == Screen::Wave && app.wave.is_empty() {
                        start_wave_load(app, tx, false);
                    }
                    if app.screen == Screen::Library {
                        load_library(app, tx);
                    }
                }
            }
            Err(e) => {
                if explicit_login || e.contains("истекла") {
                    let _ = app.logout();
                    app.status_message = format!("Вход не выполнен: {e}");
                } else {
                    app.status_message = format!("Ошибка профиля: {e}");
                }
            }
        },
        ApiEvent::Wave {
            result,
            append,
            seq,
        } => {
            if seq != app.wave_seq {
                return;
            }
            app.wave_loading = false;
            match result {
                Ok(batch) => {
                    let old_len = app.wave.len();
                    if batch.session_id.is_some() {
                        app.wave_session_id = batch.session_id;
                    }
                    if batch.batch_id.is_some() {
                        app.wave_batch_id = batch.batch_id;
                    }
                    if !batch.queue.is_empty() {
                        app.wave_queue = batch.queue;
                    }

                    if append {
                        app.wave.extend(batch.tracks);
                        if app.pending_wave {
                            app.pending_wave = false;
                            if old_len < app.wave.len() {
                                app.wave_revealed = app.wave_revealed.max(old_len + 1);
                                app.player.prepare_at(&app.wave, old_len);
                                app.playback_source = PlaybackSource::Wave;
                                start_audio_load(app, tx);
                            }
                        }
                        app.status_message = format!("Моя волна: {} треков", app.wave.len());
                    } else {
                        app.wave = batch.tracks;
                        app.wave_revealed = usize::from(!app.wave.is_empty());
                        app.status_message =
                            format!("Моя волна загружена: {} треков", app.wave.len());
                        if app.screen == Screen::Wave {
                            app.reset_selection();
                        }
                    }
                }
                Err(e) => {
                    app.pending_wave = false;
                    if !handle_session_error(app, &e) {
                        app.status_message = format!("Ошибка волны: {e}");
                    }
                }
            }
        }
        ApiEvent::WaveSettings { result, seq } => {
            if seq != app.wave_settings_seq {
                return;
            }
            app.wave_settings_loading = false;
            match result {
                Ok(settings) => {
                    app.wave_settings = Some(settings);
                    app.status_message = "Настройки волны загружены".into();
                }
                Err(e) => {
                    app.wave_settings = Some(WaveSettings::fallback());
                    if !handle_session_error(app, &e) {
                        app.status_message = format!(
                            "Не удалось загрузить настройки волны, используются стандартные: {e}"
                        );
                    }
                }
            }
            if app.screen == Screen::WaveSetup {
                app.reset_selection();
            }
        }
        ApiEvent::Library { result, tab, seq } => {
            if seq != app.library_seq {
                return;
            }
            match result {
                Ok(data) => {
                    app.library_loaded[tab.index()] = true;
                    match data {
                        LibraryData::Tracks(tracks) => {
                            app.library_tracks = tracks;
                            app.status_message =
                                format!("Библиотека: {} любимых треков", app.library_tracks.len());
                        }
                        LibraryData::Artists(artists) => {
                            app.library_artists = artists;
                            app.status_message =
                                format!("Библиотека: {} артистов", app.library_artists.len());
                        }
                        LibraryData::Playlists(playlists) => {
                            app.library_playlists = playlists;
                            app.status_message =
                                format!("Библиотека: {} плейлистов", app.library_playlists.len());
                        }
                    }
                    if app.screen == Screen::Library && app.library_tab == tab {
                        app.reset_selection();
                    }
                }
                Err(e) => {
                    if !handle_session_error(app, &e) {
                        app.status_message = format!("Ошибка библиотеки: {e}");
                    }
                }
            }
        }
        ApiEvent::Search { result, seq } => {
            if seq != app.search_seq {
                return;
            }
            match result {
                Ok(items) => {
                    app.search.results = items;
                    app.status_message = format!(
                        "Поиск «{}»: {} результатов ({})",
                        app.search.query,
                        app.search.results.len(),
                        app.search.filter.label()
                    );
                    if app.screen == Screen::Search {
                        app.reset_selection();
                    }
                }
                Err(e) => {
                    if !handle_session_error(app, &e) {
                        app.status_message = format!("Ошибка поиска: {e}");
                    }
                }
            }
        }
        ApiEvent::Audio { result, seq } => {
            if seq != app.audio_seq {
                return;
            }
            match result {
                Ok(bytes) => match app.player.play_bytes(bytes) {
                    Ok(()) => {
                        app.status_message = format!("Играет: {}", app.player.now_playing());
                    }
                    Err(e) => {
                        app.player.stop();
                        app.status_message = format!("Ошибка аудио: {e}");
                    }
                },
                Err(e) => {
                    app.player.stop();
                    if !handle_session_error(app, &e) {
                        app.status_message = format!("Ошибка загрузки трека: {e}");
                    }
                }
            }
        }
        ApiEvent::TrackList { result, seq } => {
            if seq != app.tracklist_seq {
                return;
            }
            match result {
                Ok(data) => {
                    let Some(previous) = app.track_list.as_ref() else {
                        return;
                    };
                    let back = previous.back;
                    let source = previous.source.clone();
                    app.track_list = Some(TrackListState {
                        title: data.title,
                        subtitle: data.subtitle,
                        tracks: data.tracks,
                        back,
                        source,
                    });
                    app.screen = Screen::TrackList;
                    app.status_message = format!(
                        "{}: {} треков",
                        app.track_list
                            .as_ref()
                            .map(|l| l.title.as_str())
                            .unwrap_or(""),
                        app.track_list.as_ref().map(|l| l.tracks.len()).unwrap_or(0)
                    );
                    app.reset_selection();
                }
                Err(e) => {
                    let back = app
                        .track_list
                        .as_ref()
                        .map(|list| list.back)
                        .unwrap_or(Screen::Library);
                    app.track_list = None;
                    app.screen = back;
                    app.reset_selection();
                    if !handle_session_error(app, &e) {
                        app.status_message = format!("Ошибка загрузки треков: {e}");
                    }
                }
            }
        }
        ApiEvent::Ratings { result, seq } => {
            if seq != app.ratings_seq {
                return;
            }
            match result {
                Ok((liked, disliked)) => {
                    app.liked_ids = liked.into_iter().map(|id| track_key(&id)).collect();
                    app.disliked_ids = disliked.into_iter().map(|id| track_key(&id)).collect();
                    app.ratings_loaded = true;
                }
                Err(e) => {
                    if !handle_session_error(app, &e) {
                        app.status_message = format!("Не удалось загрузить лайки: {e}");
                    }
                }
            }
        }
        ApiEvent::Rating {
            result,
            action,
            track,
            seq,
        } => {
            if seq != app.rating_seq {
                return;
            }
            match result {
                Ok(()) => {
                    apply_rating(app, action, &track);
                    app.status_message =
                        format!("{}: {} — {}", action.label(), track.artist, track.title);
                    if matches!(action, RatingAction::Unlike | RatingAction::Dislike)
                        && app.screen == Screen::Library
                        && app.library_tab == LibraryTab::Tracks
                    {
                        remove_from_library(app, &track);
                    }
                }
                Err(e) => {
                    if !handle_session_error(app, &e) {
                        app.status_message = format!("Ошибка лайка: {e}");
                    }
                }
            }
        }
        ApiEvent::Lyrics {
            result,
            track_id,
            seq,
        } => {
            if seq != app.lyrics_seq || app.lyrics.track_id.as_deref() != Some(&track_id) {
                return;
            }
            app.lyrics.loading = false;
            app.lyrics.loaded = true;
            match result {
                Ok(raw) => {
                    let lyrics = Lyrics::parse(&raw);
                    if lyrics.lines.is_empty() {
                        app.lyrics.error = Some("Текст песни не найден".into());
                    } else {
                        app.lyrics.lyrics = lyrics;
                        app.lyrics.error = None;
                        app.lyrics_scroll = 0.0;
                    }
                }
                Err(e) => {
                    app.lyrics.loaded = false;
                    app.lyrics.error = Some(e);
                }
            }
        }
        ApiEvent::Cover {
            result,
            track_id,
            seq,
        } => {
            if seq != app.cover_seq || app.cover.track_id.as_deref() != Some(&track_id) {
                return;
            }
            app.cover.loading = false;
            if let Ok(image) = result {
                app.cover.image = Some(image);
            }
        }
    }
}

fn track_key(full_id: &str) -> String {
    full_id.split(':').next().unwrap_or(full_id).to_string()
}

fn apply_rating(app: &mut App, action: RatingAction, track: &Track) {
    let key = track.id.clone();
    match action {
        RatingAction::Like => {
            app.disliked_ids.remove(&key);
            app.liked_ids.insert(key);
        }
        RatingAction::Unlike => {
            app.liked_ids.remove(&key);
        }
        RatingAction::Dislike => {
            app.liked_ids.remove(&key);
            app.disliked_ids.insert(key);
        }
        RatingAction::Undislike => {
            app.disliked_ids.remove(&key);
        }
    }
}

fn remove_from_library(app: &mut App, track: &Track) {
    let Some(index) = app
        .library_tracks
        .iter()
        .position(|item| item.id == track.id)
    else {
        return;
    };
    app.library_tracks.remove(index);
    let len = app.library_tracks.len();
    if len == 0 {
        app.list_state.select(None);
    } else if let Some(selected) = app.list_state.selected() {
        if selected >= len {
            app.list_state.select(Some(len - 1));
        } else if selected > index {
            app.list_state.select(Some(selected - 1));
        }
    }
}

fn handle_session_error(app: &mut App, error: &str) -> bool {
    if error.contains("истекла") {
        let _ = app.logout();
        app.status_message = "Сессия истекла. Войдите снова (Enter) или вставьте токен (t)".into();
        true
    } else {
        false
    }
}

fn spawn_account(tx: mpsc::Sender<ApiEvent>, client: YandexClient, explicit_login: bool) {
    tokio::spawn(async move {
        let result = client.account().await.map_err(|e| e.to_string());
        let _ = tx
            .send(ApiEvent::Account {
                result,
                explicit_login,
            })
            .await;
    });
}

fn wave_seeds(app: &App) -> Vec<String> {
    let context = app
        .config
        .wave_context
        .as_deref()
        .filter(|seed| !seed.is_empty());

    let mut seeds = vec![context.unwrap_or("user:onyourwave").to_string()];
    let mut settings: Vec<(&String, &String)> = app.config.wave_settings.iter().collect();
    settings.sort_by_key(|(key, _)| match key.as_str() {
        "moodEnergy" => 0,
        "language" => 1,
        "diversity" => 2,
        _ => 3,
    });
    for (_, seed) in settings {
        if !seed.is_empty() && !seeds.contains(seed) {
            seeds.push(seed.clone());
        }
    }

    if context.is_none() && seeds.len() == 1 {
        seeds.clear();
    }
    seeds
}

fn wave_context_label(app: &App) -> String {
    let Some(seed) = app.config.wave_context.as_deref() else {
        return "Моя волна".to_string();
    };
    app.wave_settings
        .as_ref()
        .and_then(|settings| {
            settings
                .contexts
                .iter()
                .find(|context| context.seed == seed)
        })
        .map(|context| context.name.clone())
        .unwrap_or_else(|| seed.to_string())
}

fn load_wave_settings(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    if app.wave_settings.is_some() || app.wave_settings_loading {
        return;
    }
    if !app.ensure_login() {
        return;
    }

    app.wave_settings_seq += 1;
    let seq = app.wave_settings_seq;
    let client = app.client.clone();
    let tx = tx.clone();
    app.wave_settings_loading = true;
    app.status_message = "Загружаем настройки волны...".into();

    tokio::spawn(async move {
        let result = client.wave_settings().await.map_err(|e| e.to_string());
        let _ = tx.send(ApiEvent::WaveSettings { result, seq }).await;
    });
}

fn open_wave_setup(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    if !app.ensure_login() {
        return;
    }
    app.screen = Screen::WaveSetup;
    app.wave_prefs_dirty = false;
    app.reset_selection();
    load_wave_settings(app, tx);
    app.status_message = "Настройки волны: ←/→ — значение, Enter — применить, Esc — назад".into();
}

fn cycle_wave_setup(app: &mut App, forward: bool) {
    let Some(settings) = app.wave_settings.clone() else {
        return;
    };
    let Some(index) = app.selected() else {
        return;
    };

    if index == 0 {
        let mut contexts = vec![None];
        contexts.extend(
            settings
                .contexts
                .iter()
                .map(|context| Some(context.seed.clone())),
        );
        let current = app.config.wave_context.clone();
        let position = contexts
            .iter()
            .position(|context| *context == current)
            .unwrap_or(0);
        let next = if forward {
            (position + 1) % contexts.len()
        } else {
            (position + contexts.len() - 1) % contexts.len()
        };
        app.config.wave_context = contexts[next].clone();
        app.wave_prefs_dirty = true;
        let _ = app.config.save();
        app.status_message = format!("Волна: {}", wave_context_label(app));
        return;
    }

    let Some(group) = settings.groups.get(index - 1) else {
        return;
    };
    let current = app
        .config
        .wave_settings
        .get(&group.key)
        .cloned()
        .filter(|seed| !seed.is_empty());
    let current_position = group.values.iter().position(|value| value.seed == current);
    let next = match current_position {
        Some(position) if forward => (position + 1) % group.values.len(),
        Some(position) => (position + group.values.len() - 1) % group.values.len(),
        None if forward => 0,
        None => group.values.len() - 1,
    };
    let value = &group.values[next];
    match &value.seed {
        Some(seed) => {
            app.config
                .wave_settings
                .insert(group.key.clone(), seed.clone());
        }
        None => {
            app.config.wave_settings.remove(&group.key);
        }
    }
    app.wave_prefs_dirty = true;
    let _ = app.config.save();
    app.status_message = format!("{}: {}", group.name, value.name);
}

fn reset_wave_setup(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    app.config.reset_wave_settings();
    app.wave_prefs_dirty = false;
    let _ = app.config.save();
    restart_wave(app, tx);
    app.status_message = "Настройки Моей волны сброшены · волна перезапущена".into();
}

fn restart_wave(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    app.wave_seq += 1;
    app.audio_seq += 1;
    app.wave_loading = false;
    app.pending_wave = false;
    app.wave_session_id = None;
    app.wave_batch_id = None;
    app.wave_queue.clear();
    app.wave_revealed = 0;
    app.wave.clear();
    app.player.stop();
    let _ = app.config.save();
    start_wave_load(app, tx, false);
}

fn start_wave_load(app: &mut App, tx: &mpsc::Sender<ApiEvent>, append: bool) {
    if !app.has_token() {
        app.pending_wave = false;
        app.screen = Screen::Login;
        app.reset_selection();
        return;
    }
    if app.wave_loading {
        return;
    }

    app.wave_seq += 1;
    let seq = app.wave_seq;
    let client = app.client.clone();
    let tx = tx.clone();
    let session_id = app.wave_session_id.clone();
    let queue = app.wave_queue.clone();
    let seeds = wave_seeds(app);
    app.wave_loading = true;
    if !append {
        app.pending_wave = false;
    }
    app.status_message = if append {
        "Подгружаем следующие треки волны...".into()
    } else {
        "Загружаем Мою волну...".into()
    };

    tokio::spawn(async move {
        let result = if append {
            match session_id {
                Some(session) => client.next_wave(&session, &queue).await,
                None => client.start_wave(&seeds).await,
            }
        } else {
            client.start_wave(&seeds).await
        };
        let _ = tx
            .send(ApiEvent::Wave {
                result: result.map_err(|e| e.to_string()),
                append,
                seq,
            })
            .await;
    });
}

fn load_library(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    if !app.ensure_login() {
        return;
    }
    let Some(uid) = app.uid() else {
        app.status_message = "Не удалось определить UID пользователя".into();
        return;
    };
    let tab = app.library_tab;
    app.library_seq += 1;
    let seq = app.library_seq;
    let client = app.client.clone();
    let tx = tx.clone();
    app.status_message = format!("Загружаем библиотеку: {}...", tab.label());

    tokio::spawn(async move {
        let result = match tab {
            LibraryTab::Tracks => client
                .liked_tracks(uid)
                .await
                .map(LibraryData::Tracks)
                .map_err(|e| e.to_string()),
            LibraryTab::Artists => client
                .liked_artists(uid)
                .await
                .map(LibraryData::Artists)
                .map_err(|e| e.to_string()),
            LibraryTab::Playlists => client
                .liked_playlists(uid)
                .await
                .map(LibraryData::Playlists)
                .map_err(|e| e.to_string()),
        };
        let _ = tx.send(ApiEvent::Library { result, tab, seq }).await;
    });
}

fn load_ratings(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    let Some(uid) = app.uid() else {
        return;
    };
    app.ratings_seq += 1;
    let seq = app.ratings_seq;
    let client = app.client.clone();
    let tx = tx.clone();
    app.ratings_loaded = false;

    tokio::spawn(async move {
        let result = async {
            let liked = client.liked_track_ids(uid).await?;
            let disliked = client.disliked_track_ids(uid).await?;
            Ok::<_, anyhow::Error>((liked, disliked))
        }
        .await
        .map_err(|e| e.to_string());
        let _ = tx.send(ApiEvent::Ratings { result, seq }).await;
    });
}

fn rating_target(app: &App) -> Option<Track> {
    let selected = |tracks: &[Track]| app.selected().and_then(|index| tracks.get(index).cloned());
    let current = || app.player.current_track().cloned();

    match app.screen {
        Screen::Wave => current().or_else(|| selected(&app.wave)),
        Screen::Library => match app.library_tab {
            LibraryTab::Tracks => selected(&app.library_tracks).or_else(current),
            _ => current(),
        },
        Screen::Search => app
            .selected()
            .and_then(|index| match app.search.results.get(index) {
                Some(SearchItem::Track(track)) => Some(track.clone()),
                _ => None,
            })
            .or_else(current),
        Screen::TrackList => app
            .track_list
            .as_ref()
            .and_then(|list| selected(&list.tracks))
            .or_else(current),
        Screen::Login | Screen::Settings | Screen::WaveSetup => None,
    }
}

fn toggle_like(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    let Some(track) = rating_target(app) else {
        app.status_message = "Нет трека для лайка".into();
        return;
    };
    if !app.ensure_login() {
        return;
    }
    if !app.ratings_loaded {
        load_ratings(app, tx);
        app.status_message = "Загружаем состояние лайков, повторите через секунду...".into();
        return;
    }
    let action = if app.liked_ids.contains(&track.id) {
        RatingAction::Unlike
    } else {
        RatingAction::Like
    };
    send_rating(app, tx, action, track);
}

fn toggle_dislike(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    let Some(track) = rating_target(app) else {
        app.status_message = "Нет трека для дизлайка".into();
        return;
    };
    if !app.ensure_login() {
        return;
    }
    if !app.ratings_loaded {
        load_ratings(app, tx);
        app.status_message = "Загружаем состояние лайков, повторите через секунду...".into();
        return;
    }
    let action = if app.disliked_ids.contains(&track.id) {
        RatingAction::Undislike
    } else {
        RatingAction::Dislike
    };
    send_rating(app, tx, action, track);
}

fn send_rating(app: &mut App, tx: &mpsc::Sender<ApiEvent>, action: RatingAction, track: Track) {
    let Some(uid) = app.uid() else {
        app.status_message = "Не удалось определить UID пользователя".into();
        return;
    };
    app.rating_seq += 1;
    let seq = app.rating_seq;
    let client = app.client.clone();
    let tx = tx.clone();
    let track_id = track.full_id();
    app.status_message = format!("{}: {} — {}...", action.label(), track.artist, track.title);

    tokio::spawn(async move {
        let result = match action {
            RatingAction::Like => client.set_track_like(uid, &track_id, true).await,
            RatingAction::Unlike => client.set_track_like(uid, &track_id, false).await,
            RatingAction::Dislike => client.set_track_dislike(uid, &track_id, true).await,
            RatingAction::Undislike => client.set_track_dislike(uid, &track_id, false).await,
        }
        .map_err(|e| e.to_string());
        let _ = tx
            .send(ApiEvent::Rating {
                result,
                action,
                track,
                seq,
            })
            .await;
    });
}

fn run_search(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    let text = app.search.query.trim().to_string();
    if text.is_empty() {
        app.status_message = "Введите поисковый запрос".into();
        return;
    }
    app.search.query = text.clone();
    app.search_seq += 1;
    let seq = app.search_seq;
    let client = app.client.clone();
    let tx = tx.clone();
    let filter = app.search.filter;
    app.status_message = format!("Ищем «{text}» ({}...)", filter.label());

    tokio::spawn(async move {
        let result = client
            .search(&text, filter, 0)
            .await
            .map_err(|e| e.to_string());
        let _ = tx.send(ApiEvent::Search { result, seq }).await;
    });
}

fn open_artist(app: &mut App, artist: Artist, tx: &mpsc::Sender<ApiEvent>) {
    start_track_list(app, TrackListSource::Artist(artist), app.screen, tx);
}

fn open_album(app: &mut App, album: Album, tx: &mpsc::Sender<ApiEvent>) {
    start_track_list(app, TrackListSource::Album(album), app.screen, tx);
}

fn open_playlist(app: &mut App, playlist: Playlist, tx: &mpsc::Sender<ApiEvent>) {
    start_track_list(app, TrackListSource::Playlist(playlist), app.screen, tx);
}

fn reload_track_list(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    let Some((source, back)) = app
        .track_list
        .as_ref()
        .map(|list| (list.source.clone(), list.back))
    else {
        return;
    };
    start_track_list(app, source, back, tx);
}

fn start_track_list(
    app: &mut App,
    source: TrackListSource,
    back: Screen,
    tx: &mpsc::Sender<ApiEvent>,
) {
    app.tracklist_seq += 1;
    let seq = app.tracklist_seq;
    let client = app.client.clone();
    let tx = tx.clone();
    let title = source.title().to_string();
    let subtitle = source.subtitle();
    app.track_list = Some(TrackListState {
        title: title.clone(),
        subtitle: subtitle.clone(),
        tracks: Vec::new(),
        back,
        source: source.clone(),
    });
    app.screen = Screen::TrackList;
    app.status_message = format!("Загружаем «{title}»...");
    app.reset_selection();

    tokio::spawn(async move {
        let result = match source {
            TrackListSource::Artist(artist) => client.artist_tracks(&artist.id).await,
            TrackListSource::Album(album) => client.album_tracks(&album.id).await,
            TrackListSource::Playlist(playlist) => {
                client.playlist_tracks(&playlist.uid, &playlist.kind).await
            }
        }
        .map(|tracks| TrackListData {
            title,
            subtitle,
            tracks,
        })
        .map_err(|e| e.to_string());
        let _ = tx.send(ApiEvent::TrackList { result, seq }).await;
    });
}

fn play_current(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    let Some(index) = app.selected() else {
        return;
    };

    match app.screen {
        Screen::Wave => {
            app.player.prepare_at(&app.wave, index);
            app.playback_source = PlaybackSource::Wave;
            app.wave_revealed = app.wave_revealed.max(index + 1).min(app.wave.len());
            start_audio_load(app, tx);
        }
        Screen::Library => match app.library_tab {
            LibraryTab::Tracks => {
                app.player.prepare_at(&app.library_tracks, index);
                app.playback_source = PlaybackSource::Other;
                start_audio_load(app, tx);
            }
            LibraryTab::Artists => {}
            LibraryTab::Playlists => {}
        },
        Screen::Search => {
            let selected = app.search.results.get(index).cloned();
            match selected {
                Some(SearchItem::Track(track)) => {
                    let tracks: Vec<Track> = app
                        .search
                        .results
                        .iter()
                        .filter_map(|item| match item {
                            SearchItem::Track(t) => Some(t.clone()),
                            _ => None,
                        })
                        .collect();
                    let track_index = tracks
                        .iter()
                        .position(|t| t.id == track.id && t.album_id == track.album_id)
                        .unwrap_or(0);
                    app.player.prepare_at(&tracks, track_index);
                    app.playback_source = PlaybackSource::Other;
                    start_audio_load(app, tx);
                }
                Some(SearchItem::Artist(artist)) => open_artist(app, artist, tx),
                Some(SearchItem::Album(album)) => open_album(app, album, tx),
                Some(SearchItem::Playlist(playlist)) => open_playlist(app, playlist, tx),
                None => {}
            }
        }
        Screen::TrackList => {
            let Some(list) = &app.track_list else {
                return;
            };
            app.player.prepare_at(&list.tracks, index);
            app.playback_source = PlaybackSource::Other;
            start_audio_load(app, tx);
        }
        Screen::Login | Screen::Settings | Screen::WaveSetup => {}
    }
}

fn start_audio_load(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    let Some(track) = app.player.current_track().cloned() else {
        return;
    };

    app.audio_seq += 1;
    let seq = app.audio_seq;
    let client = app.client.clone();
    let quality = app.config.audio_quality;
    let tx = tx.clone();
    app.status_message = format!("Загружаем аудио: {} — {}...", track.artist, track.title);

    tokio::spawn(async move {
        let result = client
            .track_audio(&track.id, quality)
            .await
            .map_err(|e| e.to_string());
        let _ = tx.send(ApiEvent::Audio { result, seq }).await;
    });
}

fn toggle_lyrics(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    app.show_lyrics = !app.show_lyrics;
    if app.show_lyrics {
        app.status_message = "Панель текста открыта · t — закрыть".into();
        sync_lyrics(app, tx);
    } else {
        app.status_message = "Панель текста закрыта".into();
    }
}

/// Подхватывает текст и обложку для играющего трека.
fn sync_lyrics(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    if !app.show_lyrics || app.screen == Screen::Login {
        return;
    }

    let Some(track) = app.player.current_track().cloned() else {
        if app.lyrics.track_id.is_some() || app.cover.track_id.is_some() {
            app.lyrics = LyricsState::default();
            app.cover = CoverState::default();
            app.lyrics_scroll = 0.0;
        }
        return;
    };

    if app.lyrics.track_id.as_deref() != Some(&track.id) {
        start_lyrics_load(app, &track, tx);
    }
    if app.cover.track_id.as_deref() != Some(&track.id) {
        start_cover_load(app, &track, tx);
    }
}

fn start_lyrics_load(app: &mut App, track: &Track, tx: &mpsc::Sender<ApiEvent>) {
    app.lyrics_seq += 1;
    let seq = app.lyrics_seq;
    let track_id = track.id.clone();
    let client = app.client.clone();
    let tx = tx.clone();

    app.lyrics = LyricsState {
        track_id: Some(track_id.clone()),
        loading: true,
        ..LyricsState::default()
    };
    app.lyrics_scroll = 0.0;

    tokio::spawn(async move {
        let result = client
            .track_lyrics(&track_id)
            .await
            .map_err(|e| e.to_string());
        let _ = tx
            .send(ApiEvent::Lyrics {
                result,
                track_id,
                seq,
            })
            .await;
    });
}

fn start_cover_load(app: &mut App, track: &Track, tx: &mpsc::Sender<ApiEvent>) {
    let Some(cover_uri) = track.cover_uri.clone() else {
        app.cover = CoverState {
            track_id: Some(track.id.clone()),
            ..CoverState::default()
        };
        return;
    };

    app.cover_seq += 1;
    let seq = app.cover_seq;
    let track_id = track.id.clone();
    let client = app.client.clone();
    let tx = tx.clone();

    app.cover = CoverState {
        track_id: Some(track_id.clone()),
        loading: true,
        image: None,
    };

    tokio::spawn(async move {
        let result = tokio::spawn(async move {
            let bytes = client
                .track_cover(&cover_uri)
                .await
                .map_err(|e| e.to_string())?;
            tokio::task::spawn_blocking(move || CoverImage::decode(&bytes))
                .await
                .map_err(|e| e.to_string())?
        })
        .await
        .unwrap_or_else(|e| Err(e.to_string()));
        let _ = tx
            .send(ApiEvent::Cover {
                result,
                track_id,
                seq,
            })
            .await;
    });
}

fn update_lyrics_scroll(app: &mut App, dt: Duration) {
    if !app.show_lyrics || app.lyrics_view_height == 0 {
        return;
    }
    let Some(index) = app.current_lyric_line() else {
        app.lyrics_scroll = 0.0;
        return;
    };

    let height = app.lyrics_view_height as f32;
    let anchor = (height * 0.42).floor();
    let max_scroll = (app.lyrics.lyrics.lines.len() as f32 - height).max(0.0);
    let target = ((index as f32 - anchor).max(0.0)).min(max_scroll);
    let diff = target - app.lyrics_scroll;
    if diff.abs() < 0.01 {
        app.lyrics_scroll = target;
    } else {
        let ease = (dt.as_secs_f32() * 6.0).clamp(0.12, 0.55);
        app.lyrics_scroll += diff * ease;
    }
}

fn apply_quality(app: &mut App, quality: Quality, tx: &mpsc::Sender<ApiEvent>) {
    app.config.audio_quality = quality;
    let _ = app.config.save();
    app.status_message = format!("Качество: {}", quality.label());
    if app.player.current_track().is_some() {
        start_audio_load(app, tx);
        app.status_message = format!("Качество: {} · перезагружаем трек", quality.label());
    }
}

fn cycle_quality(app: &mut App, forward: bool, tx: &mpsc::Sender<ApiEvent>) {
    let quality = if forward {
        app.config.audio_quality.next()
    } else {
        app.config.audio_quality.prev()
    };
    apply_quality(app, quality, tx);
}

fn advance_track(app: &mut App, forward: bool, tx: &mpsc::Sender<ApiEvent>) {
    if forward {
        app.player.next();
    } else {
        app.player.prev();
    }
    if app.playback_source == PlaybackSource::Wave
        && let Some(index) = app.player.index
    {
        app.wave_revealed = app.wave_revealed.max(index + 1).min(app.wave.len());
    }
    if app.player.current_track().is_some() {
        start_audio_load(app, tx);
    }
}

fn same_track(a: &Track, b: &Track) -> bool {
    a.id == b.id && a.album_id == b.album_id
}

fn current_track_list(app: &App) -> Option<&Vec<Track>> {
    match app.screen {
        Screen::Wave => Some(&app.wave),
        Screen::Library if app.library_tab == LibraryTab::Tracks => Some(&app.library_tracks),
        Screen::TrackList => app.track_list.as_ref().map(|list| &list.tracks),
        _ => None,
    }
}

fn current_track_list_mut(app: &mut App) -> Option<&mut Vec<Track>> {
    match app.screen {
        Screen::Wave => Some(&mut app.wave),
        Screen::Library if app.library_tab == LibraryTab::Tracks => Some(&mut app.library_tracks),
        Screen::TrackList => app.track_list.as_mut().map(|list| &mut list.tracks),
        _ => None,
    }
}

fn shuffle_tracks(app: &mut App) {
    let current = app.player.current_track().cloned();
    let (len, sync_queue) = match current_track_list(app) {
        Some(tracks) => {
            let sync_queue = current.is_some()
                && !app.player.queue.is_empty()
                && app
                    .player
                    .queue
                    .iter()
                    .all(|queued| tracks.iter().any(|track| same_track(track, queued)));
            (tracks.len(), sync_queue)
        }
        None => {
            app.status_message = "Перемешивание доступно в списках треков".into();
            return;
        }
    };

    if len < 2 {
        app.status_message = "Недостаточно треков для перемешивания".into();
        return;
    }

    let shuffled = {
        let Some(tracks) = current_track_list_mut(app) else {
            return;
        };
        tracks.shuffle(&mut rand::rng());
        sync_queue.then(|| tracks.clone())
    };

    if let (Some(queue), Some(current)) = (shuffled, current) {
        app.player.set_queue(queue, &current);
        if app.screen == Screen::Wave
            && let Some(index) = app.player.index
        {
            app.wave_revealed = app.wave_revealed.max(index + 1).min(app.wave.len());
        }
    }

    app.status_message = format!("Перемешано треков: {len}");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackEndAction {
    /// Заново включить тот же трек.
    RepeatTrack,
    /// Подгрузить следующую порцию «Моей волны».
    LoadMoreWave,
    /// Вернуться к первому треку альбома / очереди.
    RestartAlbum,
    /// Очередь закончилась, повтор выключен.
    StopQueue,
    /// Обычный переход к следующему треку.
    NextTrack,
}

fn track_end_action(
    repeat: RepeatMode,
    source: PlaybackSource,
    at_end: bool,
    pending_wave: bool,
    wave_loading: bool,
) -> TrackEndAction {
    if repeat == RepeatMode::One {
        return TrackEndAction::RepeatTrack;
    }

    if source == PlaybackSource::Wave && !pending_wave && at_end && !wave_loading {
        return TrackEndAction::LoadMoreWave;
    }

    if at_end && source != PlaybackSource::Wave {
        return match repeat {
            RepeatMode::All => TrackEndAction::RestartAlbum,
            RepeatMode::Off => TrackEndAction::StopQueue,
            RepeatMode::One => TrackEndAction::RepeatTrack,
        };
    }

    TrackEndAction::NextTrack
}

fn on_tick(app: &mut App, dt: Duration, tx: &mpsc::Sender<ApiEvent>) {
    let end = app.player.tick(dt);
    sync_lyrics(app, tx);
    update_lyrics_scroll(app, dt);
    if end != TrackEnd::Finished || app.player.current_track().is_none() {
        return;
    }

    let at_end = app
        .player
        .index
        .map(|i| i + 1 >= app.player.queue.len())
        .unwrap_or(false);

    match track_end_action(
        app.player.repeat,
        app.playback_source,
        at_end,
        app.pending_wave,
        app.wave_loading,
    ) {
        TrackEndAction::RepeatTrack => {
            let label = app.player.now_playing();
            app.player.replay_current();
            start_audio_load(app, tx);
            if !label.is_empty() {
                app.status_message = format!("Повтор трека: {label}");
            }
        }
        TrackEndAction::LoadMoreWave => {
            app.pending_wave = true;
            start_wave_load(app, tx, true);
        }
        TrackEndAction::RestartAlbum => {
            app.player.play_index(0);
            start_audio_load(app, tx);
            app.status_message = "Повтор альбома: играем с начала".into();
        }
        TrackEndAction::StopQueue => {
            app.status_message = "Очередь завершена · Shift+R — включить повтор".into();
        }
        TrackEndAction::NextTrack => advance_track(app, true, tx),
    }
}

fn handle_mpris_command(app: &mut App, command: MprisCommand, tx: &mpsc::Sender<ApiEvent>) {
    match command {
        MprisCommand::Play => play_from_mpris(app, tx),
        MprisCommand::Pause => {
            if app.player.engine.state == PlayState::Playing {
                app.player.toggle_pause();
            }
        }
        MprisCommand::PlayPause => match app.player.engine.state {
            PlayState::Playing | PlayState::Paused => app.player.toggle_pause(),
            PlayState::Stopped | PlayState::Loading => play_from_mpris(app, tx),
        },
        MprisCommand::Stop => {
            app.player.stop();
            app.status_message = "Воспроизведение остановлено".into();
        }
        MprisCommand::Next => advance_track(app, true, tx),
        MprisCommand::Previous => advance_track(app, false, tx),
        MprisCommand::SeekBy(secs) => {
            if matches!(
                app.player.engine.state,
                PlayState::Playing | PlayState::Paused
            ) {
                app.player.seek_by(secs.round() as i64);
            }
        }
        MprisCommand::SetPosition(secs) => {
            if matches!(
                app.player.engine.state,
                PlayState::Playing | PlayState::Paused
            ) {
                let delta = (secs - app.player.engine.position).round() as i64;
                app.player.seek_by(delta);
            }
        }
        MprisCommand::SetVolume(volume) => {
            app.player.engine.set_volume(volume);
            app.config.volume = app.player.engine.volume;
        }
        MprisCommand::SetRepeat(mode) => {
            app.player.repeat = mode;
            app.config.repeat = mode;
            let _ = app.config.save();
            app.status_message = format!("Повтор: {}", mode.description());
        }
        MprisCommand::Raise => {}
        MprisCommand::Quit => app.should_exit = true,
    }
}

fn play_from_mpris(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    match app.player.engine.state {
        PlayState::Playing | PlayState::Loading => {}
        PlayState::Paused => app.player.toggle_pause(),
        PlayState::Stopped => {
            if app.player.current_track().is_some() {
                start_audio_load(app, tx);
            } else {
                play_current(app, tx);
            }
        }
    }
}

fn mpris_snapshot(app: &App) -> MprisSnapshot {
    let track = app.player.current_track().map(|track| MprisTrack {
        id: track.id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        duration_secs: track.duration,
    });

    MprisSnapshot {
        state: app.player.engine.state,
        track,
        position_secs: app.player.engine.position,
        volume: app.player.engine.volume,
        repeat: app.player.repeat,
        can_go_next: !app.player.queue.is_empty(),
        can_go_previous: !app.player.queue.is_empty(),
    }
}

fn handle_paste(app: &mut App, text: &str) {
    if app.screen == Screen::Login && app.login_token_editing {
        app.login_token_input
            .push_str(text.trim().trim_matches('\n'));
    } else if app.screen == Screen::Search && app.search.editing {
        app.search
            .input
            .push_str(text.trim_end_matches(['\r', '\n']));
    } else if app.screen == Screen::Login {
        app.login_token_input = text.trim().to_string();
        app.login_token_editing = !app.login_token_input.is_empty();
        app.status_message = "Токен вставлен. Enter — проверить и войти".into();
    }
}

fn cycle_repeat(app: &mut App, forward: bool) {
    let mode = if forward {
        app.player.repeat.next()
    } else {
        app.player.repeat.prev()
    };
    app.player.repeat = mode;
    app.config.repeat = mode;
    let _ = app.config.save();
    app.status_message = format!("Повтор: {}", mode.description());
}

fn toggle_setting(app: &mut App, tx: &mpsc::Sender<ApiEvent>) {
    match app.selected() {
        Some(0) => {
            app.config.theme = app.config.theme.next();
            let _ = app.config.save();
            app.status_message = format!("Тема интерфейса: {}", app.config.theme.label());
        }
        Some(1) => cycle_quality(app, true, tx),
        Some(2) => cycle_repeat(app, true),
        Some(3) => open_wave_setup(app, tx),
        _ => {}
    }
}

fn cycle_setting(app: &mut App, forward: bool, tx: &mpsc::Sender<ApiEvent>) {
    match app.selected() {
        Some(0) => {
            app.config.theme = if forward {
                app.config.theme.next()
            } else {
                app.config.theme.prev()
            };
            let _ = app.config.save();
            app.status_message = format!("Тема интерфейса: {}", app.config.theme.label());
        }
        Some(1) => cycle_quality(app, forward, tx),
        Some(2) => cycle_repeat(app, forward),
        _ => {}
    }
}

fn handle_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    tx: &mpsc::Sender<ApiEvent>,
) -> AppAction {
    use crossterm::event::KeyEventKind;
    use crossterm::event::{KeyCode, KeyModifiers};

    if key.kind != KeyEventKind::Press {
        return AppAction::None;
    }

    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.should_exit = true;
        return AppAction::None;
    }

    if let Some(action) = app.confirm {
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.confirm = None;
                match action {
                    ConfirmAction::Quit => app.should_exit = true,
                    ConfirmAction::Logout => return AppAction::Logout,
                }
            }
            KeyCode::Esc
            | KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Char('q')
            | KeyCode::Char('Q') => {
                app.confirm = None;
                app.status_message = "Действие отменено".into();
            }
            _ => {}
        }
        return AppAction::None;
    }

    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => {
                app.show_help = false;
                app.help_scroll = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.help_scroll = app.help_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.help_scroll = app.help_scroll.saturating_add(1);
            }
            KeyCode::PageUp => app.help_scroll = app.help_scroll.saturating_sub(10),
            KeyCode::PageDown => app.help_scroll = app.help_scroll.saturating_add(10),
            KeyCode::Home => app.help_scroll = 0,
            _ => {}
        }
        return AppAction::None;
    }

    if app.screen == Screen::Login && app.login_token_editing {
        match key.code {
            KeyCode::Enter => match crate::auth::parse_token_input(&app.login_token_input) {
                Some(token) => {
                    app.login_token_editing = false;
                    app.config.oauth_token = Some(token.clone());
                    app.client = YandexClient::new(Some(token));
                    app.status_message = "Проверка OAuth-токена...".into();
                    spawn_account(tx.clone(), app.client.clone(), true);
                }
                None => {
                    app.status_message = "Это не redirect-ссылка. Скопируйте адрес кнопки «Открыть приложение» (ПКМ) или вставьте access_token".into();
                }
            },
            KeyCode::Esc => {
                app.login_token_editing = false;
                app.status_message = "Ввод токена отменён".into();
            }
            KeyCode::Backspace => {
                app.login_token_input.pop();
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                app.login_token_input.push(c);
            }
            _ => {}
        }
        return AppAction::None;
    }

    if app.screen == Screen::Search && app.search.editing {
        match key.code {
            KeyCode::Enter => {
                app.search.query = app.search.input.trim().to_string();
                app.search.editing = false;
                run_search(app, tx);
            }
            KeyCode::Esc => {
                app.search.editing = false;
                app.status_message = "Ввод поиска завершён".into();
            }
            KeyCode::Backspace => {
                app.search.input.pop();
            }
            KeyCode::Tab => {
                app.search.filter = app.search.filter.next();
                app.status_message = format!("Фильтр поиска: {}", app.search.filter.label());
            }
            KeyCode::BackTab => {
                app.search.filter = app.search.filter.prev();
                app.status_message = format!("Фильтр поиска: {}", app.search.filter.label());
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                app.search.input.push(c);
            }
            _ => {}
        }
        return AppAction::None;
    }

    if key.code == KeyCode::Char('?') {
        app.show_help = true;
        app.help_scroll = 0;
        return AppAction::None;
    }

    if app.screen == Screen::Login {
        match key.code {
            KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                adjust_volume(app, VOLUME_STEP as i16)
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                adjust_volume(app, -(VOLUME_STEP as i16))
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => adjust_volume(app, 1),
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => adjust_volume(app, -1),
            KeyCode::Enter => return AppAction::TriggerLogin,
            KeyCode::Char('l') | KeyCode::Char('L') => return AppAction::TriggerLogin,
            KeyCode::Char('t') | KeyCode::Char('T') => {
                app.login_token_editing = true;
                app.login_token_input.clear();
                app.status_message = "Вставьте OAuth-токен (Ctrl+Shift+V) и нажмите Enter".into();
            }
            KeyCode::Esc => {
                app.screen = Screen::Search;
                app.search.editing = true;
                app.search.input = app.search.query.clone();
                app.reset_selection();
                app.status_message =
                    "Поиск доступен без входа. Библиотека и волна требуют входа".into();
            }
            KeyCode::Char('4') | KeyCode::Char(',') => app.open_settings(),
            KeyCode::Char('q') | KeyCode::Char('Q') => app.confirm = Some(ConfirmAction::Quit),
            _ => {}
        }
        return AppAction::None;
    }

    if app.screen == Screen::Settings {
        match key.code {
            KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                adjust_volume(app, VOLUME_STEP as i16)
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                adjust_volume(app, -(VOLUME_STEP as i16))
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => adjust_volume(app, 1),
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => adjust_volume(app, -1),
            KeyCode::Esc => {
                app.screen = app.settings_back;
                app.reset_selection();
            }
            KeyCode::Enter | KeyCode::Char(' ') => toggle_setting(app, tx),
            KeyCode::Left => cycle_setting(app, false, tx),
            KeyCode::Right => cycle_setting(app, true, tx),
            KeyCode::Up => move_selection(app, -1),
            KeyCode::Down => move_selection(app, 1),
            KeyCode::Char('w') | KeyCode::Char('W') => open_wave_setup(app, tx),
            KeyCode::Char('q') | KeyCode::Char('Q') => app.confirm = Some(ConfirmAction::Quit),
            _ => {}
        }
        return AppAction::None;
    }

    if app.screen == Screen::WaveSetup {
        match key.code {
            KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                adjust_volume(app, VOLUME_STEP as i16)
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                adjust_volume(app, -(VOLUME_STEP as i16))
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => adjust_volume(app, 1),
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => adjust_volume(app, -1),
            KeyCode::Esc => {
                let dirty = app.wave_prefs_dirty;
                app.wave_prefs_dirty = false;
                app.screen = Screen::Wave;
                app.reset_selection();
                if dirty {
                    restart_wave(app, tx);
                }
            }
            KeyCode::Enter => match (app.selected(), app.wave_setup_actions()) {
                (Some(selected), Some((apply, _))) if selected == apply => {
                    app.wave_prefs_dirty = false;
                    app.screen = Screen::Wave;
                    app.reset_selection();
                    restart_wave(app, tx);
                }
                (Some(selected), Some((_, reset))) if selected == reset => {
                    reset_wave_setup(app, tx);
                }
                _ => cycle_wave_setup(app, true),
            },
            KeyCode::Char('a') | KeyCode::Char('A') => {
                app.wave_prefs_dirty = false;
                app.screen = Screen::Wave;
                app.reset_selection();
                restart_wave(app, tx);
            }
            KeyCode::Left => cycle_wave_setup(app, false),
            KeyCode::Right => cycle_wave_setup(app, true),
            KeyCode::Up => move_selection(app, -1),
            KeyCode::Down => move_selection(app, 1),
            KeyCode::Char('w') | KeyCode::Char('W') => {
                app.screen = Screen::Wave;
                app.reset_selection();
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => app.confirm = Some(ConfirmAction::Quit),
            _ => {}
        }
        return AppAction::None;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) => {
            app.confirm = Some(ConfirmAction::Quit);
            return AppAction::None;
        }
        (KeyCode::Char('1'), _) => {
            app.discard_track_list();
            if app.has_token() {
                app.screen = Screen::Wave;
                app.reset_selection();
                if app.wave.is_empty() {
                    start_wave_load(app, tx, false);
                }
            } else {
                app.screen = Screen::Login;
                app.reset_selection();
                app.status_message = "Для Моей волны нужен вход".into();
            }
            return AppAction::None;
        }
        (KeyCode::Char('2'), _) => {
            app.discard_track_list();
            if app.ensure_login() {
                app.screen = Screen::Library;
                app.reset_selection();
                if !app.library_loaded[app.library_tab.index()] {
                    load_library(app, tx);
                }
            }
            return AppAction::None;
        }
        (KeyCode::Char('3') | KeyCode::Char('/'), _) => {
            let fresh = app.screen == Screen::Search;
            app.discard_track_list();
            app.screen = Screen::Search;
            app.search.editing = true;
            app.search.input = if fresh {
                String::new()
            } else {
                app.search.query.clone()
            };
            app.reset_selection();
            app.status_message =
                "Поиск: введите запрос, Enter — искать, Esc — к результатам".into();
            return AppAction::None;
        }
        (KeyCode::Char('4') | KeyCode::Char(','), _) => {
            app.open_settings();
            return AppAction::None;
        }
        (KeyCode::Char('w') | KeyCode::Char('W'), _) => {
            open_wave_setup(app, tx);
            return AppAction::None;
        }
        (KeyCode::Char('l') | KeyCode::Char('L'), _) => return AppAction::TriggerLogin,
        (KeyCode::Char('o') | KeyCode::Char('O'), _) => {
            app.confirm = Some(ConfirmAction::Logout);
            return AppAction::None;
        }
        (KeyCode::Char('R'), _) => {
            cycle_repeat(app, true);
            return AppAction::None;
        }
        (KeyCode::Char('t') | KeyCode::Char('T'), _) => {
            toggle_lyrics(app, tx);
            return AppAction::None;
        }
        (KeyCode::Char('f') | KeyCode::Char('F'), _) => toggle_like(app, tx),
        (KeyCode::Char('d') | KeyCode::Char('D'), _) => toggle_dislike(app, tx),
        _ => {}
    }

    match key.code {
        KeyCode::Esc => {
            match app.screen {
                Screen::TrackList => {
                    let back = app
                        .track_list
                        .as_ref()
                        .map(|list| list.back)
                        .unwrap_or(Screen::Library);
                    app.tracklist_seq += 1;
                    app.track_list = None;
                    app.screen = back;
                }
                Screen::Search => {
                    if app.search.editing {
                        app.search.editing = false;
                    } else {
                        app.screen = Screen::Wave;
                    }
                }
                Screen::Library => app.screen = Screen::Wave,
                _ => {}
            }
            app.reset_selection();
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
            adjust_volume(app, VOLUME_STEP as i16)
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
            adjust_volume(app, -(VOLUME_STEP as i16))
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => adjust_volume(app, 1),
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => adjust_volume(app, -1),
        KeyCode::Up => move_selection(app, -1),
        KeyCode::Down => move_selection(app, 1),
        KeyCode::PageUp => move_selection(app, -10),
        KeyCode::PageDown => move_selection(app, 10),
        KeyCode::Home => app.list_state.select(Some(0)),
        KeyCode::End => {
            let len = app.row_count();
            if len > 0 {
                app.list_state.select(Some(len - 1));
            }
        }
        KeyCode::Tab => {
            if app.screen == Screen::Library {
                app.library_tab = app.library_tab.next();
                app.reset_selection();
                if !app.library_loaded[app.library_tab.index()] {
                    load_library(app, tx);
                }
            } else if app.screen == Screen::Search {
                app.search.filter = app.search.filter.next();
                if !app.search.query.is_empty() {
                    run_search(app, tx);
                }
            }
        }
        KeyCode::BackTab => {
            if app.screen == Screen::Library {
                app.library_tab = app.library_tab.prev();
                app.reset_selection();
                if !app.library_loaded[app.library_tab.index()] {
                    load_library(app, tx);
                }
            } else if app.screen == Screen::Search {
                app.search.filter = app.search.filter.prev();
                if !app.search.query.is_empty() {
                    run_search(app, tx);
                }
            }
        }
        KeyCode::Enter => {
            if app.screen == Screen::Library {
                let selected = app.selected();
                match (app.library_tab, selected) {
                    (LibraryTab::Artists, Some(index)) => {
                        if let Some(artist) = app.library_artists.get(index).cloned() {
                            open_artist(app, artist, tx);
                        }
                    }
                    (LibraryTab::Playlists, Some(index)) => {
                        if let Some(playlist) = app.library_playlists.get(index).cloned() {
                            open_playlist(app, playlist, tx);
                        }
                    }
                    _ => play_current(app, tx),
                }
            } else {
                play_current(app, tx);
            }
        }
        KeyCode::Char(' ') => {
            if app.player.engine.state == PlayState::Stopped {
                // После окончания очереди пробел запускает текущий трек заново.
                play_from_mpris(app, tx);
            } else {
                app.player.toggle_pause();
            }
        }
        KeyCode::Left => app.player.seek_by(-SEEK_STEP),
        KeyCode::Right => app.player.seek_by(SEEK_STEP),
        KeyCode::Char('n') => advance_track(app, true, tx),
        KeyCode::Char('p') => advance_track(app, false, tx),
        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char(']') => {
            adjust_volume(app, VOLUME_STEP as i16)
        }
        KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Char('[') => {
            adjust_volume(app, -(VOLUME_STEP as i16))
        }
        KeyCode::Char('m') => toggle_mute(app),
        KeyCode::Char('~') => cycle_quality(app, true, tx),
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.config.volume = app.player.engine.volume;
            match app.config.save() {
                Ok(()) => {
                    app.status_message = format!("Конфиг сохранён: {}", Config::path().display())
                }
                Err(e) => app.status_message = format!("Ошибка сохранения: {e}"),
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => shuffle_tracks(app),
        KeyCode::Char('r') => match app.screen {
            Screen::Wave => start_wave_load(app, tx, false),
            Screen::Library => {
                app.library_loaded[app.library_tab.index()] = false;
                load_library(app, tx);
            }
            Screen::Search => run_search(app, tx),
            Screen::TrackList => reload_track_list(app, tx),
            _ => app.status_message = "Обновление недоступно на этом экране".into(),
        },
        _ => {}
    }

    AppAction::None
}

fn adjust_volume(app: &mut App, delta: i16) {
    let volume = (app.player.engine.volume as i16 + delta).clamp(0, 100) as u8;
    app.player.engine.set_volume(volume);
    app.config.volume = volume;
    if volume > 0 {
        app.volume_before_mute = volume;
    }
    app.status_message = format!("Громкость: {volume}%");
}

fn toggle_mute(app: &mut App) {
    if app.player.engine.volume == 0 {
        let volume = app.volume_before_mute.max(1);
        app.player.engine.set_volume(volume);
        app.config.volume = volume;
        app.status_message = format!("Звук включён: {volume}%");
    } else {
        app.volume_before_mute = app.player.engine.volume;
        app.player.engine.set_volume(0);
        app.config.volume = 0;
        app.status_message = "Звук выключен (m — включить)".into();
    }
}

fn move_selection(app: &mut App, delta: i64) {
    let len = app.row_count();
    if len == 0 {
        app.list_state.select(None);
        return;
    }
    let cur = app.list_state.selected().unwrap_or(0) as i64;
    let next = (cur + delta).clamp(0, len as i64 - 1);
    app.list_state.select(Some(next as usize));
}

impl crate::player::Player {
    pub fn now_playing(&self) -> String {
        match self.current_track() {
            Some(t) => format!("{} — {}", t.artist, t.title),
            None => self.engine.now_playing.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_one_restarts_any_source() {
        assert_eq!(
            track_end_action(RepeatMode::One, PlaybackSource::Other, false, false, false),
            TrackEndAction::RepeatTrack
        );
        assert_eq!(
            track_end_action(RepeatMode::One, PlaybackSource::Wave, true, false, false),
            TrackEndAction::RepeatTrack
        );
    }

    #[test]
    fn album_end_stops_or_restarts() {
        assert_eq!(
            track_end_action(RepeatMode::Off, PlaybackSource::Other, true, false, false),
            TrackEndAction::StopQueue
        );
        assert_eq!(
            track_end_action(RepeatMode::All, PlaybackSource::Other, true, false, false),
            TrackEndAction::RestartAlbum
        );
        assert_eq!(
            track_end_action(RepeatMode::Off, PlaybackSource::Other, false, false, false),
            TrackEndAction::NextTrack
        );
        assert_eq!(
            track_end_action(RepeatMode::All, PlaybackSource::Other, false, false, false),
            TrackEndAction::NextTrack
        );
    }

    #[test]
    fn wave_end_loads_more() {
        assert_eq!(
            track_end_action(RepeatMode::Off, PlaybackSource::Wave, true, false, false),
            TrackEndAction::LoadMoreWave
        );
        assert_eq!(
            track_end_action(RepeatMode::All, PlaybackSource::Wave, true, false, false),
            TrackEndAction::LoadMoreWave
        );
        assert_eq!(
            track_end_action(RepeatMode::All, PlaybackSource::Wave, false, false, false),
            TrackEndAction::NextTrack
        );
        // Уже идёт подгрузка — повторный запрос не отправляем.
        assert_eq!(
            track_end_action(RepeatMode::Off, PlaybackSource::Wave, true, true, false),
            TrackEndAction::NextTrack
        );
        assert_eq!(
            track_end_action(RepeatMode::Off, PlaybackSource::Wave, true, false, true),
            TrackEndAction::NextTrack
        );
    }
}
