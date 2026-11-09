pub mod models;
pub mod wave;

pub use models::{Album, Artist, Playlist, SearchFilter, SearchItem, Track};
pub use wave::{WaveBatch, WaveSettingGroup, WaveSettings};

use crate::config::Quality;
use anyhow::Result;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashSet;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const BASE_URL: &str = "https://api.music.yandex.ru";

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) YandexMusic/5.62.0 Chrome/136.0.7103.149 Electron/36.4.0 Safari/537.36";

#[derive(Debug)]
pub struct SessionExpired;

impl fmt::Display for SessionExpired {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "сессия Яндекс Музыки истекла, требуется повторный вход")
    }
}

impl std::error::Error for SessionExpired {}

#[derive(Debug, Clone)]
pub struct YandexClient {
    pub token: Option<String>,
    http: reqwest::Client,
}

impl YandexClient {
    pub fn new(token: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { token, http }
    }

    fn auth_headers(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        self.auth_headers_for(request, "YandexMusicDesktopAppWindows/5.62.0")
    }

    fn auth_headers_for(
        &self,
        request: reqwest::RequestBuilder,
        client: &str,
    ) -> reqwest::RequestBuilder {
        let request = request
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::ACCEPT_LANGUAGE, "ru")
            .header("Origin", "music-application://desktop")
            .header("x-yandex-music-client", client)
            .header("x-yandex-music-without-invocation-info", "1");

