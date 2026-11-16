use crate::api::{Track, YandexClient};
use crate::config::LyricsSource;
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::time::Duration;

const LRCLIB_API: &str = "https://lrclib.net/api";
const LRCLIB_CLIENT: &str = "yandex-music-tui/0.1.0 (https://github.com/quesas/yandex-music-tui)";

/// Загружает текст из выбранного источника.
pub async fn fetch_lyrics(
    source: LyricsSource,
    client: &YandexClient,
    track: &Track,
) -> Result<String> {
    match source {
        LyricsSource::Yandex => {
            let text = client.track_lyrics(&track.id).await?;
            if is_placeholder(&text) {
                Err(anyhow!("Яндекс Музыка не отдала текст песни"))
            } else {
                Ok(text)
            }
        }
        LyricsSource::Lrclib => fetch_lrclib(track).await,
        LyricsSource::Auto => fetch_auto(client, track).await,
    }
}

async fn fetch_auto(client: &YandexClient, track: &Track) -> Result<String> {
    let yandex = client
        .track_lyrics(&track.id)
        .await
        .ok()
        .filter(|text| !text.trim().is_empty() && !is_placeholder(text));

    if let Some(text) = yandex.as_deref()
        && is_synced(text)
    {
        return Ok(text.to_string());
    }

    let external = fetch_lrclib(track)
        .await
        .ok()
        .filter(|text| !text.trim().is_empty());

    match (external, yandex) {
        (Some(external), _) if is_synced(&external) => Ok(external),
        (Some(_), Some(yandex)) => Ok(yandex),
        (Some(external), None) => Ok(external),
        (None, Some(yandex)) => Ok(yandex),
        (None, None) => Err(anyhow!("текст песни не найден ни в Яндексе, ни в LRCLIB")),
    }
}

fn is_synced(text: &str) -> bool {
    let lyrics = Lyrics::parse(text);
    lyrics.synced && !lyrics.lines.is_empty()
}

/// Яндекс без активного Плюса может вернуть заглушку вместо текста песни.
fn is_placeholder(text: &str) -> bool {
    let text = normalize(text);
    text.contains("подключите подписку")
        || text.contains("подписку на плюс")
        || text.contains("чтобы видеть текст")
        || text.contains("чтобы увидеть текст")
}

async fn fetch_lrclib(track: &Track) -> Result<String> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(LRCLIB_CLIENT)
        .build()?;
    let artist = primary_artist(&track.artist);
    let mut params = vec![
        ("artist_name", artist.clone()),
        ("track_name", track.title.clone()),
    ];
    if track.duration > 0 {
        params.push(("duration", track.duration.to_string()));
    }

    let response = http
        .get(format!("{LRCLIB_API}/get"))
        .header("Lrclib-Client", LRCLIB_CLIENT)
        .query(&params)
        .send()
        .await?;
    if response.status().is_success() {
        let body: Value = response.json().await?;
        if let Some(text) = lrclib_text(&body) {
            return Ok(text);
        }
    }

    let response = http
        .get(format!("{LRCLIB_API}/search"))
        .header("Lrclib-Client", LRCLIB_CLIENT)
        .query(&[("artist_name", artist), ("track_name", track.title.clone())])
        .send()
        .await?;
    if !response.status().is_success() {
        anyhow::bail!("LRCLIB ответил HTTP {}", response.status());
    }
    let body: Value = response.json().await?;
    let Some(items) = body.as_array() else {
        anyhow::bail!("LRCLIB вернул неожиданный ответ");
    };

    let mut best: Option<(i32, String)> = None;
    for item in items {
        let score = lrclib_score(item, track)
            + if item
                .get("syncedLyrics")
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
            {
                6
            } else {
                0
            };
        let Some(text) = lrclib_text(item) else {
            continue;
        };
        if best.as_ref().map(|(best, _)| score > *best).unwrap_or(true) {
            best = Some((score, text));
        }
    }

    best.map(|(_, text)| text)
        .ok_or_else(|| anyhow!("LRCLIB не нашёл текст для этого трека"))
}

