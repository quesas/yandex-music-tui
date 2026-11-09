use crate::app::App;
use crate::cover::CoverImage;
use crate::theme::Palette;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let p = app.palette();
    let title = if area.width >= 26 {
        " Текст · t — закрыть "
    } else {
        " Текст "
    };
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(p.border))
        .title_style(Style::new().fg(p.accent).add_modifier(Modifier::BOLD))
        .title(title)
        .style(p.base());
    let inner = block.inner(area);
    f.render_widget(block, area);

    app.lyrics_view_height = 0;
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let mut y = inner.y;
    let mut remaining = inner.height;

    if let Some(image) = app.cover.image.as_ref() {
        let max_h = (inner.height.saturating_sub(5) * 48 / 100).clamp(3, 16);
        let (cover_width, cover_height) = cover_size(image, inner.width, max_h);
        if cover_height > 0 && cover_height <= remaining {
            let cover_area = Rect {
                x: inner.x + inner.width.saturating_sub(cover_width) / 2,
                y,
                width: cover_width,
                height: cover_height,
            };
            render_cover(f, image, cover_area, p);
            y = y.saturating_add(cover_height);
            remaining = remaining.saturating_sub(cover_height);
        }
    } else if app.cover.loading {
        let rows = 2.min(remaining);
        f.render_widget(
            Paragraph::new(" Загружаем обложку…")
                .alignment(Alignment::Center)
                .style(Style::new().fg(p.dim)),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: rows,
            },
        );
        y = y.saturating_add(rows);
        remaining = remaining.saturating_sub(rows);
    }

    if remaining == 0 {
        return;
    }

    if let Some(track) = app.player.current_track() {
        let label = fit(
            &format!("{} — {}", track.artist, track.title),
            inner.width as usize,
        );
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                label,
                Style::new().fg(p.fg).add_modifier(Modifier::BOLD),
            )))
            .alignment(Alignment::Center),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y = y.saturating_add(1);
        remaining = remaining.saturating_sub(1);

        if remaining > 0 {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "─".repeat(inner.width as usize),
                    Style::new().fg(p.border),
                )))
                .alignment(Alignment::Center),
                Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                },
            );
            y = y.saturating_add(1);
            remaining = remaining.saturating_sub(1);
        }
    }

    if remaining == 0 {
        return;
    }

    let body = Rect {
        x: inner.x,
        y,
        width: inner.width,
        height: remaining,
    };

    if app.player.current_track().is_none() {
        render_message(f, body, "Ничего не играет", p);
        return;
    }

    if app.lyrics.loading {
        render_message(f, body, "Загружаем текст…", p);
        return;
    }

    if let Some(error) = app.lyrics.error.as_deref() {
        let message = if error.is_empty() || error.len() > 80 {
            "Текст песни недоступен"
        } else {
            error
        };
        render_message(f, body, message, p);
        return;
    }

    if app.lyrics.lyrics.lines.is_empty() {
        render_message(f, body, "Текст песни не найден", p);
        return;
    }

    app.lyrics_view_height = body.height;
    render_lyrics(f, app, body, p);
}

fn render_message(f: &mut Frame, area: Rect, text: &str, p: Palette) {
    let top = area.height / 2;
    let rect = Rect {
        x: area.x,
        y: area.y.saturating_add(top),
        width: area.width,
        height: 1.min(area.height),
    };
    f.render_widget(
        Paragraph::new(fit(text, area.width as usize))
            .alignment(Alignment::Center)
            .style(Style::new().fg(p.dim)),
        rect,
    );
}

fn cover_size(image: &CoverImage, max_width: u16, max_height: u16) -> (u16, u16) {
    if max_width == 0 || max_height == 0 || image.width == 0 || image.height == 0 {
        return (0, 0);
    }
    let ratio = image.width as f32 / image.height as f32;
    let mut height = max_height;
    // Один символ — два вертикальных пикселя, поэтому ширина в клетках вдвое больше.
    let mut width = (2.0 * height as f32 * ratio).round() as u16;
    if width > max_width {
        width = max_width;
        height = ((width as f32 / (2.0 * ratio)).round() as u16).max(1);
    }
    (width.max(1).min(max_width), height.max(1).min(max_height))
}

