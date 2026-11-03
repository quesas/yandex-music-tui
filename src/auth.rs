use anyhow::{Context, Result, bail};
use std::sync::mpsc;
use std::thread;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::run_return::EventLoopExtRunReturn,
    platform::unix::{EventLoopBuilderExtUnix, WindowExtUnix},
    window::WindowBuilder,
};
use wry::{WebViewBuilder, WebViewBuilderExtUnix};

/// OAuth URL авторизации Яндекс Музыки из репозитория Murglar.
pub const AUTH_START_URL: &str = "https://oauth.yandex.ru/authorize?response_type=token&display=popup&scope=music%3Acontent&scope=music%3Aread&scope=music%3Awrite&client_id=97fe03033fa34407ac9bcf91d5afed5b&redirect_uri=music-application%3A%2F%2Fdesktop%2Foauth%3FredirectUri%3D%26language%3Dru&state=abcdef123&origin=music_desktop&language=ru";

/// Префикс URI, на который редиректит Яндекс после успешного входа.
pub const REDIRECT_URI_PREFIX: &str = "music-application://desktop/oauth";

/// User-Agent официального desktop-клиента Яндекс Музыки (из Murglar).
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) YandexMusic/5.62.0 Chrome/136.0.7103.149 Electron/36.4.0 Safari/537.36";

#[derive(Debug)]
enum UserEvent {
    TokenFound(String),
}

/// Извлечение access_token из URL редиректа.
pub fn extract_token_from_url(url: &str) -> Option<String> {
    if !url.contains(REDIRECT_URI_PREFIX) {
        return None;
    }
    let after_token = url.split("access_token=").nth(1)?;
    let token = after_token.split('&').next()?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// Открывает окно WebView для входа в Яндекс Музыку.
///
/// Блокирует текущий поток до завершения входа или закрытия окна пользователем.
/// Возвращает `Ok(Some(token))` при успешной авторизации, `Ok(None)` если пользователь закрыл окно.
pub fn login_webview() -> Result<Option<String>> {
    let has_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    let has_x11 = std::env::var("DISPLAY").is_ok();
    if !has_wayland && !has_x11 {
        bail!(
            "Графический сервер не найден (WAYLAND_DISPLAY и DISPLAY отсутствуют). Авторизация через webview требует GUI."
        );
    }

    let (tx, rx) = mpsc::channel();

    let thread_handle = thread::Builder::new()
        .name("yandex-webview-oauth".into())
        .spawn(move || {
            let mut builder = EventLoopBuilder::<UserEvent>::with_user_event();
            builder.with_any_thread(true);
            let mut event_loop = builder.build();
            let proxy = event_loop.create_proxy();

            let window = match WindowBuilder::new()
                .with_title("Яндекс Музыка — Авторизация")
                .with_inner_size(tao::dpi::LogicalSize::new(600.0, 750.0))
                .build(&event_loop)
            {
                Ok(w) => w,
                Err(e) => {
                    let _ = tx.send(Err(anyhow::anyhow!("Не удалось создать окно Tao: {e}")));
                    return;
                }
            };

            let vbox = match window.default_vbox() {
                Some(v) => v,
                None => {
                    let _ = tx.send(Err(anyhow::anyhow!(
                        "Не удалось получить default_vbox окна"
                    )));
                    return;
                }
            };

            let webview_builder = WebViewBuilder::new()
                .with_user_agent(USER_AGENT)
                .with_url(AUTH_START_URL)
                .with_navigation_handler(move |uri| {
                    if let Some(token) = extract_token_from_url(&uri) {
                        let _ = proxy.send_event(UserEvent::TokenFound(token));
                        return false;
                    }
                    true
                });

            let _webview = match webview_builder.build_gtk(vbox) {
                Ok(w) => w,
                Err(e) => {
                    let _ = tx.send(Err(anyhow::anyhow!(
                        "Не удалось инициализировать WebKit WebView: {e}"
                    )));
                    return;
                }
            };

            let mut captured_token = None;
            event_loop.run_return(|event, _, control_flow| {
                *control_flow = ControlFlow::Wait;
                match event {
                    Event::UserEvent(UserEvent::TokenFound(token)) => {
                        captured_token = Some(token);
                        *control_flow = ControlFlow::Exit;
                    }
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            });

            let _ = tx.send(Ok(captured_token));
        })
        .context("не удалось запустить поток webview")?;

    thread_handle
        .join()
        .map_err(|_| anyhow::anyhow!("поток webview oauth запаниковал"))?;

    match rx.recv() {
        Ok(res) => res,
        Err(_) => Ok(None),
    }
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
}
