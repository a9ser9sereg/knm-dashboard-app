use tauri::{
  menu::{Menu, MenuItem},
  tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
  Manager, WindowEvent,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_updater::UpdaterExt;

pub fn run() {
  tauri::Builder::default()
    // Должен регистрироваться первым: повторный запуск (двойной клик по
    // ярлыку, когда приложение уже свёрнуто в трей) просто поднимает окно.
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
      // macOS: приложение пряталось целиком (app.hide) при закрытии окна —
      // разпрятать перед показом окна, иначе окно останется невидимым.
      #[cfg(target_os = "macos")]
      let _ = app.show();
      if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
      }
    }))
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_dialog::init())
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
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;

      let update_handle = app.handle().clone();
      tauri::async_runtime::spawn(async move {
        if let Err(e) = check_for_update(update_handle).await {
          log::warn!("update check failed: {e}");
        }
      });

      let show_i = MenuItem::with_id(app, "show", "Показать", true, None::<&str>)?;
      let quit_i = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
      let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

      TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
          "show" => {
            #[cfg(target_os = "macos")]
            let _ = app.show();
            if let Some(w) = app.get_webview_window("main") {
              let _ = w.show();
              let _ = w.set_focus();
            }
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
            let app = tray.app_handle();
            #[cfg(target_os = "macos")]
            let _ = app.show();
            if let Some(w) = app.get_webview_window("main") {
              let _ = w.show();
              let _ = w.set_focus();
            }
          }
        })
        .build(app)?;

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
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

// Проверка обновлений при каждом запуске (и по кнопке «Проверить обновления» в
// Настройках). Не молча — спрашиваем подтверждение диалогом, т.к. на Windows
// установка обновления сама закрывает приложение (ограничение инсталлятора).
// Ok(true) — обновление найдено (диалог показан, независимо от решения
// пользователя); Ok(false) — уже последняя версия. Различие нужно ручному
// вызову: иначе кнопка «Проверить» при «и так последняя версия» выглядела бы
// нерабочей — обратной связи не было бы совсем.
async fn check_for_update(app: tauri::AppHandle) -> tauri_plugin_updater::Result<bool> {
  let Some(update) = app.updater()?.check().await? else {
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
    .title("Обновление дашборда ГСН МО")
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
