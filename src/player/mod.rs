pub mod engine;

use engine::PlayState;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Что делать, когда трек заканчивается.
///
/// * `Off` — играть очередь до конца и остановиться;
/// * `All` — зациклить альбом / плейлист / текущую очередь;
/// * `One` — бесконечно повторять один текущий трек.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

impl RepeatMode {
    /// Короткая подпись для плеера и настроек.
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "выкл",
            Self::All => "альбом",
            Self::One => "трек",
        }
    }

    /// Понятное описание для строки состояния.
    pub fn description(self) -> &'static str {
        match self {
            Self::Off => "выключен",
            Self::All => "альбом / очередь",
            Self::One => "текущий трек",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Off => Self::One,
            Self::All => Self::Off,
            Self::One => Self::All,
        }
    }
}

#[derive(Debug, Default)]
pub struct Player {
    pub queue: Vec<crate::api::models::Track>,

    pub index: Option<usize>,
    pub engine: engine::AudioEngine,
    pub repeat: RepeatMode,

    /// Защита от повторной обработки естественного окончания трека:
    /// `on_tick` получает `TrackEnd::Finished` ровно один раз.
    finished: bool,
}

impl Player {
    pub fn prepare_at(&mut self, queue: &[crate::api::models::Track], index: usize) {
        if index >= queue.len() {
            return;
        }
        self.queue = queue.to_vec();
        self.play_index(index);
    }

    /// Запустить трек из уже загруженной очереди.
    pub fn play_index(&mut self, index: usize) {
        let Some(track) = self.queue.get(index) else {
            return;
        };
        self.index = Some(index);
        self.engine.prepare(
            track.duration,
            format!("{} — {}", track.artist, track.title),
        );
        self.finished = false;
    }

    /// Перезапустить текущий трек (используется режимом повтора «трек»).
    pub fn replay_current(&mut self) {
        if let Some(index) = self.index {
            self.play_index(index);
        }
    }

    pub fn set_queue(
        &mut self,
        queue: Vec<crate::api::models::Track>,
        current: &crate::api::models::Track,
    ) {
        self.index = queue
            .iter()
            .position(|t| t.id == current.id && t.album_id == current.album_id)
            .or_else(|| (!queue.is_empty()).then_some(0));
        self.queue = queue;
    }

    pub fn play_bytes(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.engine.play_bytes(bytes)?;
        self.finished = false;
        Ok(())
    }

    pub fn toggle_pause(&mut self) {
        self.engine.toggle_pause();
    }

    pub fn stop(&mut self) {
        self.index = None;
        self.queue.clear();
        self.engine.stop();
        self.finished = false;
    }

    pub fn next(&mut self) {
        self.advance(1);
    }

    pub fn prev(&mut self) {
        self.advance(-1);
    }

    fn advance(&mut self, step: i64) {
        if self.queue.is_empty() {
            return;
        }
        let len = self.queue.len() as i64;
        let cur = self.index.map(|i| i as i64).unwrap_or(0);
        let idx = ((cur + step).rem_euclid(len)) as usize;
        self.play_index(idx);
    }

    pub fn tick(&mut self, dt: Duration) -> TrackEnd {
        self.engine.tick(dt);
        if self.engine.state == PlayState::Stopped && !self.finished {
            self.finished = true;
            return TrackEnd::Finished;
        }
        TrackEnd::None
    }

    pub fn current_track(&self) -> Option<&crate::api::models::Track> {
        self.index.and_then(|i| self.queue.get(i))
    }

    pub fn seek_by(&mut self, secs: i64) {
        self.engine.seek_by(secs);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackEnd {
    None,
    Finished,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::Track;

    fn track(id: &str, duration: u64) -> Track {
        Track {
            id: id.to_string(),
            album_id: Some("album".into()),
            title: format!("Трек {id}"),
            artist: "Артист".into(),
            duration,
            available: true,
            cover_uri: None,
        }
    }

    #[test]
    fn repeat_mode_cycles_in_both_directions() {
        assert_eq!(RepeatMode::Off.next(), RepeatMode::All);
        assert_eq!(RepeatMode::All.next(), RepeatMode::One);
        assert_eq!(RepeatMode::One.next(), RepeatMode::Off);

        assert_eq!(RepeatMode::Off.prev(), RepeatMode::One);
        assert_eq!(RepeatMode::One.prev(), RepeatMode::All);
        assert_eq!(RepeatMode::All.prev(), RepeatMode::Off);
    }

    #[test]
    fn finished_is_reported_once() {
        let mut player = Player::default();
        assert_eq!(player.tick(Duration::from_millis(250)), TrackEnd::Finished);
        assert_eq!(player.tick(Duration::from_millis(250)), TrackEnd::None);
    }

    #[test]
    fn prepare_and_replay_reset_finished() {
        let mut player = Player::default();
        let queue = vec![track("1", 10), track("2", 20)];
        assert_eq!(player.tick(Duration::ZERO), TrackEnd::Finished);

        player.prepare_at(&queue, 1);
        assert_eq!(player.index, Some(1));
        assert_eq!(player.engine.state, PlayState::Loading);
        assert_eq!(player.tick(Duration::ZERO), TrackEnd::None);

        assert_eq!(player.current_track().map(|t| t.id.as_str()), Some("2"));
        player.replay_current();
        assert_eq!(player.index, Some(1));
        assert_eq!(player.engine.state, PlayState::Loading);
    }

    #[test]
    fn manual_navigation_wraps_queue() {
        let mut player = Player::default();
        player.prepare_at(&[track("1", 10), track("2", 20)], 1);

        player.next();
        assert_eq!(player.index, Some(0));
        player.prev();
        assert_eq!(player.index, Some(1));
        player.prev();
        assert_eq!(player.index, Some(0));
    }
}
