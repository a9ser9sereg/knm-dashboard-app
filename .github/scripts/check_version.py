#!/usr/bin/env python3
"""Сверка версии во всех файлах, где она записана (и с тегом релиза).

Версия хранится в четырёх местах и до сих пор синхронизировалась вручную —
README так и предписывал «продублировать версию». Разъехавшись, они дают
установщик одной версии с манифестом автообновления другой, причём молча.
Проверка ставится перед сборкой, чтобы релиз падал до публикации, а не после.

  python3 .github/scripts/check_version.py                # все файлы между собой
  python3 .github/scripts/check_version.py --tag v1.2.3   # и то и другое == тегу

Список файлов и шаблоны держать теми же, что в scripts/set-version.mjs:
скрипт двигает версию, этот — проверяет, что не осталось забытого места.
"""
import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]

SOURCES = (
    ("package.json", r'"version"\s*:\s*"(\d+\.\d+\.\d+)"'),
    ("src-tauri/tauri.conf.json", r'"version"\s*:\s*"(\d+\.\d+\.\d+)"'),
    # Версия из [package]: у зависимостей поле записано внутри фигурных
    # скобок и под шаблон с началом строки не подходит.
    ("src-tauri/Cargo.toml", r'^version = "(\d+\.\d+\.\d+)"$'),
    # Блок самого пакета: если он разъедется с Cargo.toml, cargo --locked
    # откажется собирать.
    ("src-tauri/Cargo.lock", r'name = "app"\r?\nversion = "(\d+\.\d+\.\d+)"'),
)


def read(rel: str, pattern: str) -> str:
    text = (ROOT / rel).read_text(encoding="utf-8")
    match = re.search(pattern, text, re.MULTILINE)
    if not match:
        print(f"::error::не нашёл версию в {rel} (шаблон: {pattern})")
        sys.exit(1)
    return match.group(1)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", help="имя тега релиза, например v0.1.7")
    args = parser.parse_args()

    found = {rel: read(rel, pattern) for rel, pattern in SOURCES}

    reference_file, reference = next(iter(found.items()))
    problems = [
        f"{reference_file} = {reference}, а {rel} = {version}"
        for rel, version in found.items()
        if version != reference
    ]

    if args.tag:
        tag = args.tag[1:] if args.tag.startswith("v") else args.tag
        problems += [
            f"тег = v{tag}, а {rel} = {version}"
            for rel, version in found.items()
            if version != tag
        ]

    if problems:
        for line in problems:
            print(f"::error::версии разошлись: {line}")
        print("Поднимать версию: npm run version:set -- <patch|minor|major|X.Y.Z>")
        return 1

    print(f"версии совпадают: {reference}" + (f" (тег {args.tag})" if args.tag else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
