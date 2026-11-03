use super::{Track, track_from_json};
use serde_json::Value;

/// Порция треков из rotor-сессии «Моей волны».
#[derive(Debug, Clone, Default)]
pub struct WaveBatch {
    pub tracks: Vec<Track>,
    pub session_id: Option<String>,
    pub batch_id: Option<String>,
    pub queue: Vec<String>,
}

impl WaveBatch {
    /// Разбор ответа /rotor/session/new и /rotor/session/{id}/tracks.
    pub fn from_response(body: &Value) -> Self {
        let result = body.get("result").unwrap_or(body);
        let session_id = result
            .get("radioSessionId")
            .or_else(|| result.get("sessionId"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let batch_id = result
            .get("batchId")
            .and_then(Value::as_str)
            .map(str::to_string);

        let mut tracks = Vec::new();
        if let Some(sequence) = result.get("sequence").and_then(Value::as_array) {
            for item in sequence {
                let payload = item.get("track").unwrap_or(item);
                if let Some(track) = track_from_json(payload) {
                    tracks.push(track);
                }
            }
        }

        let queue = tracks.iter().map(Track::full_id).collect();

        Self {
            tracks,
            session_id,
            batch_id,
            queue,
        }
    }
}
