use crate::app::App;
use crate::theme::Palette;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

pub fn draw(f: &mut Frame, app: &App) {
    let p = app.palette();
    let full = f.area();
    f.render_widget(Clear, full);
    f.render_widget(Block::new().style(p.base()), full);

    let area = centered_rect(full);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(p.border))
        .style(Style::new().fg(p.fg).bg(p.bg))
        .title_style(Style::new().fg(p.accent).add_modifier(Modifier::BOLD))
        .title(" Подсказки по клавишам ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width >= 86 {
        let [left, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .areas(inner);
        f.render_widget(
            Paragraph::new(left_lines(p)).style(Style::new().bg(p.bg)),
            left,
        );
        f.render_widget(
            Paragraph::new(right_lines(p)).style(Style::new().bg(p.bg)),
            right,
        );
    } else {
        f.render_widget(
            Paragraph::new(compact_lines(p)).style(Style::new().bg(p.bg)),
            inner,
        );
    }
}

fn centered_rect(area: Rect) -> Rect {
    let width = area.width.saturating_sub(2).max(1);
    let height = area.height.saturating_sub(1).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn heading(text: &str, p: Palette) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {text}"),
        Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
    ))
}

fn row(key: &str, description: &str, p: Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {key:<15} "), Style::new().fg(p.warning)),
        Span::styled(description.to_string(), Style::new().fg(p.fg)),
    ])
}

fn blank() -> Line<'static> {
    Line::from("")
}

fn left_lines(p: Palette) -> Vec<Line<'static>> {
    vec![
        heading("Навигация", p),
        row("1 2 3", "Волна · Библиотека · Поиск", p),
        row("4 / ,", "Настройки", p),
        row("Tab / ⇧Tab", "Раздел / фильтр", p),
        row("↑↓ PgUp PgDn", "Перемещение по списку", p),
        row("Home / End", "Начало / конец", p),
        row("Enter / Esc", "Играть-открыть / назад", p),
        row("r / s", "Обновить / перемешать", p),
        row("/", "Новый поиск", p),
        blank(),
        heading("Воспроизведение", p),
        row("Пробел", "Пауза / продолжить", p),
        row("n / p", "Следующий / предыдущий", p),
        row("← / →", "Перемотка ±5 с", p),
        row("⇧↑ / ⇧↓", "Громкость ±1%", p),
        row("Ctrl+↑/↓", "Громкость ±5%", p),
        row("+ / -", "Громкость ±5%", p),
        row("m / ~", "Mute / качество", p),
        row("f / d", "Лайк / дизлайк", p),
        row("Ctrl+S", "Сохранить конфиг", p),
    ]
}

fn compact_lines(p: Palette) -> Vec<Line<'static>> {
    vec![
        heading("Навигация", p),
        row("1 2 3 4", "Волна · Библиотека · Поиск · Настройки", p),
        row("Tab / ⇧Tab", "Раздел / фильтр", p),
        row("↑↓ PgUp PgDn", "Перемещение по списку", p),
        row("Enter / Esc", "Играть-открыть / назад", p),
        row("r / s / /", "Обновить · перемешать · поиск", p),
        heading("Воспроизведение", p),
        row("Пробел", "Пауза / продолжить", p),
        row("n / p", "Следующий / предыдущий", p),
        row("← / →", "Перемотка ±5 с", p),
        row("⇧↑ / ⇧↓", "Громкость ±1%", p),
        row("Ctrl+↑/↓", "Громкость ±5%", p),
        row("+ / -", "Громкость ±5%", p),
        row("m / ~", "Mute / качество", p),
        row("f / d", "Лайк / дизлайк", p),
        row("Ctrl+S", "Сохранить конфиг", p),
        heading("Аккаунт и выход", p),
        row("l / t / o", "Войти · токен · выйти", p),
        row("q / Ctrl+C", "Выход (q — подтверждение)", p),
        row("y / n", "Подтвердить / отменить", p),
        row("?", "Открыть / закрыть справку", p),
    ]
}

fn right_lines(p: Palette) -> Vec<Line<'static>> {
    vec![
        heading("Ввод текста", p),
        row("Enter", "Применить / искать", p),
        row("Esc", "Отменить", p),
        row("Tab / ⇧Tab", "Фильтр поиска", p),
        blank(),
        heading("Аккаунт", p),
        row("l", "Войти через браузер", p),
        row("t", "Ввести токен вручную", p),
        row("o", "Выйти из аккаунта (подтверждение)", p),
        row("y / n", "Подтвердить / отменить", p),
        blank(),
        heading("Выход и справка", p),
        row("q / Ctrl+C", "Выход (q — с подтверждением)", p),
        row("?", "Открыть справку", p),
        row("Esc / ?", "Закрыть справку", p),
    ]
}