        match &self.token {
            Some(token) => request.header(reqwest::header::AUTHORIZATION, format!("OAuth {token}")),
            None => request,
        }
    }

    pub(crate) async fn get_json(&self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        let url = format!("{BASE_URL}/{path}");
        let req = self.auth_headers(self.http.get(url)).query(query);
        self.send(req).await
    }

    async fn get_json_android(&self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        let url = format!("{BASE_URL}/{path}");
        let req = self
            .auth_headers_for(self.http.get(url), "YandexMusicAndroid/24023621")
            .query(query);
        self.send(req).await
    }

    pub(crate) async fn post_json(&self, path: &str, body: &Value) -> Result<Value> {
        let url = format!("{BASE_URL}/{path}");
        let req = self.auth_headers(self.http.post(url)).json(body);
        self.send(req).await
    }

    pub(crate) async fn post_form(&self, path: &str, form: &[(&str, String)]) -> Result<Value> {
        let url = format!("{BASE_URL}/{path}");
        let req = self.auth_headers(self.http.post(url)).form(form);
        self.send(req).await
    }

    async fn send(&self, req: reqwest::RequestBuilder) -> Result<Value> {
        let resp = req.send().await?;
        let status = resp.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SessionExpired.into());
        }

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let snippet: String = text.chars().take(300).collect();
            anyhow::bail!("Яндекс Музыка вернула HTTP {status}: {snippet}");
        }

        Ok(resp.json().await?)
    }

    pub async fn account(&self) -> Result<AccountInfo> {
        if self.token.is_none() {
            return Ok(AccountInfo {
                nickname: "guest (без токена)".into(),
                login: None,
                uid: None,
                plus: false,
            });
        }

        let body = self.get_json("account/about", &[]).await?;
        let info = parse_account_about(&body)?;

        if info.uid.is_none() && info.login.is_none() {
            return Err(SessionExpired.into());
        }

        Ok(info)
    }

    pub async fn liked_tracks(&self, uid: u64) -> Result<Vec<Track>> {
        let body = self
            .get_json(&format!("users/{uid}/likes/tracks"), &[])
            .await?;
        let ids = library_track_ids(&body);

        let mut tracks = Vec::new();
        for chunk in ids.chunks(100) {
            let body = self
                .get_json("tracks", &[("trackIds", chunk.join(","))])
                .await?;
            let items = body
                .get("result")
                .and_then(Value::as_array)
                .or_else(|| body.as_array());
            if let Some(items) = items {
                tracks.extend(items.iter().filter_map(track_from_json));
            }
        }

        Ok(tracks)
    }

    pub async fn liked_track_ids(&self, uid: u64) -> Result<HashSet<String>> {
        let body = self
            .get_json(&format!("users/{uid}/likes/tracks"), &[])
            .await?;
        Ok(library_track_ids(&body).into_iter().collect())
    }

    pub async fn disliked_track_ids(&self, uid: u64) -> Result<HashSet<String>> {
        let body = self
            .get_json(&format!("users/{uid}/dislikes/tracks"), &[])
            .await?;
        Ok(library_track_ids(&body).into_iter().collect())
    }

    pub async fn set_track_like(&self, uid: u64, track_id: &str, liked: bool) -> Result<()> {
        let action = if liked { "add-multiple" } else { "remove" };
        let form = [("track-ids", track_id.to_string())];
        self.post_form(&format!("users/{uid}/likes/tracks/{action}"), &form)
            .await?;
        Ok(())
    }

    pub async fn set_track_dislike(&self, uid: u64, track_id: &str, disliked: bool) -> Result<()> {
        let action = if disliked { "add-multiple" } else { "remove" };
        let form = [("track-ids", track_id.to_string())];
        self.post_form(&format!("users/{uid}/dislikes/tracks/{action}"), &form)
            .await?;
        Ok(())
    }

    pub async fn liked_artists(&self, uid: u64) -> Result<Vec<Artist>> {
        let body = self
            .get_json(
                &format!("users/{uid}/likes/artists/page"),
                &[
                    ("metaType", "music".into()),
                    ("rich", "true".into()),
                    ("page", "0".into()),
                    ("pageSize", "200".into()),
                ],
            )
            .await?;
        let result = result_or_body(&body);

        Ok(result
            .get("artists")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(artist_from_json).collect())
            .unwrap_or_default())
    }

    pub async fn liked_playlists(&self, uid: u64) -> Result<Vec<Playlist>> {
        let body = self
            .get_json(
                &format!("users/{uid}/likes/playlists/page"),
                &[
                    ("metaType", "music".into()),
                    ("rich", "true".into()),
                    ("page", "0".into()),
                    ("pageSize", "200".into()),
                ],
            )
            .await?;
        let result = result_or_body(&body);

        Ok(result
            .get("playlists")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(playlist_from_json).collect())
            .unwrap_or_default())
    }

    pub async fn playlist_tracks(&self, uid: &str, kind: &str) -> Result<Vec<Track>> {
        let body = self
            .get_json(
                &format!("users/{uid}/playlists/{kind}"),
                &[
                    ("richTracks", "true".into()),
                    ("page", "0".into()),
                    ("pageSize", "500".into()),
                ],
            )
            .await?;
        let result = result_or_body(&body);

        Ok(result
            .get("tracks")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(track_entry_from_json).collect())
            .unwrap_or_default())
    }

    pub async fn track_audio_url(&self, track_id: &str, quality: Quality) -> Result<String> {
        let body = self
            .get_json(&format!("tracks/{track_id}/download-info"), &[])
            .await?;
        let result = result_or_body(&body);

        let variants = result
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("download-info вернул не список вариантов"))?;

        let best = select_audio_variant(variants, quality)
            .ok_or_else(|| anyhow::anyhow!("Яндекс Музыка не предложила аудио для трека"))?;

        let info = self
            .http
            .get(best.url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?;
        if !info.status().is_success() {
            anyhow::bail!("download-info HTTP {}", info.status());
        }
        let xml = info.text().await?;

        let host =
            xml_value(&xml, "host").ok_or_else(|| anyhow::anyhow!("в download-info нет host"))?;
        let path =
            xml_value(&xml, "path").ok_or_else(|| anyhow::anyhow!("в download-info нет path"))?;
        let ts = xml_value(&xml, "ts").ok_or_else(|| anyhow::anyhow!("в download-info нет ts"))?;
        let s = xml_value(&xml, "s").ok_or_else(|| anyhow::anyhow!("в download-info нет s"))?;

        Ok(format!("https://{host}/get-mp3/{s}/{ts}{path}"))
    }

    pub async fn track_audio(&self, track_id: &str, quality: Quality) -> Result<Vec<u8>> {
        let url = self.track_audio_url(track_id, quality).await?;
        let resp = self
            .http
            .get(url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("аудиофайл вернул HTTP {}", resp.status());
        }
        Ok(resp.bytes().await?.to_vec())
    }

    /// Загружает текст трека в формате LRC (с таймкодами).
    ///
    /// Если синхронизированного текста нет, возвращается обычный текст.
    pub async fn track_lyrics(&self, track_id: &str) -> Result<String> {
        match self.lyrics_download(track_id, "LRC").await {
            Ok(lrc) if !lrc.trim().is_empty() => Ok(lrc),
            Ok(_) => self.lyrics_download(track_id, "TEXT").await,
            Err(err) if err.downcast_ref::<SessionExpired>().is_some() => Err(err),
            Err(_) => self.lyrics_download(track_id, "TEXT").await,
        }
    }

    async fn lyrics_download(&self, track_id: &str, format: &str) -> Result<String> {
        let (timestamp, sign) = lyrics_sign(track_id);
        let body = self
            .get_json_android(
                &format!("tracks/{track_id}/lyrics"),
                &[
                    ("format", format.to_string()),
                    ("timeStamp", timestamp.to_string()),
                    ("sign", sign),
                ],
            )
            .await?;
        let result = result_or_body(&body);

        if let Some(text) = result.get("lyrics").and_then(Value::as_str) {
            return Ok(text.to_string());
        }

        let url = result
            .get("downloadUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("в ответе нет ссылки на текст песни"))?;
        let resp = self
            .http
            .get(url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("текст песни вернул HTTP {}", resp.status());
        }
        Ok(resp.text().await?)
    }

    /// Скачивает обложку трека. `%%` в ссылке заменяется на нужный размер.
    pub async fn track_cover(&self, cover_uri: &str) -> Result<Vec<u8>> {
        let url = cover_url(cover_uri, 400);
        let resp = self
            .http
            .get(url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("обложка вернула HTTP {}", resp.status());
        }
        Ok(resp.bytes().await?.to_vec())
    }

    pub async fn album_tracks(&self, id: &str) -> Result<Vec<Track>> {
        let body = self
            .get_json(
                &format!("albums/{id}/with-tracks"),
                &[("richTracks", "true".into())],
            )
            .await?;
        let result = result_or_body(&body);

        let mut tracks = Vec::new();
        if let Some(volumes) = result.get("volumes").and_then(Value::as_array) {
            for volume in volumes {
                if let Some(items) = volume.as_array() {
                    tracks.extend(items.iter().filter_map(track_entry_from_json));
                }
            }
        }
        if tracks.is_empty()
            && let Some(items) = result.get("tracks").and_then(Value::as_array)
        {
            tracks.extend(items.iter().filter_map(track_entry_from_json));
        }

        Ok(tracks)
    }

    pub async fn artist_tracks(&self, id: &str) -> Result<Vec<Track>> {
        let body = self
            .get_json(
                &format!("artists/{id}/tracks"),
                &[("page", "0".into()), ("pageSize", "200".into())],
            )
            .await?;
        let result = result_or_body(&body);

        Ok(result
            .get("tracks")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(track_entry_from_json).collect())
            .unwrap_or_default())
    }

    pub async fn search(
        &self,
        text: &str,
        filter: SearchFilter,
        page: u32,
    ) -> Result<Vec<SearchItem>> {
        let body = self
            .get_json(
                "search/instant/mixed",
                &[
                    ("text", text.to_string()),
                    ("type", filter.api_value().to_string()),
                    ("page", page.to_string()),
                    ("pageSize", "50".into()),
                    ("withLikesCount", "false".into()),
                    ("withBestResults", "false".into()),
                ],
            )
            .await?;

        let result = body.get("result").unwrap_or(&body);

        let mut items = Vec::new();
        if let Some(results) = result.get("results").and_then(Value::as_array) {
            for item in results {
                let kind = item.get("type").and_then(Value::as_str).unwrap_or_default();
                let payload = item.get(kind).unwrap_or(item);
                match kind {
                    "track" | "podcast-episode" => {
                        if let Some(track) = track_from_json(payload) {
                            items.push(SearchItem::Track(track));
                        }
                    }
                    "artist" => {
                        if let Some(artist) = artist_from_json(payload) {
                            items.push(SearchItem::Artist(artist));
                        }
                    }
                    "album" => {
                        if let Some(album) = album_from_json(payload) {
                            items.push(SearchItem::Album(album));
                        }
                    }
                    "playlist" => {
                        if let Some(playlist) = playlist_from_json(payload) {
                            items.push(SearchItem::Playlist(playlist));
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(items)
    }

    pub async fn wave_settings(&self) -> Result<WaveSettings> {
        let body = self
            .get_json(
                "rotor/wave/settings",
                &[("seeds", "user:onyourwave".to_string())],
            )
            .await?;
        Ok(WaveSettings::from_response(&body))
    }

    pub async fn start_wave(&self, seeds: &[String]) -> Result<WaveBatch> {
        let attempts = if seeds.is_empty() {
            vec![
                vec!["myVibe".to_string()],
                vec!["user:onyourwave".to_string()],
            ]
        } else {
            vec![seeds.to_vec()]
        };
        let mut last_error = None;
        for seeds in attempts {
            let body = serde_json::json!({
                "queue": [],
                "seeds": seeds,
                "includeTracksInResponse": true,
                "interactive": true,
            });
            match self.post_json("rotor/session/new", &body).await {
                Ok(body) => return Ok(WaveBatch::from_response(&body)),
                Err(err) if err.downcast_ref::<SessionExpired>().is_some() => return Err(err),
                Err(err) => last_error = Some(err),
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("не удалось запустить Мою волну")))
    }

    pub async fn next_wave(&self, session_id: &str, queue: &[String]) -> Result<WaveBatch> {
        let body = serde_json::json!({ "queue": queue });
        let body = self
            .post_json(&format!("rotor/session/{session_id}/tracks"), &body)
            .await?;
        Ok(WaveBatch::from_response(&body))
    }
}

fn result_or_body(body: &Value) -> &Value {
    body.get("result").unwrap_or(body)
}

fn library_track_ids(body: &Value) -> Vec<String> {
    let result = result_or_body(body);
    let mut ids = Vec::new();
    if let Some(tracks) = result
        .get("library")
        .and_then(|library| library.get("tracks"))
        .and_then(Value::as_array)
    {
        for item in tracks {
            let id = item.get("id").and_then(value_id);
            let album = item.get("albumId").and_then(value_id);
            if let Some(id) = id {
                ids.push(match album {
                    Some(album) => format!("{id}:{album}"),
                    None => id,
                });
            }
        }
    }
    ids
}

pub fn parse_account_about(body: &serde_json::Value) -> Result<AccountInfo> {
    let result = result_or_body(body);

    let uid = result.get("uid").and_then(value_id_u64);

    let login = result
        .get("login")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let display_name = result
        .get("displayName")
        .or_else(|| result.get("display_name"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let nickname = display_name
        .or_else(|| login.clone())
        .unwrap_or_else(|| "Пользователь".to_string());

    let plus = result
        .get("hasPlus")
        .and_then(|v| v.as_bool())
        .or_else(|| {
            result
                .get("plus")
                .and_then(|p| p.get("hasPlus"))
                .and_then(|v| v.as_bool())
        })
        .unwrap_or(false);

    Ok(AccountInfo {
        nickname,
        login,
        uid,
        plus,
    })
}

fn value_id(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        (!s.is_empty()).then(|| s.to_string())
    } else if let Some(n) = value.as_u64() {
        Some(n.to_string())
    } else {
        value.as_i64().map(|n| n.to_string())
    }
}

fn value_id_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

fn value_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

struct AudioVariant<'a> {
    codec: &'a str,
    bitrate: u64,
    url: &'a str,
}

impl AudioVariant<'_> {
    fn is_lossless(&self) -> bool {
        self.codec.eq_ignore_ascii_case("flac")
    }
}

fn select_audio_variant<'a>(variants: &'a [Value], quality: Quality) -> Option<AudioVariant<'a>> {
    let parsed: Vec<_> = variants
        .iter()
        .filter(|item| {
            !item
                .get("preview")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|item| {
            let url = item.get("downloadInfoUrl").and_then(Value::as_str)?;
            let codec = item.get("codec").and_then(Value::as_str).unwrap_or("");
            let bitrate = item
                .get("bitrateInKbps")
                .or_else(|| item.get("bitrate"))
                .and_then(value_u64)
                .unwrap_or(0);
            Some(AudioVariant {
                codec,
                bitrate,
                url,
            })
        })
        .collect();

    let (lossless, lossy): (Vec<_>, Vec<_>) =
        parsed.into_iter().partition(AudioVariant::is_lossless);

    match quality {
        Quality::Lossless => max_by_bitrate(lossless).or_else(|| max_by_bitrate(lossy)),
        Quality::High => {
            closest_by_bitrate(lossy, 320).or_else(|| closest_by_bitrate(lossless, 320))
        }
        Quality::Medium => {
            closest_by_bitrate(lossy, 192).or_else(|| closest_by_bitrate(lossless, 192))
        }
        Quality::Low => closest_by_bitrate(lossy, 64).or_else(|| closest_by_bitrate(lossless, 64)),
    }
}

fn max_by_bitrate<'a>(variants: Vec<AudioVariant<'a>>) -> Option<AudioVariant<'a>> {
    variants.into_iter().max_by_key(|variant| variant.bitrate)
}

