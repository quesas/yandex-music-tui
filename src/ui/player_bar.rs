use crate::app::App;
use ratatui::Frame;
use ratatui::prelude::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let p = app.palette();
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(p.border))
        .title_style(Style::new().fg(p.accent).add_modifier(Modifier::BOLD))
        .title(" Сейчас играет ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 {
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let state = match app.player.engine.state {
        crate::player::engine::PlayState::Playing => "▶",
        crate::player::engine::PlayState::Paused => "⏸",
        crate::player::engine::PlayState::Loading => "…",
        crate::player::engine::PlayState::Stopped => "■",
    };
    let title = if app.player.now_playing().is_empty() {
        "Ничего не играет".to_string()
    } else {
        app.player.now_playing()
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {state} "),
                Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(title, Style::new().fg(p.fg).add_modifier(Modifier::BOLD)),
        ])),
        rows[0],
    );

    let (pos, dur) = (app.player.engine.position, app.player.engine.duration);
    let ratio = if dur > 0.0 { pos / dur } else { 0.0 };
    let gauge = Gauge::default()
        .ratio(ratio.clamp(0.0, 1.0))
        .gauge_style(Style::new().fg(p.accent).bg(p.border))
        .label(format!(
            "{:0>2}:{:0>2} / {:0>2}:{:0>2}",
            pos as u64 / 60,
            pos as u64 % 60,
            dur as u64 / 60,
            dur as u64 % 60,
        ));
    f.render_widget(gauge, rows[1]);

    let vol = app.player.engine.volume;
    let filled = (vol / 10) as usize;
    let vol_bars = format!("{}{}", "▮".repeat(filled), "▯".repeat(10 - filled));
    let repeat = app.player.repeat;
    let repeat_style = if repeat == crate::player::RepeatMode::Off {
        Style::new().fg(p.dim)
    } else {
        Style::new().fg(p.accent).add_modifier(Modifier::BOLD)
    };
    let mut volume = vec![
        Span::styled(" громкость ", Style::new().fg(p.dim)),
        Span::styled(
            format!("{vol:>3}% "),
            Style::new().fg(p.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(vol_bars, Style::new().fg(p.accent)),
        Span::styled(
            format!("   {} ", app.config.audio_quality.label()),
            Style::new().fg(p.dim),
        ),
        Span::styled("  повтор: ", Style::new().fg(p.dim)),
        Span::styled(repeat.label(), repeat_style),
    ];
    if vol == 0 {
        volume.push(Span::styled("· mute", Style::new().fg(p.warning)));
    }
    f.render_widget(Paragraph::new(Line::from(volume)), rows[2]);
}
