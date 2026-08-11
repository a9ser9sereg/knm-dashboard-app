#!/usr/bin/env python3
"""Проверяет, что опубликованный релиз действительно работает у пользователя.

До сих пор единственной проверкой релиза был сам факт, что workflow дошёл до
конца. Но обновление живое только если выполняется всё сразу: манифест в
бакете отдаёт новую версию, ссылки в нём ведут на файлы этой же версии, файлы
лежат публично и не пустые. Разъехаться это может тихо — например, объект
залился без публичного доступа: сборка зелёная, а у пользователей
автообновление молча не работает.

  python3 .github/scripts/verify_release.py --version 0.1.7 \
      --manifest-url https://storage.yandexcloud.net/knm-dashboard-app/latest.json
"""
import argparse
import json
import sys
import time
import urllib.error
import urllib.request

# Ключи платформ, которые обязаны быть в манифесте: Windows и обе архитектуры
# macOS (universal-сборка отдаёт один файл на оба ключа).
REQUIRED_PLATFORMS = ("windows-x86_64", "darwin-x86_64", "darwin-aarch64")

# Бакет отдаёт latest.json с no-cache, но сразу после заливки объект может
# успеть отдаться старым — пробуем несколько раз, прежде чем ронять релиз.
ATTEMPTS = 5
PAUSE_SECONDS = 10


def fetch(url: str, timeout: int = 30) -> bytes:
    request = urllib.request.Request(url, headers={"Cache-Control": "no-cache"})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.read()


def head(url: str, timeout: int = 30):
    request = urllib.request.Request(url, method="HEAD")
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.status, int(response.headers.get("Content-Length") or 0)


def load_manifest(url: str, version: str) -> dict:
    """Ждёт, пока по ссылке появится манифест нужной версии."""
    last = ""
    for attempt in range(1, ATTEMPTS + 1):
        try:
            manifest = json.loads(fetch(url))
            if manifest.get("version") == version:
                return manifest
            last = f"манифест отдаёт версию {manifest.get('version')!r}, ждём {version!r}"
        except (urllib.error.URLError, json.JSONDecodeError, TimeoutError) as e:
            last = f"манифест не читается: {e}"
        print(f"попытка {attempt}/{ATTEMPTS}: {last}")
        if attempt < ATTEMPTS:
            time.sleep(PAUSE_SECONDS)
    print(f"::error::{url}: {last}")
    sys.exit(1)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True, help="версия без v, например 0.1.7")
    parser.add_argument("--manifest-url", required=True)
    args = parser.parse_args()

    manifest = load_manifest(args.manifest_url, args.version)
    platforms = manifest.get("platforms") or {}

    problems = []
    for key in REQUIRED_PLATFORMS:
        entry = platforms.get(key)
        if not entry:
            problems.append(f"в манифесте нет платформы {key}")
            continue
        if not entry.get("signature"):
            problems.append(f"{key}: пустая подпись")
        url = entry.get("url", "")
        if f"/v{args.version}/" not in url:
            problems.append(f"{key}: ссылка ведёт не на v{args.version} — {url}")
            continue
        try:
            status, size = head(url)
        except (urllib.error.URLError, TimeoutError) as e:
            # Приватный объект отдаёт 403 — ровно тот случай, ради которого
            # проверка и заведена: сборка зелёная, а скачать нечего.
            problems.append(f"{key}: файл недоступен ({e}) — {url}")
            continue
        if status != 200 or size == 0:
            problems.append(f"{key}: HTTP {status}, размер {size} — {url}")
        else:
            print(f"{key}: ок, {size / 1024 / 1024:.1f} МБ — {url}")

    if problems:
        for line in problems:
            print(f"::error::{line}")
        return 1

    print(f"релиз {args.version} опубликован и доступен на скачивание")
    return 0


if __name__ == "__main__":
    sys.exit(main())
