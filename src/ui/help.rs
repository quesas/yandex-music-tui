use crate::app::App;
use crate::theme::Palette;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

pub fn draw(f: &mut Frame, app: &mut App) {
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
        .title(" Подсказки по клавишам · ↑/↓ — прокрутка ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if inner.width >= 110 {
        let [left, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .areas(inner);
        let left_lines = left_lines(p);
        let right_lines = right_lines(p);
        let max_scroll = left_lines
            .len()
            .max(right_lines.len())
            .saturating_sub(inner.height as usize);
        app.help_scroll = app
            .help_scroll
            .min(max_scroll.min(u16::MAX as usize) as u16);
        let scroll = (app.help_scroll, 0);
        f.render_widget(
            Paragraph::new(left_lines)
                .scroll(scroll)
                .style(Style::new().bg(p.bg)),
            left,
        );
        f.render_widget(
            Paragraph::new(right_lines)
                .scroll(scroll)
                .style(Style::new().bg(p.bg)),
            right,
        );
    } else {
        let lines = compact_lines(p);
        let max_scroll = lines.len().saturating_sub(inner.height as usize);
        app.help_scroll = app
            .help_scroll
            .min(max_scroll.min(u16::MAX as usize) as u16);
        f.render_widget(
            Paragraph::new(lines)
                .scroll((app.help_scroll, 0))
                .style(Style::new().bg(p.bg)),
            inner,
        );
    }
}

fn centered_rect(area: Rect) -> Rect {
    let width = area.width.saturating_sub(2).max(1);
    let height = area.height.max(1);
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
        Span::styled(format!(" {key:<20} "), Style::new().fg(p.warning)),
        Span::styled(description.to_string(), Style::new().fg(p.fg)),
    ])
}

fn blank() -> Line<'static> {
    Line::from("")
}

fn left_lines(p: Palette) -> Vec<Line<'static>> {
    vec![
        heading("Навигация", p),
        row("1 / 2 / 3", "Волна · Библиотека · Поиск", p),
        row("4 или , (запятая)", "Настройки", p),
        row("w", "Настройки Моей волны", p),
        row("Tab / Shift+Tab", "Раздел или фильтр", p),
        row("↑ / ↓, PgUp / PgDn", "Перемещение / прокрутка", p),
        row("Home / End", "Начало / конец списка", p),
        row("Enter", "Играть / открыть", p),
        row("Esc", "Назад / отменить", p),
        row("r / s", "Обновить / перемешать", p),
        row("t", "Текст песни и обложка справа", p),
        row("/", "Новый поиск", p),
        heading("Воспроизведение", p),
        row("Пробел", "Пауза / продолжить", p),
        row("n / p", "Следующий / предыдущий", p),
        row("← / →", "Перемотка на 5 секунд", p),
        row("Shift + ↑ / ↓", "Громкость ±1 %", p),
        row("Ctrl + ↑ / ↓", "Громкость ±5 % (и + / −)", p),
        row("m / ~ (тильда)", "Mute / качество звука", p),
        row("Shift + R", "Повтор: выкл → альбом → трек", p),
        row("f / d", "Лайк / дизлайк", p),
    ]
}

fn compact_lines(p: Palette) -> Vec<Line<'static>> {
    vec![
        heading("Навигация", p),
        row("1 / 2 / 3 / 4", "Волна · Библиотека · Поиск · Настройки", p),
        row("w", "Настройки Моей волны", p),
        row("Tab / Shift+Tab", "Раздел / фильтр", p),
        row("↑ / ↓, PgUp / PgDn", "Перемещение / прокрутка", p),
        row("Enter / Esc", "Играть-открыть / назад", p),
        row("r / s / /", "Обновить · перемешать · поиск", p),
        row("t", "Текст песни и обложка", p),
        heading("Воспроизведение", p),
        row("Пробел", "Пауза / продолжить", p),
        row("n / p", "Следующий / предыдущий", p),
        row("← / →", "Перемотка на 5 секунд", p),
        row("Shift + ↑ / ↓", "Громкость ±1 %", p),
        row("Ctrl + ↑ / ↓", "Громкость ±5 % (и + / −)", p),
        row("m / ~ (тильда)", "Mute / качество звука", p),
        row("Shift + R", "Повтор: выкл → альбом → трек", p),
        row("f / d", "Лайк / дизлайк", p),
        heading("Аккаунт и выход", p),
        row("l / t / o", "Войти · токен · выйти", p),
        row("q / Ctrl+C", "Выход (q — с подтверждением)", p),
        row("y / n", "Подтвердить / отменить", p),
        row("?", "Открыть / закрыть справку", p),
        row("Ctrl+S", "Сохранить настройки", p),
    ]
}

fn right_lines(p: Palette) -> Vec<Line<'static>> {
    vec![
        heading("Настройки и ввод", p),
        row("↑ / ↓", "Выбор пункта", p),
        row("← / → или Enter", "Изменить значение", p),
        row("a", "Применить настройки волны", p),
        row("Esc", "Назад с применением", p),
        blank(),
        heading("Ввод текста", p),
        row("Tab / Shift+Tab", "Фильтр поиска", p),
        row("Enter", "Применить / искать", p),
        row("Esc", "Отменить ввод", p),
        blank(),
        heading("Аккаунт", p),
        row("l", "Войти через браузер", p),
        row("t", "Ввести токен вручную", p),
        row("o", "Выйти из аккаунта", p),
        blank(),
        heading("Выход и справка", p),
        row("q / Ctrl+C", "Выход (q — подтвердить)", p),
        row("y / n", "Подтвердить / отменить", p),
        row("?", "Открыть / закрыть справку", p),
        row("Ctrl+S", "Сохранить настройки", p),
    ]
}
