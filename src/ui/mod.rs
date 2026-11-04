pub mod content;
pub mod login;
pub mod player_bar;

use crate::app::{App, LibraryTab, Screen};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

/// Разметка: заголовок с разделами, контент + статус, плеер-бар, статус и подсказки.
pub fn draw(f: &mut Frame, app: &mut App) {
    let palette = app.palette();
    f.render_widget(Block::new().style(palette.base()), f.area());

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),                                         // заголовок
            Constraint::Min(5),                                            // контент
            Constraint::Length(3),                                         // плеер-бар
            Constraint::Length(if app.config.show_hints { 2 } else { 1 }), // статус + подсказки
        ])
        .split(f.area());

    draw_header(f, app, main[0]);
    if app.screen == Screen::Login {
        login::draw(f, app, main[1]);
    } else {
        content::draw(f, app, main[1]);
    }
    player_bar::draw(f, app, main[2]);
    draw_status(f, app, main[3]);
}

fn draw_header(f: &mut Frame, app: &App, area: ratatui::prelude::Rect) {
    let palette = app.palette();
    let active = match app.screen {
        Screen::Wave => 0,
        Screen::Library => 1,
        Screen::Search | Screen::TrackList => 2,
        Screen::Settings => 3,
        Screen::Login => usize::MAX,
    };
    let tab = |index: usize, label: &str| {
        if index == active {
            Span::styled(
                format!(" {label} "),
                Style::new()
                    .fg(palette.accent_fg)
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(format!(" {label} "), Style::new().fg(palette.dim))
        }
    };

    let tabs = Line::from(vec![
        Span::styled(
            "Яндекс Музыка ",
            Style::new().fg(palette.fg).add_modifier(Modifier::BOLD),
        ),
        tab(0, "1 Волна"),
        Span::raw(" "),
        tab(1, "2 Библиотека"),
        Span::raw(" "),
        tab(2, "3 Поиск"),
        Span::raw(" "),
        tab(3, "4 Настройки"),
    ]);

    let account = app.account.clone().unwrap_or_default();
    let library = match app.library_tab {
        LibraryTab::Tracks => "треки",
        LibraryTab::Artists => "артисты",
        LibraryTab::Playlists => "плейлисты",
    };
    let source = match app.screen {
        Screen::Library => format!("библиотека: {library}"),
        Screen::Settings => "настройки".to_string(),
        _ => app.track_source(),
    };
    let line2 = format!(
        " {} · Плюс: {} · {} · громкость {}% · {}",
        account.nickname,
        if account.plus { "да" } else { "нет" },
        app.config.audio_quality.label(),
        app.player.engine.volume,
        source
    );

    f.render_widget(
        Paragraph::new(vec![
            tabs,
            Line::from(Span::styled(line2, Style::new().fg(palette.dim))),
        ]),
        area,
    );
}

fn draw_status(f: &mut Frame, app: &App, area: ratatui::prelude::Rect) {
    let palette = app.palette();
    let [status_area, hints_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .areas(area);

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
        status_area,
    );

    if !app.config.show_hints {
        return;
    }

    let hints = match app.screen {
        Screen::Login if app.login_token_editing => " Enter проверить · Esc отмена · q выход ",
        Screen::Login => {
            " Enter открыть вход в браузере · t токен · Esc поиск без входа · 4 настройки · q выход "
        }
        Screen::Wave => {
            " ↑/↓ выбор · Enter играть · f лайк · d дизлайк · ␣ пауза · +/- громкость · m mute · r обновить · 2 библиотека · 3 поиск · 4 настройки "
        }
        Screen::Library => {
            " Tab раздел · ↑/↓ выбор · Enter играть · f лайк · d дизлайк · +/- громкость · m mute · r обновить · 4 настройки · Esc назад "
        }
        Screen::Search if app.search.editing => {
            " Введите запрос · Enter искать · Tab фильтр · Esc отмена "
        }
        Screen::Search => {
            " ↑/↓ выбор · Enter открыть/играть · f лайк · d дизлайк · Tab фильтр · / новый запрос · +/- громкость · 4 настройки · Esc назад "
        }
        Screen::TrackList => {
            " ↑/↓ выбор · Enter играть · f лайк · d дизлайк · +/- громкость · m mute · Esc назад · r обновить · 4 настройки "
        }
        Screen::Settings => " ↑/↓ выбор · Enter/←/→ изменить · Esc назад · q выход ",
    };
    f.render_widget(
        Paragraph::new(Span::styled(hints, Style::new().fg(palette.dim))),
        hints_area,
    );
}