fn closest_by_bitrate<'a>(
    variants: Vec<AudioVariant<'a>>,
    target: u64,
) -> Option<AudioVariant<'a>> {
    variants.into_iter().min_by_key(|variant| {
        (
            variant.bitrate.abs_diff(target),
            std::cmp::Reverse(variant.bitrate),
        )
    })
}

fn artists_names(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

const LYRICS_SIGN_KEY: &str = "p93jhgh689SBReK6ghtw62";

fn lyrics_sign(track_id: &str) -> (u64, String) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let mut mac = Hmac::<Sha256>::new_from_slice(LYRICS_SIGN_KEY.as_bytes())
        .expect("HMAC принимает ключ любой длины");
    mac.update(format!("{track_id}{timestamp}").as_bytes());
    let sign = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    (timestamp, sign)
}

fn cover_url(cover_uri: &str, size: u32) -> String {
    let with_size = if cover_uri.contains("%%") {
        cover_uri.replace("%%", &format!("{size}x{size}"))
    } else {
        cover_uri.to_string()
    };
    if with_size.starts_with("http://") || with_size.starts_with("https://") {
        with_size
    } else {
        format!("https://{with_size}")
    }
}

fn cover_uri_from_json(value: &Value) -> Option<String> {
    value
        .get("coverUri")
        .and_then(Value::as_str)
        .filter(|uri| !uri.is_empty())
        .map(str::to_string)
        .or_else(|| {
            value
                .get("albums")
                .and_then(Value::as_array)
                .and_then(|albums| albums.first())
                .and_then(|album| album.get("coverUri"))
                .and_then(Value::as_str)
                .filter(|uri| !uri.is_empty())
                .map(str::to_string)
        })
        .or_else(|| {
            value
                .get("ogImage")
                .and_then(Value::as_str)
                .filter(|uri| !uri.is_empty())
                .map(str::to_string)
        })
}

