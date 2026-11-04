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

    if let Some(callback) = args
        .iter()
        .position(|arg| arg == "--oauth-callback")
        .and_then(|pos| args.get(pos + 1))
        .cloned()
        .or_else(|| {
            args.iter()
                .find_map(|arg| arg.strip_prefix("--oauth-callback=").map(str::to_string))
        })
    {
        return run_oauth_callback(&callback);
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
    println!("  yandex-music-tui --login         открыть вход Яндекс OAuth в браузере");
    println!("  yandex-music-tui --token <TOKEN> войти по готовому OAuth-токену или URL");
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
    match auth::register_uri_handler() {
        Ok(()) => println!("Обработчик music-application:// зарегистрирован."),
        Err(e) => {
            println!("Не удалось зарегистрировать обработчик music-application://: {e}");
            println!("После входа скопируйте адрес кнопки «Открыть приложение» вручную.");
        }
    }

    println!("Открываю страницу авторизации Яндекс Музыки в браузере...");
    println!();
    println!("Если браузер не открылся, перейдите по ссылке вручную:");
    println!("{}", auth::AUTH_START_URL);
    println!();
    if let Err(e) = auth::open_auth_url() {
        println!("Не удалось открыть браузер автоматически: {e}");
    }
    println!(
        "После входа нажмите в браузере кнопку «Открыть приложение» — токен вернётся автоматически."
    );
    println!("Если кнопка не сработала: ПКМ по ней → «Копировать адрес ссылки», затем:");
    println!("  yandex-music-tui --token \"<СКОПИРОВАННАЯ_ССЫЛКА>\"");
    Ok(())
}

fn run_oauth_callback(url: &str) -> anyhow::Result<()> {
    let token = auth::parse_token_input(url)
        .context("в OAuth-редиректе не найден параметр access_token")?;
    config::Config::store_pending_token(&token)?;
    Ok(())
}

fn run_cli_token(token_input: &str) -> anyhow::Result<()> {
    let token = auth::parse_token_input(token_input)
        .context("не удалось найти OAuth-токен во введённом значении")?;

    println!("Проверка OAuth-токена Яндекс Музыки...");
    let account = block_on(async {
        let client = api::YandexClient::new(Some(token.clone()));
        client.account().await
    })
    .context("токен не принят Яндекс Музыкой")?;

    save_login(token, &account)
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
