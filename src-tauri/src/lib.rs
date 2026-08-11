use std::time::Duration;

use tauri::{
  menu::{Menu, MenuItem, PredefinedMenuItem},
  tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
  AppHandle, Manager, WindowEvent,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_window_state::StateFlags;

const DIALOG_TITLE: &str = "Обновление дашборда ГСН МО";

// Сколько ждём ответа от бакета с манифестом. Без ограничения зависший ответ
// оставляет висеть фоновую задачу, а при ручной проверке — ещё и пользователя
// без ответа: нажал «Проверить обновления» и ничего не происходит.
const UPDATE_CHECK_TIMEOUT: Duration = Duration::from_secs(20);

// Логи режутся по размеру и хранится только последний файл: это диагностика
// «что случилось только что», а не архив.
const LOG_FILE_LIMIT_BYTES: u128 = 512 * 1024;

pub fn run() {
  tauri::Builder::default()
    // Должен регистрироваться первым: повторный запуск (двойной клик по
    // ярлыку, когда приложение уже свёрнуто в трей) просто поднимает окно.
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
      show_main(app);
    }))
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_dialog::init())
    // Размер и положение окна между запусками. Дашборд держат открытым и
    // подгоняют окно под свой экран — раскладывать его заново при каждом
    // запуске незачем.
    //
    // Флаги перечислены явно, без флага VISIBLE из набора по умолчанию: у нас
    // закрытие окна = сворачивание в трей, поэтому «Выход» из трея почти всегда
    // случается при спрятанном окне. Плагин запомнил бы visible = false и при
    // следующем запуске окно не показал — приложение стартовало бы одной
    // иконкой в трее.
    .plugin(
      tauri_plugin_window_state::Builder::new()
        .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
        .build(),
    )
    // Закрытие окна сворачивает в трей вместо завершения процесса —
    // выйти можно только через пункт «Выход» в меню трея.
    .on_window_event(|window, event| {
      if let WindowEvent::CloseRequested { api, .. } = event {
        // На macOS вызов window.hide() прямо внутри обработчика CloseRequested
        // вешает главный поток (дедлок в цикле закрытия NSWindow) — окно не
        // сворачивается, приложение зависает, приходится завершать через док.
        // Поэтому на macOS прячем всё приложение (аналог Cmd+H) — окно остаётся
        // живым, восстанавливается из трея. На Windows/Linux — обычный hide окна.
        #[cfg(target_os = "macos")]
        let _ = window.app_handle().hide();
        #[cfg(not(target_os = "macos"))]
        let _ = window.hide();
        api.prevent_close();
      }
    })
    .plugin(tauri_plugin_store::Builder::default().build())
    // Ссылки target="_blank" внутри страницы (например, наши же кнопки
    // "скачать" в анонсе приложения) Tauri по умолчанию открывает в новом
    // "сыром" окне Tauri, а не в системном браузере — его закрытие могло
    // валить всё приложение. JS-шим в sites/index.html перехватывает такие
    // клики и зовёт этот плагин, чтобы ссылка открылась в обычном браузере.
    .plugin(tauri_plugin_opener::init())
    .invoke_handler(tauri::generate_handler![get_app_version, trigger_update_check])
    .setup(|app| {
      // Логи пишутся и в релизной сборке. Раньше плагин подключался только под
      // debug_assertions, то есть у пользователя единственная запись о том,
      // почему не сработало автообновление, уходила в никуда — и разбираться
      // приходилось по описанию «просто не обновляется».
      app.handle().plugin(
        tauri_plugin_log::Builder::new()
          .level(if cfg!(debug_assertions) {
            log::LevelFilter::Info
          } else {
            log::LevelFilter::Warn
          })
          .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
          ])
          .max_file_size(LOG_FILE_LIMIT_BYTES)
          .rotation_strategy(RotationStrategy::KeepOne)
          .build(),
      )?;

      app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;

      let update_handle = app.handle().clone();
      tauri::async_runtime::spawn(async move {
        if let Err(e) = check_for_update(update_handle).await {
          log::warn!("update check failed: {e}");
        }
      });

      let show_i = MenuItem::with_id(app, "show", "Показать", true, None::<&str>)?;
      let reload_i = MenuItem::with_id(app, "reload", "Перезагрузить страницу", true, None::<&str>)?;
      let browser_i = MenuItem::with_id(app, "browser", "Открыть в браузере", true, None::<&str>)?;
      let update_i = MenuItem::with_id(app, "update", "Проверить обновления", true, None::<&str>)?;
      let quit_i = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
      let menu = Menu::with_items(
        app,
        &[
          &show_i,
          &reload_i,
          &browser_i,
          &PredefinedMenuItem::separator(app)?,
          &update_i,
          &PredefinedMenuItem::separator(app)?,
          &quit_i,
        ],
      )?;

      let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
          "show" => show_main(app),
          "reload" => reload_page(app),
          "browser" => open_in_browser(app),
          "update" => {
            // Проверка ходит в сеть и показывает диалог — в обработчике меню
            // ей делать нечего, иначе меню зависнет вместе с ней.
            let handle = app.clone();
            tauri::async_runtime::spawn(async move { manual_update_check(handle).await });
          }
          "quit" => app.exit(0),
          _ => {}
        })
        .on_tray_icon_event(|tray, event| {
          if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
          } = event
          {
            show_main(tray.app_handle());
          }
        });

      // Иконка окна задана в конфиге, но брать её через unwrap незачем:
      // приложение без иконки в трее полезнее, чем паника при запуске.
      if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
      }
      tray.build(app)?;

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

