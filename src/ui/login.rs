use crate::app::App;
use ratatui::Frame;
use ratatui::prelude::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let p = app.palette();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Length(15),
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
            Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter  ", Style::new().fg(p.warning)),
            Span::styled(
                "открыть вход Яндекс OAuth в браузере",
                Style::new().fg(p.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("  t      ", Style::new().fg(p.warning)),
            Span::styled(
                "вставить токен или адрес кнопки «Открыть приложение»",
                Style::new().fg(p.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Esc    ", Style::new().fg(p.warning)),
            Span::styled(
                "продолжить без входа (поиск и артисты)",
                Style::new().fg(p.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("  4      ", Style::new().fg(p.warning)),
            Span::styled("настройки", Style::new().fg(p.fg)),
        ]),
        Line::from(vec![
            Span::styled("  q      ", Style::new().fg(p.warning)),
            Span::styled("выход", Style::new().fg(p.fg)),
        ]),
        Line::from(""),
    ];

    if app.login_token_editing {
        let masked = mask_token(&app.login_token_input);
        lines.push(Line::from(vec![
            Span::styled(
                "  Токен: ",
                Style::new().fg(p.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{masked}█"), Style::new().fg(p.warning)),
        ]));
        lines.push(Line::from(Span::styled(
            "  Вставьте токен или адрес кнопки «Открыть приложение» (Ctrl+Shift+V), Enter — проверить",
            Style::new().fg(p.dim),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Токен хранится только локально в config.toml",
            Style::new().fg(p.dim),
        )));
    }

    lines.push(Line::from(""));
    if !app.status_message.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {}", app.status_message),
            Style::new().fg(p.warning),
        )));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(p.border))
                    .title_style(Style::new().fg(p.fg))
                    .title(" Яндекс Музыка "),
            )
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
