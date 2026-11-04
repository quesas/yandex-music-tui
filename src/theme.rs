use ratatui::style::{Color, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    White,
    #[default]
    Black,
    TokyoNight,
    RosePine,
}

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Self::White => "Белая",
            Self::Black => "Чёрная",
            Self::TokyoNight => "TokyoNight",
            Self::RosePine => "RosePine",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::TokyoNight,
            Self::TokyoNight => Self::RosePine,
            Self::RosePine => Self::White,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::White => Self::RosePine,
            Self::Black => Self::White,
            Self::TokyoNight => Self::Black,
            Self::RosePine => Self::TokyoNight,
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            Self::White => Palette {
                bg: Color::Rgb(0xff, 0xff, 0xff),
                fg: Color::Rgb(0x1c, 0x1c, 0x1c),
                accent: Color::Rgb(0x09, 0x69, 0xda),
                accent_fg: Color::Rgb(0xff, 0xff, 0xff),
                highlight_bg: Color::Rgb(0xd0, 0xd7, 0xde),
                highlight_fg: Color::Rgb(0x1c, 0x1c, 0x1c),
                dim: Color::Rgb(0x57, 0x60, 0x6a),
                warning: Color::Rgb(0x9a, 0x67, 0x00),
                danger: Color::Rgb(0xcf, 0x22, 0x2e),
                like: Color::Rgb(0xcf, 0x22, 0x2e),
                tag_artist: Color::Rgb(0x82, 0x50, 0xdf),
                tag_album: Color::Rgb(0x09, 0x69, 0xda),
                tag_playlist: Color::Rgb(0x1a, 0x7f, 0x37),
                border: Color::Rgb(0xd0, 0xd7, 0xde),
            },
            Self::Black => Palette {
                bg: Color::Rgb(0x00, 0x00, 0x00),
                fg: Color::Rgb(0xe6, 0xe6, 0xe6),
                accent: Color::Rgb(0x00, 0xd7, 0xff),
                accent_fg: Color::Rgb(0x00, 0x00, 0x00),
                highlight_bg: Color::Rgb(0x33, 0x33, 0x33),
                highlight_fg: Color::Rgb(0xff, 0xff, 0xff),
                dim: Color::Rgb(0x8a, 0x8a, 0x8a),
                warning: Color::Rgb(0xff, 0xd7, 0x5f),
                danger: Color::Rgb(0xff, 0x5f, 0x5f),
                like: Color::Rgb(0xff, 0x5f, 0x5f),
                tag_artist: Color::Rgb(0xd7, 0x87, 0xff),
                tag_album: Color::Rgb(0x5f, 0xaf, 0xff),
                tag_playlist: Color::Rgb(0x5f, 0xd7, 0x5f),
                border: Color::Rgb(0x4a, 0x4a, 0x4a),
            },
            Self::TokyoNight => Palette {
                bg: Color::Rgb(0x1a, 0x1b, 0x26),
                fg: Color::Rgb(0xc0, 0xca, 0xf5),
                accent: Color::Rgb(0x7a, 0xa2, 0xf7),
                accent_fg: Color::Rgb(0x1a, 0x1b, 0x26),
                highlight_bg: Color::Rgb(0x29, 0x2e, 0x42),
                highlight_fg: Color::Rgb(0xc0, 0xca, 0xf5),
                dim: Color::Rgb(0x56, 0x5f, 0x89),
                warning: Color::Rgb(0xe0, 0xaf, 0x68),
                danger: Color::Rgb(0xf7, 0x76, 0x8e),
                like: Color::Rgb(0xff, 0x9e, 0x64),
                tag_artist: Color::Rgb(0xbb, 0x9a, 0xf7),
                tag_album: Color::Rgb(0x7a, 0xa2, 0xf7),
                tag_playlist: Color::Rgb(0x9e, 0xce, 0x6a),
                border: Color::Rgb(0x3b, 0x42, 0x61),
            },
            Self::RosePine => Palette {
                bg: Color::Rgb(0x19, 0x17, 0x24),
                fg: Color::Rgb(0xe0, 0xde, 0xf4),
                accent: Color::Rgb(0xc4, 0xa7, 0xe7),
                accent_fg: Color::Rgb(0x19, 0x17, 0x24),
                highlight_bg: Color::Rgb(0x26, 0x23, 0x3a),
                highlight_fg: Color::Rgb(0xe0, 0xde, 0xf4),
                dim: Color::Rgb(0x6e, 0x6a, 0x86),
                warning: Color::Rgb(0xf6, 0xc1, 0x77),
                danger: Color::Rgb(0xeb, 0x6f, 0x92),
                like: Color::Rgb(0xeb, 0xbc, 0xba),
                tag_artist: Color::Rgb(0xc4, 0xa7, 0xe7),
                tag_album: Color::Rgb(0x9c, 0xcf, 0xd8),
                tag_playlist: Color::Rgb(0x31, 0x74, 0x8f),
                border: Color::Rgb(0x40, 0x3d, 0x52),
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub accent_fg: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub dim: Color,
    pub warning: Color,
    pub danger: Color,
    pub like: Color,
    pub tag_artist: Color,
    pub tag_album: Color,
    pub tag_playlist: Color,
    pub border: Color,
}

impl Palette {
    pub fn base(self) -> Style {
        Style::new().fg(self.fg).bg(self.bg)
    }
}
