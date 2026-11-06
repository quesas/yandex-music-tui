use crate::player::engine::PlayState;
use mpris_server::{Metadata, PlaybackStatus, Player, Time, TrackId};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MprisCommand {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    SeekBy(f64),
    SetPosition(f64),
    SetVolume(u8),
    Raise,
    Quit,
}

#[derive(Debug, Clone)]
pub struct MprisTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration_secs: u64,
}

#[derive(Debug, Clone)]
pub struct MprisSnapshot {
    pub state: PlayState,
    pub track: Option<MprisTrack>,
    pub position_secs: f64,
    pub volume: u8,
    pub can_go_next: bool,
    pub can_go_previous: bool,
}

pub fn spawn(command_tx: UnboundedSender<MprisCommand>) -> UnboundedSender<MprisSnapshot> {
    let (snapshot_tx, snapshot_rx) = unbounded_channel();
    let _ = std::thread::Builder::new()
        .name("mpris".into())
        .spawn(move || run(command_tx, snapshot_rx));
    snapshot_tx
}

fn run(
    command_tx: UnboundedSender<MprisCommand>,
    mut snapshot_rx: UnboundedReceiver<MprisSnapshot>,
) {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };

    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async move {
        let Ok(player) = Player::builder("yandex-music-tui")
            .identity("Яндекс Музыка TUI")
            .desktop_entry("yandex-music-tui")
            .can_quit(true)
            .can_raise(true)
            .can_play(true)
            .can_pause(true)
            .can_go_next(true)
            .can_go_previous(true)
            .can_seek(true)
            .build()
            .await
        else {
            return;
        };

        connect(&player, command_tx);
        tokio::task::spawn_local(player.run());

        while let Some(snapshot) = snapshot_rx.recv().await {
            apply(&player, snapshot).await;
        }
    });
}

fn connect(player: &Player, tx: UnboundedSender<MprisCommand>) {
    let sender = tx.clone();
    player.connect_play(move |_| {
        let _ = sender.send(MprisCommand::Play);
    });

    let sender = tx.clone();
    player.connect_pause(move |_| {
        let _ = sender.send(MprisCommand::Pause);
    });

    let sender = tx.clone();
    player.connect_play_pause(move |_| {
        let _ = sender.send(MprisCommand::PlayPause);
    });

    let sender = tx.clone();
    player.connect_stop(move |_| {
        let _ = sender.send(MprisCommand::Stop);
    });

    let sender = tx.clone();
    player.connect_next(move |_| {
        let _ = sender.send(MprisCommand::Next);
    });

    let sender = tx.clone();
    player.connect_previous(move |_| {
        let _ = sender.send(MprisCommand::Previous);
    });

    let sender = tx.clone();
    player.connect_seek(move |_, offset| {
        let _ = sender.send(MprisCommand::SeekBy(
            offset.as_micros() as f64 / 1_000_000.0,
        ));
    });

    let sender = tx.clone();
    player.connect_set_position(move |_, _, position| {
        let _ = sender.send(MprisCommand::SetPosition(
            position.as_micros() as f64 / 1_000_000.0,
        ));
    });

    let sender = tx.clone();
    player.connect_set_volume(move |_, volume| {
        let _ = sender.send(MprisCommand::SetVolume(
            (volume.clamp(0.0, 1.0) * 100.0).round() as u8,
        ));
    });

    let sender = tx.clone();
    player.connect_raise(move |_| {
        let _ = sender.send(MprisCommand::Raise);
    });

    player.connect_quit(move |_| {
        let _ = tx.send(MprisCommand::Quit);
    });
}

async fn apply(player: &Player, snapshot: MprisSnapshot) {
    let status = match snapshot.state {
        PlayState::Stopped => PlaybackStatus::Stopped,
        PlayState::Loading | PlayState::Playing => PlaybackStatus::Playing,
        PlayState::Paused => PlaybackStatus::Paused,
    };

    let _ = player.set_playback_status(status).await;
    let _ = player.set_metadata(metadata(&snapshot.track)).await;
    player.set_position(Time::from_micros(seconds_to_micros(snapshot.position_secs)));
    let _ = player
        .set_volume(snapshot.volume.min(100) as f64 / 100.0)
        .await;
    let _ = player.set_can_play(snapshot.track.is_some()).await;
    let _ = player.set_can_pause(snapshot.track.is_some()).await;
    let _ = player
        .set_can_seek(
            snapshot
                .track
                .as_ref()
                .is_some_and(|track| track.duration_secs > 0),
        )
        .await;
    let _ = player.set_can_go_next(snapshot.can_go_next).await;
    let _ = player.set_can_go_previous(snapshot.can_go_previous).await;
}

fn metadata(track: &Option<MprisTrack>) -> Metadata {
    let Some(track) = track else {
        return Metadata::builder().trackid(TrackId::NO_TRACK).build();
    };

    let track_id = TrackId::try_from(track_path(&track.id)).unwrap_or(TrackId::NO_TRACK);
    let artists: Vec<String> = track.artist.split(", ").map(str::to_string).collect();

    Metadata::builder()
        .trackid(track_id)
        .title(track.title.clone())
        .artist(artists)
        .length(Time::from_secs(track.duration_secs as i64))
        .build()
}

fn track_path(id: &str) -> String {
    let safe: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    let safe = if safe.is_empty() {
        "unknown".to_string()
    } else {
        safe
    };
    format!("/yandex/music/track/{safe}")
}

fn seconds_to_micros(secs: f64) -> i64 {
    if secs.is_finite() {
        (secs.max(0.0) * 1_000_000.0) as i64
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_metadata_for_track() {
        let track = MprisTrack {
            id: "12345".into(),
            title: "Кинолента".into(),
            artist: "КИНО, Гость".into(),
            duration_secs: 263,
        };

        let metadata = metadata(&Some(track));
        assert_eq!(metadata.title(), Some("Кинолента"));
        assert_eq!(
            metadata.artist(),
            Some(vec!["КИНО".to_string(), "Гость".to_string()])
        );
        assert_eq!(metadata.length(), Some(Time::from_secs(263)));
        assert_eq!(
            metadata.trackid().as_ref().map(|id| id.as_str()),
            Some("/yandex/music/track/12345")
        );
    }

    #[test]
    fn empty_metadata_uses_no_track() {
        let metadata = metadata(&None);
        assert_eq!(metadata.title(), None);
        assert_eq!(
            metadata.trackid().as_ref().map(|id| id.as_str()),
            Some("/org/mpris/MediaPlayer2/TrackList/NoTrack")
        );
    }

    #[test]
    fn path_is_sanitized() {
        assert_eq!(track_path("12/34:56"), "/yandex/music/track/123456");
        assert_eq!(track_path(""), "/yandex/music/track/unknown");
    }
}