// Показать главное окно. Собрано в одном месте: те же три строки нужны и
// повторному запуску, и меню трея, и клику по иконке трея.
fn show_main(app: &AppHandle) {
  // macOS: приложение прячется целиком (app.hide) при закрытии окна —
  // разпрятать перед показом окна, иначе окно останется невидимым.
  #[cfg(target_os = "macos")]
  let _ = app.show();
  if let Some(w) = app.get_webview_window("main") {
    let _ = w.show();
    let _ = w.set_focus();
  }
}

// Окно смотрит в удалённый адрес: если страница отвалилась (сеть моргнула,
// шлюз перезапустился), перезагрузить её изнутри приложения было нечем.
fn reload_page(app: &AppHandle) {
  if let Some(w) = app.get_webview_window("main") {
    let _ = w.reload();
  }
}

// Открыть тот же адрес в системном браузере — запасной путь, когда с окном
// приложения что-то не так, а данные нужны сейчас. Адрес берётся из самого
// окна, чтобы не держать вторую копию URL в коде.
fn open_in_browser(app: &AppHandle) {
  let Some(window) = app.get_webview_window("main") else {
    return;
  };
  match window.url() {
    Ok(url) => {
      if let Err(e) = app.opener().open_url(url.to_string(), None::<&str>) {
        log::warn!("не удалось открыть адрес в браузере: {e}");
      }
    }
    Err(e) => log::warn!("не удалось получить адрес окна: {e}"),
  }
}

// Возвращает версию приложения — для панели «Настройки» (десктоп).
#[tauri::command]
fn get_app_version(app: tauri::AppHandle) -> String {
  app.package_info().version.to_string()
}

// Ручной вызов проверки обновлений из «Настроек» — переиспользует ту же
// check_for_update, что и автозапуск, без дублирования логики диалога/установки.
#[tauri::command]
async fn trigger_update_check(app: tauri::AppHandle) -> Result<bool, String> {
  check_for_update(app).await.map_err(|e| e.to_string())
}

// Проверка по пункту меню в трее. В отличие от проверки при запуске, молчать
// здесь нельзя: пользователь нажал и ждёт ответа, даже если обновления нет.
// В «Настройках» ту же роль играет ответ команды trigger_update_check.
async fn manual_update_check(app: AppHandle) {
  let message = match check_for_update(app.clone()).await {
    // Обновление нашлось — диалог с ним уже показан, второй не нужен.
    Ok(true) => return,
    Ok(false) => format!(
      "Установлена последняя версия {}.",
      app.package_info().version
    ),
    Err(e) => {
      log::warn!("manual update check failed: {e}");
      format!("Не удалось проверить обновления.\n\n{e}")
    }
  };
  app.dialog().message(message).title(DIALOG_TITLE).blocking_show();
}

// Проверка обновлений при каждом запуске (и по кнопке «Проверить обновления» в
// Настройках и меню трея). Не молча — спрашиваем подтверждение диалогом, т.к.
// на Windows установка обновления сама закрывает приложение (ограничение
// инсталлятора).
// Ok(true) — обновление найдено (диалог показан, независимо от решения
// пользователя); Ok(false) — уже последняя версия. Различие нужно ручному
// вызову: иначе кнопка «Проверить» при «и так последняя версия» выглядела бы
// нерабочей — обратной связи не было бы совсем.
async fn check_for_update(app: AppHandle) -> tauri_plugin_updater::Result<bool> {
  let updater = app.updater_builder().timeout(UPDATE_CHECK_TIMEOUT).build()?;
  let Some(update) = updater.check().await? else {
    return Ok(false);
  };

  let notes = update.body.clone().unwrap_or_default();
  let message = format!(
    "Доступна новая версия {}.\n\n{}\n\nОбновить и перезапустить сейчас?",
    update.version, notes
  );
  let confirmed = app
    .dialog()
    .message(message)
    .title(DIALOG_TITLE)
    .buttons(MessageDialogButtons::OkCancelCustom(
      "Обновить".into(),
      "Позже".into(),
    ))
    .blocking_show();

  if !confirmed {
    return Ok(true);
  }

  update.download_and_install(|_chunk, _total| {}, || {}).await?;
  app.restart();
}