fn xml_value(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].to_string())
}

fn track_entry_from_json(value: &Value) -> Option<Track> {
    track_from_json(value.get("track").unwrap_or(value))
}

pub(crate) fn track_from_json(value: &Value) -> Option<Track> {
    let id = value.get("id").and_then(value_id)?;
    let title = value.get("title")?.as_str()?.trim();
    if title.is_empty() {
        return None;
    }

    let artist = artists_names(value.get("artists"));
    let duration = value
        .get("durationMs")
        .and_then(value_u64)
        .map(|ms| ms / 1000)
        .or_else(|| value.get("duration").and_then(value_u64))
        .unwrap_or(0);
    let album_id = value.get("albumId").and_then(value_id).or_else(|| {
        value
            .get("albums")
            .and_then(Value::as_array)
            .and_then(|albums| albums.first())
            .and_then(|album| album.get("id"))
            .and_then(value_id)
    });
    let available = value
        .get("available")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let cover_uri = cover_uri_from_json(value);

    Some(Track {
        id,
        album_id,
        title: title.to_string(),
        artist,
        duration,
        available,
        cover_uri,
    })
}

fn artist_from_json(value: &Value) -> Option<Artist> {
    let id = value.get("id").and_then(value_id)?;
    let name = value.get("name")?.as_str()?.trim();
    if name.is_empty() {
        return None;
    }
    let counts = value.get("counts");
    let tracks_count = counts
        .and_then(|c| c.get("tracks"))
        .and_then(value_u64)
        .unwrap_or(0);
    let albums_count = counts
        .and_then(|c| c.get("directAlbums"))
        .and_then(value_u64)
        .unwrap_or(0);

    Some(Artist {
        id,
        name: name.to_string(),
        tracks_count,
        albums_count,
    })
}

