use crate::api::{Artist, Playlist, Track};
use crate::app::{App, LibraryTab, Screen};
use ratatui::Frame;
use ratatui::prelude::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let [content_area, side_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(45), Constraint::Length(30)])
        .areas(area);

    match app.screen {
        Screen::Search => draw_search(f, app, content_area),
        Screen::Library => draw_library(f, app, content_area),
        Screen::Wave => draw_wave(f, app, content_area),
        Screen::TrackList => draw_track_list(f, app, content_area),
        Screen::Login => {}
    }

    draw_sidebar(f, app, side_area);
}

fn draw_wave(f: &mut Frame, app: &mut App, area: Rect) {
    let title = format!(" Моя волна · {} треков ", app.wave.len());
    let items = track_items(app, &app.wave);
    let list = List::new(items)
        .block(Block::new().borders(Borders::ALL).title(title))
        .highlight_style(Style::new().bg(Color::DarkGray))
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_library(f: &mut Frame, app: &mut App, area: Rect) {
    let title = format!(
        " Библиотека · {} · {} ",
        app.library_tab.label(),
        app.row_count()
    );

    let items = match app.library_tab {
        LibraryTab::Tracks => track_items(app, &app.library_tracks),
        LibraryTab::Artists => artist_items(&app.library_artists),
        LibraryTab::Playlists => playlist_items(&app.library_playlists),
    };

    let list = List::new(items)
        .block(Block::new().borders(Borders::ALL).title(title))
        .highlight_style(Style::new().bg(Color::DarkGray))
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_search(f: &mut Frame, app: &mut App, area: Rect) {
    let [input_area, list_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .areas(area);

    let input = if app.search.editing {
        format!("{}█", app.search.input)
    } else if app.search.query.is_empty() {
        "нажмите / или 3, чтобы искать".to_string()
    } else {
        app.search.query.clone()
    };
    let input_style = if app.search.editing {
        Style::new().fg(Color::Yellow)
    } else {
        Style::new()
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Поиск: ", Style::new().add_modifier(Modifier::BOLD)),
            Span::styled(input, input_style),
        ]))
        .block(
            Block::new()
                .borders(Borders::ALL)
                .title(format!(" фильтр: {} (Tab) ", app.search.filter.label())),
        ),
        input_area,
    );

    let title = format!(
        " Результаты · {} · {} ",
        app.search.filter.label(),
        app.search.results.len()
    );
    let items = search_items(&app.search.results, app);
    let list = List::new(items)
        .block(Block::new().borders(Borders::ALL).title(title))
        .highlight_style(Style::new().bg(Color::DarkGray))
        .highlight_symbol("");
    f.render_stateful_widget(list, list_area, &mut app.list_state);
}

fn draw_track_list(f: &mut Frame, app: &mut App, area: Rect) {
    let title = match &app.track_list {
        Some(list) => format!(" {} · {} ", list.title, list.subtitle),
        None => " Треки ".to_string(),
    };
    let tracks = app
        .track_list
        .as_ref()
        .map(|list| list.tracks.as_slice())
        .unwrap_or(&[]);
    let items = track_items(app, tracks);
    let list = List::new(items)
        .block(Block::new().borders(Borders::ALL).title(title))
        .highlight_style(Style::new().bg(Color::DarkGray))
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn track_items(app: &App, tracks: &[Track]) -> Vec<ListItem<'static>> {
    tracks
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let current = app
                .player
                .current_track()
                .map(|t| t.id == track.id && t.album_id == track.album_id)
                .unwrap_or(false);
            let marker = if current {
                match app.player.engine.state {
                    crate::player::engine::PlayState::Playing => "▶ ",
                    crate::player::engine::PlayState::Paused => "⏸ ",
                    crate::player::engine::PlayState::Loading => "… ",
                    crate::player::engine::PlayState::Stopped => "  ",
                }
            } else {
                "  "
            };
            let style = if current {
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if !track.available {
                Style::new().fg(Color::DarkGray)
            } else {
                Style::new()
            };
            let mut spans = vec![
                Span::styled(marker.to_string(), style),
                Span::styled(format!("{:>3} ", i + 1), Style::new().dim()),
                Span::styled(track.title.clone(), style),
                Span::styled(format!(" — {}", track.artist), Style::new().dim()),
                Span::styled(
                    format!("  {}:{:02}", track.duration / 60, track.duration % 60),
                    Style::new().dim(),
                ),
            ];
            if app.is_liked(track) {
                spans.push(Span::styled("  ♥", Style::new().fg(Color::Red)));
            } else if app.is_disliked(track) {
                spans.push(Span::styled("  ✗", Style::new().fg(Color::DarkGray)));
            }
            if !track.available {
                spans.push(Span::styled("  ✕", Style::new().fg(Color::Red)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect()
}

fn artist_items(artists: &[Artist]) -> Vec<ListItem<'static>> {
    artists
        .iter()
        .enumerate()
        .map(|(i, artist)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:>3} ", i + 1), Style::new().dim()),
                Span::styled(
                    artist.name.clone(),
                    Style::new().add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {}", artist.subtitle()), Style::new().dim()),
            ]))
        })
        .collect()
}

