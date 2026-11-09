/// Одна строка текста песни.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricLine {
    /// Время начала строки в миллисекундах. Для обычного текста — 0.
    pub time_ms: u64,
    pub text: String,
}

/// Разобранный текст песни.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    /// Есть ли у строк реальные таймкоды (LRC).
    pub synced: bool,
}

impl Lyrics {
    pub fn parse(raw: &str) -> Self {
        let mut lines = Vec::new();
        let mut offset_ms: i64 = 0;

        for source in raw.lines() {
            let source = source.trim_end_matches('\r');
            let (timestamps, text) = split_timestamps(source, &mut offset_ms);
            if timestamps.is_empty() {
                continue;
            }
            for time in timestamps {
                let time = (time as i64 + offset_ms).max(0) as u64;
                lines.push(LyricLine {
                    time_ms: time,
                    text: text.to_string(),
                });
            }
        }

        if lines.is_empty() {
            let plain: Vec<_> = raw
                .lines()
                .map(|line| line.trim().trim_end_matches('\r'))
                .filter(|line| !line.is_empty())
                .map(|line| LyricLine {
                    time_ms: 0,
                    text: line.to_string(),
                })
                .collect();
            return Self {
                synced: false,
                lines: plain,
            };
        }

        lines.sort_by_key(|line| line.time_ms);
        Self {
            synced: true,
            lines,
        }
    }

    /// Индекс строки, которая звучит в момент `position_secs`.
    pub fn current_index(&self, position_secs: f64) -> Option<usize> {
        if !self.synced || self.lines.is_empty() {
            return None;
        }
        let position_ms = (position_secs.max(0.0) * 1000.0) as u64;
        let index = self
            .lines
            .partition_point(|line| line.time_ms <= position_ms);
        index.checked_sub(1)
    }
}

fn split_timestamps<'a>(line: &'a str, offset_ms: &mut i64) -> (Vec<u64>, &'a str) {
    let mut timestamps = Vec::new();
    let mut rest = line.trim_start();

    while let Some(after_open) = rest.strip_prefix('[') {
        let Some(close) = after_open.find(']') else {
            break;
        };
        let tag = &after_open[..close];
        let after = &after_open[close + 1..];

        if let Some(value) = tag.strip_prefix("offset:") {
            if let Ok(value) = value.trim().parse::<i64>() {
                *offset_ms = value;
            }
            rest = after.trim_start();
            continue;
        }

        match parse_timestamp(tag) {
            Some(time) => {
                timestamps.push(time);
                rest = after.trim_start();
            }
            None => break,
        }
    }

    (timestamps, rest.trim())
}

fn parse_timestamp(tag: &str) -> Option<u64> {
    let (minutes, rest) = tag.split_once(':')?;
    let minutes: u64 = minutes.trim().parse().ok()?;

    let (seconds, fraction) = match rest.split_once(['.', ':']) {
        Some((seconds, fraction)) => (seconds, Some(fraction)),
        None => (rest, None),
    };
    let seconds: u64 = seconds.trim().parse().ok()?;
    let millis = fraction
        .map(|fraction| {
            let digits: String = fraction
                .chars()
                .take_while(char::is_ascii_digit)
                .take(3)
                .collect();
            if digits.is_empty() {
                return 0;
            }
            let value: u64 = digits.parse().unwrap_or(0);
            match digits.len() {
                1 => value * 100,
                2 => value * 10,
                _ => value,
            }
        })
        .unwrap_or(0);

    Some((minutes * 60 + seconds) * 1000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lrc_with_multiple_timestamps() {
        let raw = "[ar:Артист]\n[00:01.20][00:05.500]Привет\n[00:12]Мир\n";
        let lyrics = Lyrics::parse(raw);
        assert!(lyrics.synced);
        assert_eq!(lyrics.lines.len(), 3);
        assert_eq!(lyrics.lines[0].time_ms, 1200);
        assert_eq!(lyrics.lines[0].text, "Привет");
        assert_eq!(lyrics.lines[1].time_ms, 5500);
        assert_eq!(lyrics.lines[2].time_ms, 12_000);
    }

    #[test]
    fn applies_lrc_offset() {
        let raw = "[offset:500]\n[00:01.00]Строка";
        let lyrics = Lyrics::parse(raw);
        assert_eq!(lyrics.lines[0].time_ms, 1500);
    }

    #[test]
    fn falls_back_to_plain_text() {
        let lyrics = Lyrics::parse("Первая строка\n\nВторая строка\n");
        assert!(!lyrics.synced);
        assert_eq!(lyrics.lines.len(), 2);
        assert_eq!(lyrics.lines[1].text, "Вторая строка");
    }

    #[test]
    fn finds_current_line() {
        let lyrics = Lyrics::parse("[00:01.00]Раз\n[00:03.00]Два\n[00:08.00]Три");
        assert_eq!(lyrics.current_index(0.5), None);
        assert_eq!(lyrics.current_index(1.0), Some(0));
        assert_eq!(lyrics.current_index(4.0), Some(1));
        assert_eq!(lyrics.current_index(20.0), Some(2));
    }
}