fn album_from_json(value: &Value) -> Option<Album> {
    let id = value.get("id").and_then(value_id)?;
    let title = value.get("title")?.as_str()?.trim();
    if title.is_empty() {
        return None;
    }

    Some(Album {
        id,
        title: title.to_string(),
        artist: artists_names(value.get("artists")),
        year: value.get("year").and_then(|v| v.as_i64()),
        track_count: value.get("trackCount").and_then(value_u64).unwrap_or(0),
    })
}

fn playlist_from_json(value: &Value) -> Option<Playlist> {
    let uid = value.get("uid").and_then(value_id)?;
    let kind = value.get("kind").and_then(value_id)?;
    let title = value.get("title")?.as_str()?.trim();
    if title.is_empty() {
        return None;
    }

    Some(Playlist {
        uid,
        kind,
        title: title.to_string(),
        track_count: value.get("trackCount").and_then(value_u64).unwrap_or(0),
    })
}

#[derive(Debug, Clone, Default)]
pub struct AccountInfo {
    pub nickname: String,
    pub login: Option<String>,
    pub uid: Option<u64>,
    pub plus: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_murglar_account_about() {
        let body = serde_json::json!({
            "result": {
                "uid": 123456,
                "login": "listener",
                "displayName": "Слушатель",
                "hasPlus": true
            }
        });
        let info = parse_account_about(&body).unwrap();
        assert_eq!(info.uid, Some(123456));
        assert_eq!(info.login.as_deref(), Some("listener"));
        assert_eq!(info.nickname, "Слушатель");
        assert!(info.plus);
    }

