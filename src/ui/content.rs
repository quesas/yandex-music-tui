use crate::api::{Artist, Playlist, Track};
use crate::app::{App, LibraryTab, Screen};
use ratatui::Frame;
use ratatui::prelude::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    match app.screen {
        Screen::Search => draw_search(f, app, area),
        Screen::Library => draw_library(f, app, area),
        Screen::Wave => draw_wave(f, app, area),
        Screen::TrackList => draw_track_list(f, app, area),
        Screen::Settings => draw_settings(f, app, area),
        Screen::WaveSetup => draw_wave_setup(f, app, area),
        Screen::Login => {}
    }
}

fn panel(title: String, p: crate::theme::Palette) -> Block<'static> {
    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(p.border))
        .title_style(Style::new().fg(p.accent).add_modifier(Modifier::BOLD))
        .title(title)
}

fn draw_wave(f: &mut Frame, app: &mut App, area: Rect) {
    let p = app.palette();
    let revealed = app.row_count();
    let title = if area.width >= 60 {
        format!(
            " Моя волна · {} из {} · w — настроить ",
            revealed,
            app.wave.len()
        )
    } else {
        format!(" Моя волна · {} из {} ", revealed, app.wave.len())
    };
    let items = track_items(app, &app.wave[..revealed]);
    let list = List::new(items)
        .block(panel(title, p))
        .highlight_style(Style::new().bg(p.highlight_bg).fg(p.highlight_fg))
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_library(f: &mut Frame, app: &mut App, area: Rect) {
    let p = app.palette();
    let title = format!(
        " Библиотека · {} · {} ",
        app.library_tab.label(),
        app.row_count()
    );

    let items = match app.library_tab {
        LibraryTab::Tracks => track_items(app, &app.library_tracks),
        LibraryTab::Artists => artist_items(app, &app.library_artists),
        LibraryTab::Playlists => playlist_items(app, &app.library_playlists),
    };

    let list = List::new(items)
        .block(panel(title, p))
        .highlight_style(Style::new().bg(p.highlight_bg).fg(p.highlight_fg))
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_settings(f: &mut Frame, app: &mut App, area: Rect) {
    let p = app.palette();
    let items = vec![
        setting_item(0, "Тема интерфейса", app.config.theme.label(), p),
        setting_item(
            1,
            "Качество по умолчанию",
            app.config.audio_quality.label(),
            p,
        ),
        setting_item(2, "Настройки Моей волны", "w", p),
    ];
    let list = List::new(items)
        .block(panel(" Настройки ".to_string(), p))
        .highlight_style(Style::new().bg(p.highlight_bg).fg(p.highlight_fg))
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_wave_setup(f: &mut Frame, app: &mut App, area: Rect) {
    let p = app.palette();
    let items = match &app.wave_settings {
        Some(settings) => {
            let mut items = vec![setting_item(
                0,
                "Волна / занятие",
                &wave_context_label(app),
                p,
            )];
            for (index, group) in settings.groups.iter().enumerate() {
                items.push(setting_item(
                    index + 1,
                    &group.name,
                    &wave_group_value_label(app, group),
                    p,
                ));
            }
            items.push(ListItem::new(Line::from(vec![
                Span::styled("     ", Style::new()),
                Span::styled(
                    "Применить и перезапустить волну",
                    Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
                ),
            ])));
            items
        }
        None => vec![ListItem::new(Line::from(Span::styled(
            " Загружаем настройки волны...",
            Style::new().fg(p.dim),
        )))],
    };
    let list = List::new(items)
        .block(panel(" Настройки Моей волны ".to_string(), p))
        .highlight_style(Style::new().bg(p.highlight_bg).fg(p.highlight_fg))
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn wave_context_label(app: &App) -> String {
    let Some(seed) = app.config.wave_context.as_deref() else {
        return "Моя волна".to_string();
    };
    app.wave_settings
        .as_ref()
        .and_then(|settings| {
            settings
                .contexts
                .iter()
                .find(|context| context.seed == seed)
        })
        .map(|context| context.name.clone())
        .unwrap_or_else(|| seed.to_string())
}

fn wave_group_value_label(app: &App, group: &crate::api::WaveSettingGroup) -> String {
    let seed = app
        .config
        .wave_settings
        .get(&group.key)
        .map(String::as_str)
        .filter(|seed| !seed.is_empty());
    match seed {
        Some(seed) => group
            .values
            .iter()
            .find(|value| value.seed.as_deref() == Some(seed))
            .map(|value| value.name.clone())
            .unwrap_or_else(|| seed.to_string()),
        None => group
            .values
            .iter()
            .find(|value| value.seed.is_none())
            .map(|value| value.name.clone())
            .unwrap_or_else(|| "Любое".to_string()),
    }
}

fn setting_item(
    index: usize,
    label: &str,
    value: &str,
    p: crate::theme::Palette,
) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(format!(" {:>2}. ", index + 1), Style::new().fg(p.dim)),
        Span::styled(
            label.to_string(),
            Style::new().fg(p.fg).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(value.to_string(), Style::new().fg(p.accent)),
    ]))
}

fn draw_search(f: &mut Frame, app: &mut App, area: Rect) {
    let p = app.palette();
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
        Style::new().fg(p.warning)
    } else {
        Style::new().fg(p.fg)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " Поиск: ",
                Style::new().fg(p.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(input, input_style),
        ]))
        .block(panel(
            format!(" фильтр: {} (Tab) ", app.search.filter.label()),
            p,
        )),
        input_area,
    );

    let title = format!(
        " Результаты · {} · {} ",
        app.search.filter.label(),
        app.search.results.len()
    );
    let items = search_items(&app.search.results, app);
    let list = List::new(items)
        .block(panel(title, p))
        .highlight_style(Style::new().bg(p.highlight_bg).fg(p.highlight_fg))
        .highlight_symbol("");
    f.render_stateful_widget(list, list_area, &mut app.list_state);
}

