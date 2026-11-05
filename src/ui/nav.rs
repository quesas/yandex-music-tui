use crate::app::{App, Screen};
use crate::theme::Palette;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let p = app.palette();
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(p.border))
        .title_style(Style::new().fg(p.accent).add_modifier(Modifier::BOLD))
        .title(" Меню ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let [top, bottom] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .areas(inner);

    let width = inner.width as usize;
    let mut lines = vec![
        Line::from(Span::styled(
            " Яндекс Музыка",
            Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let active = match app.screen {
        Screen::Wave => Some(0),
        Screen::Library => Some(1),
        Screen::Search | Screen::TrackList => Some(2),
        Screen::Settings => Some(3),
        Screen::Login => None,
    };
    let items = [
        ("1", "Волна"),
        ("2", "Библиотека"),
        ("3", "Поиск"),
        ("4", "Настройки"),
    ];
    for (index, (key, label)) in items.iter().enumerate() {
        lines.push(nav_item(key, label, active == Some(index), width, p));
    }

    let account = app.account.clone().unwrap_or_default();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Аккаунт",
        Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!(
            " {}",
            if account.nickname.is_empty() {
                "—"
            } else {
                account.nickname.as_str()
            }
        ),
        Style::new().fg(p.fg),
    )));
    lines.push(Line::from(vec![
        Span::styled(" Плюс: ", Style::new().fg(p.dim)),
        Span::styled(
            if account.plus { "да" } else { "нет" },
            Style::new().fg(p.fg),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        format!(
            " {} · {}%",
            app.config.audio_quality.label(),
            app.player.engine.volume
        ),
        Style::new().fg(p.dim),
    )));

    f.render_widget(Paragraph::new(lines), top);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " ? — подсказки",
            Style::new().fg(p.dim),
        ))),
        bottom,
    );
}

fn nav_item(key: &str, label: &str, active: bool, width: usize, p: Palette) -> Line<'static> {
    let prefix = if active { " ▸ " } else { "   " };
    let used = prefix.chars().count() + key.chars().count() + 1 + label.chars().count();
    let padding = width.saturating_sub(used);
    let style = if active {
        Style::new()
            .fg(p.accent_fg)
            .bg(p.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(p.fg)
    };
    let key_style = if active {
        style
    } else {
        Style::new().fg(p.dim)
    };
    Line::from(vec![
        Span::styled(prefix.to_string(), style),
        Span::styled(format!("{key} "), key_style),
        Span::styled(label.to_string(), style),
        Span::styled(" ".repeat(padding), style),
    ])
}
