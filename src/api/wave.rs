use super::{Track, track_from_json};
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct WaveBatch {
    pub tracks: Vec<Track>,
    pub session_id: Option<String>,
    pub batch_id: Option<String>,
    pub queue: Vec<String>,
}

impl WaveBatch {
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

#[derive(Debug, Clone, Default)]
pub struct WaveContext {
    pub seed: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct WaveSettingGroup {
    pub key: String,
    pub name: String,
    pub values: Vec<WaveSettingValue>,
}

#[derive(Debug, Clone, Default)]
pub struct WaveSettingValue {
    pub name: String,
    pub seed: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WaveSettings {
    pub contexts: Vec<WaveContext>,
    pub groups: Vec<WaveSettingGroup>,
}

impl WaveSettings {
    pub fn from_response(body: &Value) -> Self {
        let result = body.get("result").unwrap_or(body);
        let mut contexts = Vec::new();
        if let Some(blocks) = result.get("blocks").and_then(Value::as_array) {
            for block in blocks {
                let Some(items) = block.get("items").and_then(Value::as_array) else {
                    continue;
                };
                for item in items {
                    let id = item.get("id");
                    let kind = id
                        .and_then(|id| id.get("type"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let tag = id
                        .and_then(|id| id.get("tag"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let name = item.get("name").and_then(Value::as_str).unwrap_or_default();
                    if kind.is_empty() || tag.is_empty() || name.is_empty() {
                        continue;
                    }
                    contexts.push(WaveContext {
                        seed: format!("{kind}:{tag}"),
                        name: name.to_string(),
                    });
                }
            }
        }

        let mut groups = Vec::new();
        if let Some(restrictions) = result.get("settingRestrictions").and_then(Value::as_object) {
            for (key, restriction) in restrictions {
                if restriction.get("type").and_then(Value::as_str) != Some("enum") {
                    continue;
                }
                let Some(possible) = restriction.get("possibleValues").and_then(Value::as_array)
                else {
                    continue;
                };
                let name = restriction
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(key)
                    .to_string();
                let mut values = Vec::new();
                for value in possible {
                    let value_name = value
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if value_name.is_empty() {
                        continue;
                    }
                    let unspecified = value
                        .get("unspecified")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let seed = value
                        .get("serializedSeed")
                        .and_then(Value::as_str)
                        .filter(|_| !unspecified)
                        .map(str::to_string);
                    values.push(WaveSettingValue {
                        name: value_name.to_string(),
                        seed,
                    });
                }
                if !values.is_empty() {
                    groups.push(WaveSettingGroup {
                        key: key.clone(),
                        name,
                        values,
                    });
                }
            }
        }

        groups.sort_by_key(|group| match group.key.as_str() {
            "moodEnergy" => 0,
            "language" => 1,
            "diversity" => 2,
            _ => 3,
        });

        if contexts.is_empty() && groups.is_empty() {
            return Self::fallback();
        }

        Self { contexts, groups }
    }

    pub fn fallback() -> Self {
        let group = |key: &str, name: &str, values: &[(&str, Option<&str>)]| WaveSettingGroup {
            key: key.to_string(),
            name: name.to_string(),
            values: values
                .iter()
                .map(|(name, seed)| WaveSettingValue {
                    name: (*name).to_string(),
                    seed: seed.map(str::to_string),
                })
                .collect(),
        };

        Self {
            contexts: vec![
                WaveContext {
                    seed: "activity:wake-up".into(),
                    name: "Просыпаюсь".into(),
                },
                WaveContext {
                    seed: "activity:road-trip".into(),
                    name: "В дороге".into(),
                },
                WaveContext {
                    seed: "activity:work-background".into(),
                    name: "Работаю".into(),
                },
                WaveContext {
                    seed: "activity:workout".into(),
                    name: "Тренируюсь".into(),
                },
                WaveContext {
                    seed: "activity:fall-asleep".into(),
                    name: "Засыпаю".into(),
                },
            ],
            groups: vec![
                group(
                    "moodEnergy",
                    "Под настроение",
                    &[
                        ("Бодрое", Some("settingMoodEnergy:active")),
                        ("Весёлое", Some("settingMoodEnergy:fun")),
                        ("Спокойное", Some("settingMoodEnergy:calm")),
                        ("Грустное", Some("settingMoodEnergy:sad")),
                        ("Любое", None),
                    ],
                ),
                group(
                    "language",
                    "По языку",
                    &[
                        ("Русский", Some("settingLanguage:russian")),
                        ("Иностранный", Some("settingLanguage:not-russian")),
                        ("Без слов", Some("settingLanguage:without-words")),
                        ("Любой", None),
                    ],
                ),
                group(
                    "diversity",
                    "По характеру",
                    &[
                        ("Любимое", Some("settingDiversity:favorite")),
                        ("Незнакомое", Some("settingDiversity:discover")),
                        ("Популярное", Some("settingDiversity:popular")),
                        ("Любое", None),
                    ],
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_wave_settings_contexts_and_groups() {
        let body = json!({
            "result": {
                "blocks": [{
                    "type": "contexts",
                    "items": [{
                        "id": {"type": "activity", "tag": "fall-asleep"},
                        "name": "Засыпаю"
                    }]
                }],
                "settingRestrictions": {
                    "language": {
                        "type": "enum",
                        "name": "По языку",
                        "possibleValues": [
                            {"value": "russian", "name": "Русский", "serializedSeed": "settingLanguage:russian"},
                            {"value": "any", "name": "Любой", "unspecified": true, "serializedSeed": "settingLanguage:any"}
                        ]
                    },
                    "moodEnergy": {
                        "type": "enum",
                        "name": "Под настроение",
                        "possibleValues": [
                            {"value": "sad", "name": "Грустное", "serializedSeed": "settingMoodEnergy:sad"}
                        ]
                    }
                }
            }
        });

        let settings = WaveSettings::from_response(&body);
        assert_eq!(settings.contexts.len(), 1);
        assert_eq!(settings.contexts[0].seed, "activity:fall-asleep");
        assert_eq!(settings.contexts[0].name, "Засыпаю");
        assert_eq!(settings.groups.len(), 2);
        assert_eq!(settings.groups[0].key, "moodEnergy");
        assert_eq!(
            settings.groups[0].values[0].seed.as_deref(),
            Some("settingMoodEnergy:sad")
        );
        assert_eq!(settings.groups[1].key, "language");
        assert_eq!(settings.groups[1].values[1].seed, None);
    }

    #[test]
    fn missing_settings_use_fallback() {
        let settings = WaveSettings::from_response(&json!({}));
        assert!(!settings.contexts.is_empty());
        assert!(settings.groups.iter().any(|group| group.key == "language"));
    }
}
