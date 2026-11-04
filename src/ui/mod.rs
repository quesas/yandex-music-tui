pub mod content;
pub mod login;
pub mod player_bar;

use crate::app::{App, LibraryTab, Screen};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Разметка: заголовок с разделами, контент + статус, плеер-бар, статус и подсказки.
pub fn draw(f: &mut Frame, app: &mut App) {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // заголовок
            Constraint::Min(5),    // контент
            Constraint::Length(3), // плеер-бар
            Constraint::Length(2), // статус + подсказки
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
    let active = match app.screen {
        Screen::Wave => 0,
        Screen::Library => 1,
        Screen::Search | Screen::TrackList => 2,
        Screen::Login => usize::MAX,
    };
    let tab = |index: usize, label: &str| {
        if index == active {
            Span::styled(
                format!(" {label} "),
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(format!(" {label} "), Style::new().dim())
        }
    };

    let tabs = Line::from(vec![
        Span::styled("Яндекс Музыка ", Style::new().add_modifier(Modifier::BOLD)),
        tab(0, "1 Волна"),
        Span::raw(" "),
        tab(1, "2 Библиотека"),
        Span::raw(" "),
        tab(2, "3 Поиск"),
    ]);

    let account = app.account.clone().unwrap_or_default();
    let library = match app.library_tab {
        LibraryTab::Tracks => "треки",
        LibraryTab::Artists => "артисты",
        LibraryTab::Playlists => "плейлисты",
    };
    let line2 = format!(
        " {} · Плюс: {} · {} · громкость {}% · {}",
        account.nickname,
        if account.plus { "да" } else { "нет" },
        app.config.audio_quality.label(),
        app.player.engine.volume,
        if app.screen == Screen::Library {
            format!("библиотека: {library}")
        } else {
            app.track_source()
        }
    );

    f.render_widget(
        Paragraph::new(vec![
            tabs,
            Line::from(Span::styled(line2, Style::new().dim())),
        ]),
        area,
    );
}

fn draw_status(f: &mut Frame, app: &App, area: ratatui::prelude::Rect) {
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
        Paragraph::new(Span::styled(format!(" {status}"), Style::new().dim())),
        status_area,
    );

    let hints = match app.screen {
        Screen::Login if app.login_token_editing => " Enter проверить · Esc отмена · q выход ",
        Screen::Login => {
            " Enter открыть вход в браузере · t токен · Esc поиск без входа · q выход "
        }
        Screen::Wave => {
            " ↑/↓ выбор · Enter играть · f лайк · d дизлайк · ␣ пауза · +/- громкость · m mute · r обновить · 2 библиотека · 3 поиск "
        }
        Screen::Library => {
            " Tab раздел · ↑/↓ выбор · Enter играть · f лайк · d дизлайк · +/- громкость · m mute · r обновить · Esc назад "
        }
        Screen::Search if app.search.editing => {
            " Введите запрос · Enter искать · Tab фильтр · Esc отмена "
        }
        Screen::Search => {
            " ↑/↓ выбор · Enter открыть/играть · f лайк · d дизлайк · Tab фильтр · / новый запрос · +/- громкость · Esc назад "
        }
        Screen::TrackList => {
            " ↑/↓ выбор · Enter играть · f лайк · d дизлайк · +/- громкость · m mute · Esc назад · r обновить "
        }
    };
    f.render_widget(
        Paragraph::new(Span::styled(hints, Style::new().fg(Color::DarkGray))),
        hints_area,
    );
}
