pub mod confirm;
pub mod content;
pub mod help;
pub mod login;
pub mod nav;
pub mod player_bar;

use crate::app::{App, Screen};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

pub fn draw(f: &mut Frame, app: &mut App) {
    let palette = app.palette();
    f.render_widget(Block::new().style(palette.base()), f.area());

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(6),
            Constraint::Length(5),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_header(f, app, main[0]);

    let content_area = if main[1].width >= 68 {
        let nav_width = (main[1].width / 4).clamp(20, 28);
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(nav_width), Constraint::Min(20)])
            .split(main[1]);
        nav::draw(f, app, columns[0]);
        columns[1]
    } else {
        main[1]
    };

    if app.screen == Screen::Login {
        login::draw(f, app, content_area);
    } else {
        content::draw(f, app, content_area);
    }
    player_bar::draw(f, app, main[2]);
    draw_status(f, app, main[3]);

    if app.show_help {
        help::draw(f, app);
    }
    if app.confirm.is_some() {
        confirm::draw(f, app);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: ratatui::prelude::Rect) {
    let palette = app.palette();
    let account = app.account.clone().unwrap_or_default();

    let mut spans = vec![
        Span::styled(
            " Яндекс Музыка",
            Style::new().fg(palette.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ·  {}", screen_name(app.screen)),
            Style::new().fg(palette.fg),
        ),
    ];

    if area.width >= 72 {
        spans.push(Span::styled(
            format!(
                "   {} · Плюс: {} · {} · {}%",
                account.nickname,
                if account.plus { "да" } else { "нет" },
                app.config.audio_quality.label(),
                app.player.engine.volume
            ),
            Style::new().fg(palette.dim),
        ));
    }

    if area.width >= 44 {
        spans.push(Span::styled(
            "   ? — подсказки",
            Style::new().fg(palette.dim),
        ));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn screen_name(screen: Screen) -> &'static str {
    match screen {
        Screen::Login => "Вход",
        Screen::Wave => "Волна",
        Screen::Library => "Библиотека",
        Screen::Search => "Поиск",
        Screen::TrackList => "Треки",
        Screen::Settings => "Настройки",
        Screen::WaveSetup => "Настройки волны",
    }
}

fn draw_status(f: &mut Frame, app: &App, area: ratatui::prelude::Rect) {
    let palette = app.palette();
    let status = if app.status_message.is_empty() {
        "готово".to_string()
    } else {
        app.status_message.clone()
    };
    f.render_widget(
        Paragraph::new(Span::styled(
            format!(" {status}"),
            Style::new().fg(palette.dim),
        )),
        area,
    );
}