fn playlist_items(playlists: &[Playlist]) -> Vec<ListItem<'static>> {
    playlists
        .iter()
        .enumerate()
        .map(|(i, playlist)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:>3} ", i + 1), Style::new().dim()),
                Span::styled(
                    playlist.title.clone(),
                    Style::new().add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {}", playlist.subtitle()), Style::new().dim()),
            ]))
        })
        .collect()
}

fn search_items(items: &[crate::api::SearchItem], app: &App) -> Vec<ListItem<'static>> {
    items
        .iter()
        .map(|item| {
            let tag_style = match item {
                crate::api::SearchItem::Track(_) => Style::new().fg(Color::Cyan),
                crate::api::SearchItem::Artist(_) => Style::new().fg(Color::Magenta),
                crate::api::SearchItem::Album(_) => Style::new().fg(Color::Blue),
                crate::api::SearchItem::Playlist(_) => Style::new().fg(Color::Green),
            };
            let current = match item {
                crate::api::SearchItem::Track(track) => app
                    .player
                    .current_track()
                    .map(|t| t.id == track.id && t.album_id == track.album_id)
                    .unwrap_or(false),
                _ => false,
            };
            let title_style = if current {
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::new().add_modifier(Modifier::BOLD)
            };
            let mut spans = vec![
                Span::styled(format!("[{:>2}] ", item.tag()), tag_style),
                Span::styled(item.title().to_string(), title_style),
                Span::styled(format!("  {}", item.subtitle()), Style::new().dim()),
            ];
            if let crate::api::SearchItem::Track(track) = item {
                if app.is_liked(track) {
                    spans.push(Span::styled("  ♥", Style::new().fg(Color::Red)));
                } else if app.is_disliked(track) {
                    spans.push(Span::styled("  ✗", Style::new().fg(Color::DarkGray)));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect()
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let account = app.account.clone().unwrap_or_default();
    let source = match app.screen {
        Screen::Wave => format!("Волна: {}", app.wave.len()),
        Screen::Library => format!("Библиотека: {}", app.row_count()),
        Screen::Search => format!("Поиск: {}", app.search.results.len()),
        Screen::TrackList => format!("Список: {}", app.row_count()),
        Screen::Login => "Вход".to_string(),
    };
    let lines = vec![
        Line::from(vec![
            Span::styled("Аккаунт: ", Style::new().dim()),
            Span::raw(account.nickname.clone()),
        ]),
        Line::from(vec![
            Span::styled("Плюс: ", Style::new().dim()),
            Span::raw(if account.plus {
                "активен"
            } else {
                "нет"
            }),
        ]),
        Line::from(vec![
            Span::styled("Качество: ", Style::new().dim()),
            Span::raw(app.config.audio_quality.label()),
        ]),
        Line::from(vec![
            Span::styled("Громкость: ", Style::new().dim()),
            Span::raw(format!("{}%", app.player.engine.volume)),
        ]),
        Line::from(vec![
            Span::styled("Токен: ", Style::new().dim()),
            Span::raw(if app.has_token() {
                "сохранён"
            } else {
                "нет"
            }),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            source,
            Style::new().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("1 волна", Style::new().dim())),
        Line::from(Span::styled("2 библиотека", Style::new().dim())),
        Line::from(Span::styled("3 поиск", Style::new().dim())),
        Line::from(Span::styled("Tab разделы", Style::new().dim())),
        Line::from(Span::styled("f лайк · d дизлайк", Style::new().dim())),
        Line::from(Span::styled("r обновить", Style::new().dim())),
        Line::from(Span::styled("o выйти", Style::new().dim())),
    ];
    f.render_widget(
        Paragraph::new(lines).block(Block::new().borders(Borders::ALL).title(" Статус ")),
        area,
    );
}
