use anyhow::{Context, Result};

pub const AUTH_START_URL: &str = "https://oauth.yandex.ru/authorize?response_type=token&display=popup&scope=music%3Acontent&scope=music%3Aread&scope=music%3Awrite&client_id=97fe03033fa34407ac9bcf91d5afed5b&redirect_uri=music-application%3A%2F%2Fdesktop%2Foauth%3FredirectUri%3D%26language%3Dru&state=abcdef123&origin=music_desktop&language=ru";

pub const REDIRECT_URI_PREFIX: &str = "music-application://desktop/oauth";

pub fn extract_token_from_url(url: &str) -> Option<String> {
    if !url.contains(REDIRECT_URI_PREFIX) {
        return None;
    }
    extract_access_token(url)
}

fn extract_access_token(input: &str) -> Option<String> {
    let after_token = input.split("access_token=").nth(1)?;
    let token = after_token.split(['&', '#']).next()?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

pub fn parse_token_input(input: &str) -> Option<String> {
    let trimmed = input.trim().trim_matches(|c| c == '"' || c == '\'');
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("access_token=") {
        return extract_access_token(trimmed);
    }
    if trimmed.contains(REDIRECT_URI_PREFIX)
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
    {
        return None;
    }
    Some(trimmed.to_string())
}

pub fn open_auth_url() -> Result<()> {
    std::process::Command::new("xdg-open")
        .arg(AUTH_START_URL)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("не удалось открыть браузер через xdg-open")?;
    Ok(())
}

pub fn register_uri_handler() -> Result<()> {
    let exe = std::env::current_exe().context("не удалось определить путь к исполняемому файлу")?;
    let apps_dir = dirs::data_dir()
        .context("не удалось определить XDG_DATA_HOME")?
        .join("applications");
    std::fs::create_dir_all(&apps_dir)?;

    let exec = exe
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%");
    let desktop = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Yandex Music TUI OAuth\n\
         Comment=Handle Yandex Music OAuth callback\n\
         Exec=\"{exec}\" --oauth-callback %u\n\
         Terminal=false\n\
         NoDisplay=true\n\
         MimeType=x-scheme-handler/music-application;\n"
    );
    let desktop_path = apps_dir.join("yandex-music-tui-oauth.desktop");
    std::fs::write(&desktop_path, desktop)?;

    let status = std::process::Command::new("xdg-mime")
        .args([
            "default",
            "yandex-music-tui-oauth.desktop",
            "x-scheme-handler/music-application",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("не удалось запустить xdg-mime")?;
    if !status.success() {
        anyhow::bail!("xdg-mime завершился с ошибкой");
    }

    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_token() {
        let url1 = "music-application://desktop/oauth?redirectUri=&language=ru#access_token=AQAAAAAAtest_12345&token_type=bearer&expires_in=31536000";
        assert_eq!(
            extract_token_from_url(url1),
            Some("AQAAAAAAtest_12345".to_string())
        );

        let url2 = "music-application://desktop/oauth?access_token=token_no_hash&state=123";
        assert_eq!(
            extract_token_from_url(url2),
            Some("token_no_hash".to_string())
        );

        let url_invalid = "https://oauth.yandex.ru/authorize?client_id=123";
        assert_eq!(extract_token_from_url(url_invalid), None);
    }

    #[test]
    fn test_parse_token_input() {
        let raw = "AQAAAAAAtest_12345";
        assert_eq!(parse_token_input(raw), Some(raw.to_string()));

        let quoted = " \"AQAAAAAAtest_12345\" ";
        assert_eq!(parse_token_input(quoted), Some(raw.to_string()));

        let redirect =
            "music-application://desktop/oauth#access_token=AQAAAAAAtest_12345&token_type=bearer";
        assert_eq!(parse_token_input(redirect), Some(raw.to_string()));

        assert_eq!(parse_token_input("   "), None);
        assert_eq!(parse_token_input("music-application://desktop/oauth"), None);
        assert_eq!(
            parse_token_input("https://oauth.yandex.ru/authorize?client_id=123"),
            None
        );
    }
}
