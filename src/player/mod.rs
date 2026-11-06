pub mod engine;

use engine::PlayState;
use std::time::Duration;

#[derive(Debug, Default)]
pub struct Player {
    pub queue: Vec<crate::api::models::Track>,

    pub index: Option<usize>,
    pub engine: engine::AudioEngine,
}

impl Player {
    pub fn prepare_at(&mut self, queue: &[crate::api::models::Track], index: usize) {
        if index >= queue.len() {
            return;
        }
        self.queue = queue.to_vec();
        self.index = Some(index);
        let track = &self.queue[index];
        self.engine.prepare(
            track.duration,
            format!("{} — {}", track.artist, track.title),
        );
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
        self.engine.play_bytes(bytes)
    }

    pub fn toggle_pause(&mut self) {
        self.engine.toggle_pause();
    }

    pub fn stop(&mut self) {
        self.index = None;
        self.queue.clear();
        self.engine.stop();
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
        self.index = Some(idx);
        let track = &self.queue[idx];
        self.engine.prepare(
            track.duration,
            format!("{} — {}", track.artist, track.title),
        );
    }

    pub fn tick(&mut self, dt: Duration) -> TrackEnd {
        self.engine.tick(dt);
        match self.engine.state {
            PlayState::Stopped => TrackEnd::Finished,
            _ => TrackEnd::None,
        }
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
