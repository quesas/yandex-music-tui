use crate::app::App;
use ratatui::Frame;
use ratatui::prelude::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Length(14),
            Constraint::Percentage(20),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(vertical[1]);
    let panel = horizontal[1];

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Вход в Яндекс Музыку",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter  ", Style::new().fg(Color::Yellow)),
            Span::raw("открыть окно Яндекс OAuth (webview)"),
        ]),
        Line::from(vec![
            Span::styled("  t      ", Style::new().fg(Color::Yellow)),
            Span::raw("вставить OAuth-токен вручную"),
        ]),
        Line::from(vec![
            Span::styled("  Esc    ", Style::new().fg(Color::Yellow)),
            Span::raw("продолжить без входа (поиск и артисты)"),
        ]),
        Line::from(vec![
            Span::styled("  q      ", Style::new().fg(Color::Yellow)),
            Span::raw("выход"),
        ]),
        Line::from(""),
    ];

    if app.login_token_editing {
        let masked = mask_token(&app.login_token_input);
        lines.push(Line::from(vec![
            Span::styled("  Токен: ", Style::new().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{masked}█"), Style::new().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(Span::styled(
            "  Вставьте токен (Ctrl+Shift+V), Enter — проверить",
            Style::new().dim(),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Токен хранится только локально в config.toml",
            Style::new().dim(),
        )));
    }

    lines.push(Line::from(""));
    if !app.status_message.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {}", app.status_message),
            Style::new().fg(Color::LightYellow),
        )));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::new().borders(Borders::ALL).title(" Яндекс Музыка "))
            .wrap(Wrap { trim: false }),
        panel,
    );
}

fn mask_token(token: &str) -> String {
    let count = token.chars().count();
    if count <= 8 {
        "•".repeat(count)
    } else {
        format!(
            "{}…{}",
            "•".repeat(4),
            &token[token.len().saturating_sub(4)..]
        )
    }
}