    #[test]
    fn parse_string_uid_and_nested_plus() {
        let body = serde_json::json!({
            "result": {
                "uid": "42",
                "login": "user",
                "plus": { "hasPlus": true }
            }
        });
        let info = parse_account_about(&body).unwrap();
        assert_eq!(info.uid, Some(42));
        assert!(info.plus);
    }

    #[test]
    fn parses_search_track_and_artist() {
        let track = serde_json::json!({
            "id": "48591617",
            "title": "Кинолента",
            "durationMs": 263320,
            "available": true,
            "artists": [{"id": 1, "name": "КИНО"}, {"id": 2, "name": "Гость"}],
            "albums": [{"id": 15100069, "title": "Альбом"}]
        });
        let parsed = track_from_json(&track).unwrap();
        assert_eq!(parsed.id, "48591617");
        assert_eq!(parsed.artist, "КИНО, Гость");
        assert_eq!(parsed.album_id.as_deref(), Some("15100069"));
        assert_eq!(parsed.duration, 263);

        let artist = serde_json::json!({
            "id": "41075",
            "name": "КИНО",
            "counts": {"tracks": 146, "directAlbums": 18}
        });
        let parsed = artist_from_json(&artist).unwrap();
        assert_eq!(parsed.id, "41075");
        assert_eq!(parsed.tracks_count, 146);
        assert_eq!(parsed.albums_count, 18);
    }

    #[test]
    fn builds_cover_url_with_size() {
        assert_eq!(
            cover_url("avatars.yandex.net/get-music-content/1/cover/%%", 400),
            "https://avatars.yandex.net/get-music-content/1/cover/400x400"
        );
        assert_eq!(
            cover_url("https://example.com/cover.jpg", 400),
            "https://example.com/cover.jpg"
        );
    }

