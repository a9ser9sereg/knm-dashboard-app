fn main() {
  // #![windows_subsystem = "windows"] в main.rs не всегда надёжно подавляет консоль
  // в релизных сборках Tauri v2 (известный баг — github.com/tauri-apps/tauri/issues/13230).
  // Форсируем через линковщик напрямую по PROFILE (не cfg!(debug_assertions) — в
  // build.rs это профиль самого build-скрипта, а не собираемого приложения).
  if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
    && std::env::var("PROFILE").as_deref() == Ok("release")
  {
    // -bin=app: ТОЛЬКО для бинарника (app.exe), не для app_lib.dll/staticlib,
    // иначе линковка cdylib падает (у DLL нет функции main).
    println!("cargo:rustc-link-arg-bin=app=/SUBSYSTEM:WINDOWS");
    println!("cargo:rustc-link-arg-bin=app=/ENTRY:mainCRTStartup");
  }
  // Без явного списка команд tauri-build не генерирует для них ACL-разрешение —
  // страница на remote-origin (наш случай, окно грузит настоящий HTTPS URL, не
  // локальный bundle) не может вызвать invoke() для команд приложения вообще,
  // никакая запись в capabilities/remote.json их не разблокирует (проверено
  // эмпирически: любой идентификатор для них Tauri считает "not found").
  tauri_build::try_build(
    tauri_build::Attributes::new().app_manifest(
      tauri_build::AppManifest::new().commands(&["get_app_version", "trigger_update_check"]),
    ),
  )
  .expect("failed to run tauri-build");
}