fn lrclib_text(item: &Value) -> Option<String> {
    item.get("syncedLyrics")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            item.get("plainLyrics")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
                .map(str::to_string)
        })
}

fn lrclib_score(item: &Value, track: &Track) -> i32 {
    let artist = item
        .get("artistName")
        .and_then(Value::as_str)
        .map(normalize)
        .unwrap_or_default();
    let title = item
        .get("trackName")
        .and_then(Value::as_str)
        .map(normalize)
        .unwrap_or_default();
    let primary = normalize(&primary_artist(&track.artist));
    let wanted_title = normalize(&track.title);

    let mut score = 0;
    if artist == primary {
        score += 4;
    } else if artist.contains(&primary) || primary.contains(&artist) {
        score += 2;
    }
    if title == wanted_title {
        score += 4;
    } else if title.contains(&wanted_title) || wanted_title.contains(&title) {
        score += 2;
    }

    if let Some(duration) = item.get("duration").and_then(Value::as_f64)
        && track.duration > 0
    {
        let diff = (duration - track.duration as f64).abs();
        if diff <= 2.0 {
            score += 2;
        } else if diff <= 10.0 {
            score += 1;
        }
    }
    score
}

fn primary_artist(artist: &str) -> String {
    artist
        .split(',')
        .next()
        .unwrap_or(artist)
        .trim()
        .to_string()
}

fn normalize(text: &str) -> String {
    text.to_lowercase().trim().to_string()
}

/// Одна строка текста песни.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricLine {
    /// Время начала строки в миллисекундах. Для обычного текста — 0.
    pub time_ms: u64,
    pub text: String,
}

/// Разобранный текст песни.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    /// Есть ли у строк реальные таймкоды (LRC).
    pub synced: bool,
}

impl Lyrics {
    pub fn parse(raw: &str) -> Self {
        let mut lines = Vec::new();
        let mut offset_ms: i64 = 0;

        for source in raw.lines() {
            let source = source.trim_end_matches('\r');
            let (timestamps, text) = split_timestamps(source, &mut offset_ms);
            if timestamps.is_empty() {
                continue;
            }
            for time in timestamps {
                let time = (time as i64 + offset_ms).max(0) as u64;
                lines.push(LyricLine {
                    time_ms: time,
                    text: text.to_string(),
                });
            }
        }

        if lines.is_empty() {
            let plain: Vec<_> = raw
                .lines()
                .map(|line| line.trim().trim_end_matches('\r'))
                .filter(|line| !line.is_empty())
                .map(|line| LyricLine {
                    time_ms: 0,
                    text: line.to_string(),
                })
                .collect();
            return Self {
                synced: false,
                lines: plain,
            };
        }

        lines.sort_by_key(|line| line.time_ms);
        Self {
            synced: true,
            lines,
        }
    }

    /// Индекс строки, которая звучит в момент `position_secs`.
    pub fn current_index(&self, position_secs: f64) -> Option<usize> {
        if !self.synced || self.lines.is_empty() {
            return None;
        }
        let position_ms = (position_secs.max(0.0) * 1000.0) as u64;
        let index = self
            .lines
            .partition_point(|line| line.time_ms <= position_ms);
        index.checked_sub(1)
    }
}

fn split_timestamps<'a>(line: &'a str, offset_ms: &mut i64) -> (Vec<u64>, &'a str) {
    let mut timestamps = Vec::new();
    let mut rest = line.trim_start();

    while let Some(after_open) = rest.strip_prefix('[') {
        let Some(close) = after_open.find(']') else {
            break;
        };
        let tag = &after_open[..close];
        let after = &after_open[close + 1..];

        if let Some(value) = tag.strip_prefix("offset:") {
            if let Ok(value) = value.trim().parse::<i64>() {
                *offset_ms = value;
            }
            rest = after.trim_start();
            continue;
        }

        match parse_timestamp(tag) {
            Some(time) => {
                timestamps.push(time);
                rest = after.trim_start();
            }
            None => break,
        }
    }

    (timestamps, rest.trim())
}

