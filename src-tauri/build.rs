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
  tauri_build::build()
}
