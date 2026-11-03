mod api;
mod app;
pub mod auth;
mod config;
mod player;
mod ui;

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|arg| arg == "--logout") {
        return run_cli_logout();
    }

    if let Some(token) = args
        .iter()
        .position(|arg| arg == "--token")
        .and_then(|pos| args.get(pos + 1))
        .cloned()
        .or_else(|| {
            args.iter()
                .find_map(|arg| arg.strip_prefix("--token=").map(str::to_string))
        })
    {
        return run_cli_token(&token);
    }

    if args.iter().any(|arg| arg == "--login" || arg == "login") {
        return run_cli_login();
    }

    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .context("не удалось создать tokio runtime")?;

    tokio_rt.block_on(app::run())
}

fn print_help() {
    println!("yandex-music-tui — TUI-клиент Яндекс Музыки");
    println!();
    println!("Использование:");
    println!("  yandex-music-tui                 запустить TUI");
    println!("  yandex-music-tui --login         войти через окно Яндекс OAuth (webview)");
    println!("  yandex-music-tui --token <TOKEN> войти по готовому OAuth-токену");
    println!("  yandex-music-tui --logout        удалить сохранённый токен");
    println!("  yandex-music-tui --help          показать эту справку");
    println!();
    println!("В TUI: l — войти, o — выйти из аккаунта, q — выход.");
}

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("не удалось создать tokio runtime")
        .block_on(fut)
}

fn run_cli_login() -> anyhow::Result<()> {
    println!("Запуск окна авторизации Яндекс Музыки (webview oauth, как в Murglar)...");
    let token = match auth::login_webview()? {
        Some(t) => t,
        None => {
            println!("Авторизация отменена пользователем (окно закрыто).");
            return Ok(());
        }
    };

    let account = block_on(async {
        let client = api::YandexClient::new(Some(token.clone()));
        client.account().await
    })
    .context("токен получен, но профиль Яндекс Музыки не подтвердился")?;

    save_login(token, &account)
}

fn run_cli_token(token: &str) -> anyhow::Result<()> {
    println!("Проверка OAuth-токена Яндекс Музыки...");
    let account = block_on(async {
        let client = api::YandexClient::new(Some(token.trim().to_string()));
        client.account().await
    })
    .context("токен не принят Яндекс Музыкой")?;

    save_login(token.trim().to_string(), &account)
}

fn save_login(token: String, account: &api::AccountInfo) -> anyhow::Result<()> {
    let mut config = config::Config::load();
    config.oauth_token = Some(token);
    config.apply_account(account);
    config.save()?;

    println!("✔ Успешный вход!");
    println!("  Пользователь: {}", account.nickname);
    if let Some(login) = &account.login {
        println!("  Логин:        {login}");
    }
    if let Some(uid) = account.uid {
        println!("  UID:          {uid}");
    }
    println!(
        "  Яндекс Плюс:  {}",
        if account.plus {
            "активен"
        } else {
            "нет"
        }
    );
    println!("  Конфиг сохранён в: {}", config::Config::path().display());

    Ok(())
}

fn run_cli_logout() -> anyhow::Result<()> {
    let mut config = config::Config::load();
    config.logout();
    config.save()?;
    println!("Вы вышли из Яндекс Музыки, токен удалён.");
    println!("Конфиг: {}", config::Config::path().display());
    Ok(())
}
