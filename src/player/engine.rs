use rodio::{Decoder, DeviceSinkBuilder, Player as RodioPlayer};
use std::fmt;
use std::io::Cursor;
use std::time::Duration;

/// Состояние воспроизведения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Stopped,
    Loading,
    Playing,
    Paused,
}

/// Реальный аудиодвижок на rodio/symphonia.
pub struct AudioEngine {
    pub state: PlayState,
    /// Позиция текущего трека в секундах
    pub position: f64,
    /// Длительность трека в секундах
    pub duration: f64,
    /// 0..=100
    pub volume: u8,
    /// Подпись для UI (артист — трек)
    pub now_playing: String,
    player: Option<RodioPlayer>,
    sink: Option<rodio::MixerDeviceSink>,
}

impl fmt::Debug for AudioEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioEngine")
            .field("state", &self.state)
            .field("position", &self.position)
            .field("duration", &self.duration)
            .field("volume", &self.volume)
            .field("now_playing", &self.now_playing)
            .finish_non_exhaustive()
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            state: PlayState::Stopped,
            position: 0.0,
            duration: 0.0,
            volume: 70,
            now_playing: String::new(),
            player: None,
            sink: None,
        }
    }

    /// Подготовить движок к загрузке трека (звук ещё не получен).
    pub fn prepare(&mut self, duration_sec: u64, label: impl Into<String>) {
        if let Some(player) = &self.player {
            player.stop();
        }
        self.state = PlayState::Loading;
        self.position = 0.0;
        self.duration = duration_sec as f64;
        self.now_playing = label.into();
    }

    fn ensure_output(&mut self) -> Result<(), String> {
        if self.player.is_some() {
            return Ok(());
        }

        let mut sink = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| format!("не удалось открыть аудиовыход: {e}"))?;
        sink.log_on_drop(false);
        let player = RodioPlayer::connect_new(sink.mixer());
        player.set_volume(self.volume as f32 / 100.0);
        self.player = Some(player);
        self.sink = Some(sink);
        Ok(())
    }

    /// Запустить воспроизведение уже скачанного MP3/FLAC и т.п.
    pub fn play_bytes(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.ensure_output()?;

        let byte_len = bytes.len() as u64;
        let decoder = Decoder::builder()
            .with_data(Cursor::new(bytes))
            .with_byte_len(byte_len)
            .with_seekable(true)
            .build()
            .map_err(|e| format!("не удалось декодировать аудио: {e}"))?;

        let player = self
            .player
            .as_ref()
            .ok_or_else(|| "аудиовыход не инициализирован".to_string())?;
        player.stop();
        player.set_volume(self.volume as f32 / 100.0);
        player.append(decoder);
        player.play();

        self.state = PlayState::Playing;
        self.position = 0.0;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(player) = &self.player {
            player.stop();
        }
        self.state = PlayState::Stopped;
        self.position = 0.0;
        self.now_playing.clear();
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            PlayState::Playing => {
                if let Some(player) = &self.player {
                    player.pause();
                }
                self.state = PlayState::Paused;
            }
            PlayState::Paused => {
                if let Some(player) = &self.player {
                    player.play();
                }
                self.state = PlayState::Playing;
            }
            PlayState::Stopped | PlayState::Loading => {}
        }
    }

    pub fn seek_by(&mut self, secs: i64) {
        if !matches!(self.state, PlayState::Playing | PlayState::Paused) {
            return;
        }
        let target = (self.position + secs as f64).clamp(0.0, self.duration);
        if let Some(player) = &self.player {
            let _ = player.try_seek(Duration::from_secs_f64(target));
        }
        self.position = target;
    }

    pub fn set_volume(&mut self, v: u8) {
        self.volume = v.clamp(0, 100);
        if let Some(player) = &self.player {
            player.set_volume(self.volume as f32 / 100.0);
        }
    }

    /// Обновить позицию и определить конец трека.
    pub fn tick(&mut self, _dt: Duration) {
        let Some(player) = &self.player else {
            return;
        };

        if !matches!(self.state, PlayState::Playing | PlayState::Paused) {
            return;
        }

        self.position = player.get_pos().as_secs_f64();
        if self.state == PlayState::Playing && player.empty() {
            self.state = PlayState::Stopped;
            if self.duration > 0.0 {
                self.position = self.duration;
            }
        }
    }
}