fn draw_track_list(f: &mut Frame, app: &mut App, area: Rect) {
    let p = app.palette();
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
        .block(panel(title, p))
        .highlight_style(Style::new().bg(p.highlight_bg).fg(p.highlight_fg))
        .highlight_symbol("");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn track_items(app: &App, tracks: &[Track]) -> Vec<ListItem<'static>> {
    let p = app.palette();
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
                Style::new().fg(p.accent).add_modifier(Modifier::BOLD)
            } else if !track.available {
                Style::new().fg(p.dim)
            } else {
                Style::new().fg(p.fg)
            };
            let mut spans = vec![
                Span::styled(marker.to_string(), style),
                Span::styled(format!("{:>3} ", i + 1), Style::new().fg(p.dim)),
                Span::styled(track.title.clone(), style),
                Span::styled(format!(" — {}", track.artist), Style::new().fg(p.dim)),
                Span::styled(
                    format!("  {}:{:02}", track.duration / 60, track.duration % 60),
                    Style::new().fg(p.dim),
                ),
            ];
            if app.is_liked(track) {
                spans.push(Span::styled("  ♥", Style::new().fg(p.like)));
            } else if app.is_disliked(track) {
                spans.push(Span::styled("  ✗", Style::new().fg(p.dim)));
            }
            if !track.available {
                spans.push(Span::styled("  ✕", Style::new().fg(p.danger)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect()
}

fn artist_items(app: &App, artists: &[Artist]) -> Vec<ListItem<'static>> {
    let p = app.palette();
    artists
        .iter()
        .enumerate()
        .map(|(i, artist)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:>3} ", i + 1), Style::new().fg(p.dim)),
                Span::styled(
                    artist.name.clone(),
                    Style::new().fg(p.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {}", artist.subtitle()), Style::new().fg(p.dim)),
            ]))
        })
        .collect()
}

fn playlist_items(app: &App, playlists: &[Playlist]) -> Vec<ListItem<'static>> {
    let p = app.palette();
    playlists
        .iter()
        .enumerate()
        .map(|(i, playlist)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:>3} ", i + 1), Style::new().fg(p.dim)),
                Span::styled(
                    playlist.title.clone(),
                    Style::new().fg(p.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {}", playlist.subtitle()), Style::new().fg(p.dim)),
            ]))
        })
        .collect()
}

fn search_items(items: &[crate::api::SearchItem], app: &App) -> Vec<ListItem<'static>> {
    let p = app.palette();
    items
        .iter()
        .map(|item| {
            let tag_style = match item {
                crate::api::SearchItem::Track(_) => Style::new().fg(p.accent),
                crate::api::SearchItem::Artist(_) => Style::new().fg(p.tag_artist),
                crate::api::SearchItem::Album(_) => Style::new().fg(p.tag_album),
                crate::api::SearchItem::Playlist(_) => Style::new().fg(p.tag_playlist),
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
                Style::new().fg(p.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(p.fg).add_modifier(Modifier::BOLD)
            };
            let mut spans = vec![
                Span::styled(format!("[{:>2}] ", item.tag()), tag_style),
                Span::styled(item.title().to_string(), title_style),
                Span::styled(format!("  {}", item.subtitle()), Style::new().fg(p.dim)),
            ];
            if let crate::api::SearchItem::Track(track) = item {
                if app.is_liked(track) {
                    spans.push(Span::styled("  ♥", Style::new().fg(p.like)));
                } else if app.is_disliked(track) {
                    spans.push(Span::styled("  ✗", Style::new().fg(p.dim)));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect()
}