    #[test]
    fn parses_cover_from_track_or_album() {
        let track = serde_json::json!({
            "id": 1,
            "title": "Трек",
            "coverUri": "avatars.yandex.net/track/%%",
            "albums": [{"id": 2, "coverUri": "avatars.yandex.net/album/%%"}]
        });
        assert_eq!(
            track_from_json(&track).unwrap().cover_uri.as_deref(),
            Some("avatars.yandex.net/track/%%")
        );

        let album_only = serde_json::json!({
            "id": 1,
            "title": "Трек",
            "albums": [{"id": 2, "coverUri": "avatars.yandex.net/album/%%"}]
        });
        assert_eq!(
            track_from_json(&album_only).unwrap().cover_uri.as_deref(),
            Some("avatars.yandex.net/album/%%")
        );
    }

    #[test]
    fn parses_download_info_xml() {
        let xml = "<download-info><host>storage.example</host><path>/abc</path>                   <ts>123</ts><region>-1</region><s>sig</s></download-info>";
        assert_eq!(xml_value(xml, "host").as_deref(), Some("storage.example"));
        assert_eq!(xml_value(xml, "path").as_deref(), Some("/abc"));
        assert_eq!(xml_value(xml, "ts").as_deref(), Some("123"));
        assert_eq!(xml_value(xml, "s").as_deref(), Some("sig"));
        assert_eq!(xml_value(xml, "missing"), None);
    }

    #[test]
    fn selects_audio_variant_by_quality() {
        let variants = vec![
            serde_json::json!({"codec": "flac", "bitrateInKbps": 900, "downloadInfoUrl": "flac"}),
            serde_json::json!({"codec": "mp3", "bitrateInKbps": 320, "downloadInfoUrl": "mp3-320"}),
            serde_json::json!({"codec": "aac", "bitrateInKbps": 192, "downloadInfoUrl": "aac-192"}),
            serde_json::json!({"codec": "aac", "bitrateInKbps": 64, "downloadInfoUrl": "aac-64"}),
            serde_json::json!({
                "codec": "flac",
                "bitrateInKbps": 900,
                "preview": true,
                "downloadInfoUrl": "preview"
            }),
        ];

        let pick = |quality| select_audio_variant(&variants, quality).unwrap().url;
        assert_eq!(pick(Quality::Lossless), "flac");
        assert_eq!(pick(Quality::High), "mp3-320");
        assert_eq!(pick(Quality::Medium), "aac-192");
        assert_eq!(pick(Quality::Low), "aac-64");
    }

    #[test]
    fn falls_back_to_best_available_quality() {
        let variants = vec![
            serde_json::json!({"codec": "mp3", "bitrateInKbps": 320, "downloadInfoUrl": "mp3-320"}),
            serde_json::json!({"codec": "aac", "bitrateInKbps": 64, "downloadInfoUrl": "aac-64"}),
        ];

        assert_eq!(
            select_audio_variant(&variants, Quality::Lossless)
                .unwrap()
                .url,
            "mp3-320"
        );
    }

    #[test]
    fn unwraps_rich_playlist_track_entry() {
        let entry = serde_json::json!({
            "id": 38633712,
            "originalIndex": 0,
            "track": {
                "id": "38633712",
                "title": "Группа крови",
                "durationMs": 235100,
                "artists": [{"id": 41075, "name": "КИНО"}],
                "albums": [{"id": 5307396}]
            }
        });
        let parsed = track_entry_from_json(&entry).unwrap();
        assert_eq!(parsed.id, "38633712");
        assert_eq!(parsed.title, "Группа крови");
        assert_eq!(parsed.album_id.as_deref(), Some("5307396"));
        assert_eq!(parsed.duration, 235);
    }

    #[test]
    fn parses_wave_sequence_with_nested_track() {
        let body = serde_json::json!({
            "result": {
                "radioSessionId": "session-1",
                "batchId": "batch-1",
                "sequence": [
                    {"type": "track", "track": {
                        "id": "1", "title": "Первый", "durationMs": 61000,
                        "artists": [{"name": "Артист"}],
                        "albums": [{"id": 7}]
                    }},
                    {"id": "2", "title": "Второй", "durationMs": 120000,
                     "artists": [{"name": "Артист"}], "albums": [{"id": 8}]}
                ]
            }
        });
        let batch = WaveBatch::from_response(&body);
        assert_eq!(batch.tracks.len(), 2);
        assert_eq!(batch.session_id.as_deref(), Some("session-1"));
        assert_eq!(batch.queue, vec!["1:7", "2:8"]);
    }
}
