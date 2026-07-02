# Дашборд ГСН МО — десктоп-приложение

Tauri-обёртка (Win/macOS) вокруг дашборда https://d5do79fn06e3ml9pipcc.kr8f6hld.apigw.yandexcloud.net.
Окно открывает реальный удалённый сайт — весь код дашборда и данные живут в
приватном репозитории `control_KNM_UNzS`, здесь только код обёртки (трей,
офлайн-кэш последних данных, автообновление). Долгая сессия для приложения —
через `/api/refresh` на бэкенде дашборда, см. `auth/login/login.py` в
`control_KNM_UNzS`.

## Установка (пользователю)

Скачать установщик под свою систему:
- Windows: `https://storage.yandexcloud.net/knm-dashboard-app/latest/KNM-Dashboard-Setup.exe` *(ссылку уточнить после первого релиза)*
- macOS: аналогично, `.dmg`

Без цифровой подписи (внутренний инструмент) — Windows/macOS могут показать
предупреждение о неизвестном разработчике при первом запуске, это ожидаемо.

## Релиз новой версии

```
npm version patch   # или minor/major — поднимает версию в package.json
# вручную продублировать версию в src-tauri/tauri.conf.json ("version")
git commit -am "vX.Y.Z"
git tag vX.Y.Z
git push && git push --tags
```

Пуш тега запускает `.github/workflows/release.yml`: сборка .exe (Windows) и
универсального .dmg (macOS) на раннерах GitHub Actions, публикация
установщиков и манифеста автообновления `latest.json` в бакет Yandex Object
Storage `knm-dashboard-app` (публичный на чтение). GitHub Releases
сознательно не используются — установщики на `github.io`/`githubusercontent`
блокируются у части пользователей в РФ без VPN (тот же урок, что и с самим
дашбордом).

## Секреты репозитория (Settings → Secrets → Actions)

- `YC_RELEASER_KEY_ID` / `YC_RELEASER_SECRET` — статический S3-ключ SA
  `app-releaser` (доступ только к бакету `knm-dashboard-app`, отдельный от
  ключей самого дашборда).
- `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` —
  ключ подписи автообновления (`secrets/updater.key` + пароль локально,
  публичный ключ уже в `src-tauri/tauri.conf.json`).
