use crate::app::{App, ConfirmAction};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

pub fn draw(f: &mut Frame, app: &App) {
    let Some(action) = app.confirm else {
        return;
    };
    let p = app.palette();
    let full = f.area();
    let width = 48.min(full.width.saturating_sub(4)).max(1);
    let height = 7.min(full.height).max(1);
    let area = Rect {
        x: full.x + full.width.saturating_sub(width) / 2,
        y: full.y + full.height.saturating_sub(height) / 2,
        width,
        height,
    };
    f.render_widget(Clear, area);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(p.border))
        .style(p.base())
        .title_style(Style::new().fg(p.accent).add_modifier(Modifier::BOLD))
        .title(" Подтверждение ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let question = match action {
        ConfirmAction::Quit => "Выйти из приложения?",
        ConfirmAction::Logout => "Выйти из аккаунта?",
    };
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {question}"),
            Style::new().fg(p.warning).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter / y  ", Style::new().fg(p.accent)),
            Span::styled("— да", Style::new().fg(p.fg)),
        ]),
        Line::from(vec![
            Span::styled("  Esc / n    ", Style::new().fg(p.dim)),
            Span::styled("— нет", Style::new().fg(p.fg)),
        ]),
    ];
    f.render_widget(Paragraph::new(lines).style(Style::new().bg(p.bg)), inner);
}