fn render_cover(f: &mut Frame, image: &CoverImage, area: Rect, p: Palette) {
    let width = area.width as u32;
    let height = area.height as u32;
    if width == 0 || height == 0 {
        return;
    }

    let mut lines = Vec::with_capacity(area.height as usize);
    for row in 0..height {
        let mut spans = Vec::with_capacity(area.width as usize);
        for col in 0..width {
            let x = (col * image.width / width).min(image.width - 1);
            let top_y = ((row * 2) * image.height / (height * 2)).min(image.height - 1);
            let bottom_y = ((row * 2 + 1) * image.height / (height * 2)).min(image.height - 1);
            let top = tint(image.pixel(x, top_y), p);
            let bottom = tint(image.pixel(x, bottom_y), p);
            spans.push(Span::styled(
                "▀",
                Style::new().fg(rgb_color(top)).bg(rgb_color(bottom)),
            ));
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn render_lyrics(f: &mut Frame, app: &App, area: Rect, p: Palette) {
    let active = app.current_lyric_line();
    let width = area.width as usize;
    let lines: Vec<Line<'static>> = app
        .lyrics
        .lyrics
        .lines
        .iter()
        .enumerate()
        .map(|(index, lyric)| {
            let text = if lyric.text.trim().is_empty() {
                "♪".to_string()
            } else {
                lyric.text.clone()
            };
            let (prefix, style) = match active {
                Some(active) if index == active => {
                    ("▸ ", Style::new().fg(p.accent).add_modifier(Modifier::BOLD))
                }
                Some(active) if index.abs_diff(active) <= 1 => ("  ", Style::new().fg(p.fg)),
                Some(_) => ("  ", Style::new().fg(p.dim)),
                None => ("  ", Style::new().fg(p.fg)),
            };
            let text = fit(&text, width.saturating_sub(prefix.chars().count()));
            Line::from(vec![Span::styled(prefix, style), Span::styled(text, style)])
        })
        .collect();

    let max_scroll = lines.len().saturating_sub(area.height as usize);
    let scroll = (app.lyrics_scroll.round() as usize).min(max_scroll) as u16;
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .scroll((scroll, 0))
            .style(p.base()),
        area,
    );
}

fn fit(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if text.width() <= width {
        return text.to_string();
    }

    // Оставляем место под многоточие и не разрываем широкие символы (CJK).
    let limit = width - 1;
    let mut result = String::with_capacity(width);
    let mut used = 0;
    for ch in text.chars() {
        let char_width = ch.width().unwrap_or(0);
        if used + char_width > limit {
            break;
        }
        result.push(ch);
        used += char_width;
    }
    result.push('…');
    result
}

fn tint(pixel: [u8; 3], p: Palette) -> [u8; 3] {
    let luma =
        (0.2126 * pixel[0] as f32 + 0.7152 * pixel[1] as f32 + 0.0722 * pixel[2] as f32) / 255.0;
    let amount = (0.16 + 0.84 * luma.powf(0.55)).clamp(0.0, 1.0);
    mix(rgb(p.bg), rgb(p.accent), amount)
}

fn rgb(color: Color) -> [u8; 3] {
    match color {
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Black => [0, 0, 0],
        Color::White => [255, 255, 255],
        Color::Red => [255, 0, 0],
        Color::Green => [0, 255, 0],
        Color::Blue => [0, 0, 255],
        Color::Yellow => [255, 255, 0],
        Color::Magenta => [255, 0, 255],
        Color::Cyan => [0, 255, 255],
        Color::Gray => [128, 128, 128],
        Color::DarkGray => [64, 64, 64],
        Color::LightRed => [255, 128, 128],
        Color::LightGreen => [128, 255, 128],
        Color::LightBlue => [128, 128, 255],
        Color::LightYellow => [255, 255, 128],
        Color::LightMagenta => [255, 128, 255],
        Color::LightCyan => [128, 255, 255],
        Color::Indexed(index) => [index, index, index],
        Color::Reset => [0, 0, 0],
    }
}

fn rgb_color([r, g, b]: [u8; 3]) -> Color {
    Color::Rgb(r, g, b)
}

fn mix(a: [u8; 3], b: [u8; 3], amount: f32) -> [u8; 3] {
    let blend = |from: u8, to: u8| {
        (from as f32 + (to as f32 - from as f32) * amount)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    [blend(a[0], b[0]), blend(a[1], b[1]), blend(a[2], b[2])]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    fn image(width: u32, height: u32) -> CoverImage {
        CoverImage {
            width,
            height,
            pixels: vec![[128, 128, 128]; (width * height) as usize],
        }
    }

    #[test]
    fn keeps_cover_aspect_with_half_blocks() {
        assert_eq!(cover_size(&image(400, 400), 40, 20), (40, 20));
        assert_eq!(cover_size(&image(200, 400), 40, 20), (20, 20));
        assert_eq!(cover_size(&image(800, 200), 40, 20), (40, 5));
    }

    #[test]
    fn tint_uses_theme_accent() {
        let palette = Theme::RosePine.palette();
        assert_eq!(tint([255, 255, 255], palette), rgb(palette.accent));
        let dark = tint([0, 0, 0], palette);
        assert_ne!(dark, rgb(palette.bg));
    }

    #[test]
    fn truncates_long_lyrics_lines() {
        assert_eq!(fit("привет", 10), "привет");
        assert_eq!(fit("привет, мир", 7), "привет…");
        assert_eq!(fit("ああああ", 5), "ああ…");
    }
}
