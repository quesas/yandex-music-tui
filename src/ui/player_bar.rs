use crate::app::App;
use ratatui::Frame;
use ratatui::prelude::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};

pub fn draw(f: &mut Frame, app: &App, area: ratatui::prelude::Rect) {
    let p = app.palette();
    let [left, right] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(1)])
        .areas(area);

    // --- Первая строка: исполнитель — трек, справа время ---
    let title = if app.player.now_playing().is_empty() {
        " Ничего не играет ".to_string()
    } else {
        format!(" {} ", app.player.now_playing())
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title,
            Style::new().fg(p.fg).add_modifier(Modifier::BOLD),
        ))),
        left,
    );

    // --- Вторая строка: прогресс-бар ---
    let (pos, dur) = (app.player.engine.position, app.player.engine.duration);
    let ratio = if dur > 0.0 { pos / dur } else { 0.0 };
    let gauge = Gauge::default()
        .ratio(ratio.clamp(0.0, 1.0))
        .gauge_style(Style::new().fg(p.accent).bg(p.bg))
        .label(format!(
            "{:0>2}:{:0>2} / {:0>2}:{:0>2}",
            pos as u64 / 60,
            pos as u64 % 60,
            dur as u64 / 60,
            dur as u64 % 60,
        ));
    f.render_widget(gauge, left);

    // --- Третья строка: громкость и статус ---
    let vol = app.player.engine.volume;
    let state = match app.player.engine.state {
        crate::player::engine::PlayState::Playing => "▶",
        crate::player::engine::PlayState::Paused => "⏸",
        crate::player::engine::PlayState::Loading => "…",
        crate::player::engine::PlayState::Stopped => "■",
    };
    let filled = (vol / 10) as usize;
    let vol_bars = format!("{}{}", "▮".repeat(filled), "▯".repeat(10 - filled));
    let muted = if vol == 0 { "  mute" } else { "" };
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{}  громкость {:0>3}% {}{}", state, vol, vol_bars, muted),
            Style::new().fg(p.fg),
        )),
        right,
    );
}
