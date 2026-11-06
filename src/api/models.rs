use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub album_id: Option<String>,
    pub title: String,
    pub artist: String,

    pub duration: u64,

    pub available: bool,
}

impl Track {
    pub fn full_id(&self) -> String {
        match &self.album_id {
            Some(album) => format!("{}:{}", self.id, album),
            None => self.id.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub tracks_count: u64,
    pub albums_count: u64,
}

impl Artist {
    pub fn subtitle(&self) -> String {
        format!(
            "{} треков · {} альбомов",
            self.tracks_count, self.albums_count
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub year: Option<i64>,
    pub track_count: u64,
}

impl Album {
    pub fn subtitle(&self) -> String {
        let mut parts = Vec::new();
        if !self.artist.is_empty() {
            parts.push(self.artist.clone());
        }
        if let Some(year) = self.year {
            parts.push(year.to_string());
        }
        parts.push(format!("{} треков", self.track_count));
        parts.join(" · ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist {
    pub uid: String,
    pub kind: String,
    pub title: String,
    pub track_count: u64,
}

impl Playlist {
    pub fn subtitle(&self) -> String {
        format!("{} треков · плейлист {}", self.track_count, self.uid)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchItem {
    Track(Track),
    Artist(Artist),
    Album(Album),
    Playlist(Playlist),
}

impl SearchItem {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Track(_) => "T",
            Self::Artist(_) => "A",
            Self::Album(_) => "AL",
            Self::Playlist(_) => "P",
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Track(t) => &t.title,
            Self::Artist(a) => &a.name,
            Self::Album(a) => &a.title,
            Self::Playlist(p) => &p.title,
        }
    }

    pub fn subtitle(&self) -> String {
        match self {
            Self::Track(t) => format!("{} · {}:{:02}", t.artist, t.duration / 60, t.duration % 60),
            Self::Artist(a) => a.subtitle(),
            Self::Album(a) => a.subtitle(),
            Self::Playlist(p) => p.subtitle(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFilter {
    All,
    Tracks,
    Artists,
    Albums,
    Playlists,
}

impl SearchFilter {
    pub fn api_value(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Tracks => "track",
            Self::Artists => "artist",
            Self::Albums => "album",
            Self::Playlists => "playlist",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "всё",
            Self::Tracks => "треки",
            Self::Artists => "артисты",
            Self::Albums => "альбомы",
            Self::Playlists => "плейлисты",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Tracks,
            Self::Tracks => Self::Artists,
            Self::Artists => Self::Albums,
            Self::Albums => Self::Playlists,
            Self::Playlists => Self::All,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::All => Self::Playlists,
            Self::Tracks => Self::All,
            Self::Artists => Self::Tracks,
            Self::Albums => Self::Artists,
            Self::Playlists => Self::Albums,
        }
    }
}