fn parse_timestamp(tag: &str) -> Option<u64> {
    let (minutes, rest) = tag.split_once(':')?;
    let minutes: u64 = minutes.trim().parse().ok()?;

    let (seconds, fraction) = match rest.split_once(['.', ':']) {
        Some((seconds, fraction)) => (seconds, Some(fraction)),
        None => (rest, None),
    };
    let seconds: u64 = seconds.trim().parse().ok()?;
    let millis = fraction
        .map(|fraction| {
            let digits: String = fraction
                .chars()
                .take_while(char::is_ascii_digit)
                .take(3)
                .collect();
            if digits.is_empty() {
                return 0;
            }
            let value: u64 = digits.parse().unwrap_or(0);
            match digits.len() {
                1 => value * 100,
                2 => value * 10,
                _ => value,
            }
        })
        .unwrap_or(0);

    Some((minutes * 60 + seconds) * 1000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lrc_with_multiple_timestamps() {
        let raw = "[ar:Артист]\n[00:01.20][00:05.500]Привет\n[00:12]Мир\n";
        let lyrics = Lyrics::parse(raw);
        assert!(lyrics.synced);
        assert_eq!(lyrics.lines.len(), 3);
        assert_eq!(lyrics.lines[0].time_ms, 1200);
        assert_eq!(lyrics.lines[0].text, "Привет");
        assert_eq!(lyrics.lines[1].time_ms, 5500);
        assert_eq!(lyrics.lines[2].time_ms, 12_000);
    }

    #[test]
    fn applies_lrc_offset() {
        let raw = "[offset:500]\n[00:01.00]Строка";
        let lyrics = Lyrics::parse(raw);
        assert_eq!(lyrics.lines[0].time_ms, 1500);
    }

    #[test]
    fn falls_back_to_plain_text() {
        let lyrics = Lyrics::parse("Первая строка\n\nВторая строка\n");
        assert!(!lyrics.synced);
        assert_eq!(lyrics.lines.len(), 2);
        assert_eq!(lyrics.lines[1].text, "Вторая строка");
    }

    #[test]
    fn finds_current_line() {
        let lyrics = Lyrics::parse("[00:01.00]Раз\n[00:03.00]Два\n[00:08.00]Три");
        assert_eq!(lyrics.current_index(0.5), None);
        assert_eq!(lyrics.current_index(1.0), Some(0));
        assert_eq!(lyrics.current_index(4.0), Some(1));
        assert_eq!(lyrics.current_index(20.0), Some(2));
    }

    #[test]
    fn prefers_synced_external_lyrics() {
        let item = serde_json::json!({
            "trackName": "Песня",
            "artistName": "Артист",
            "syncedLyrics": "[00:01.00]Строка",
            "plainLyrics": "Строка"
        });
        assert_eq!(lrclib_text(&item).as_deref(), Some("[00:01.00]Строка"));

        let plain = serde_json::json!({
            "trackName": "Песня",
            "artistName": "Артист",
            "syncedLyrics": null,
            "plainLyrics": "Обычный текст"
        });
        assert_eq!(lrclib_text(&plain).as_deref(), Some("Обычный текст"));
    }

    #[test]
    fn takes_primary_artist_from_yandex_list() {
        assert_eq!(primary_artist("КИНО, Гость"), "КИНО");
        assert_eq!(primary_artist("Один"), "Один");
    }

    #[test]
    fn detects_synced_and_plain_text() {
        assert!(is_synced("[00:01.00]Строка"));
        assert!(!is_synced("Обычный текст\nБез времени"));
    }

    #[test]
    fn detects_yandex_plus_placeholder() {
        assert!(is_placeholder(
            "[00:00.00]Подключите подписку на Плюс, чтобы видеть текст песни"
        ));
        assert!(!is_placeholder("Обычный текст песни"));
    }
}
