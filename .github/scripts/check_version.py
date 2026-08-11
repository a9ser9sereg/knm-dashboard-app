#!/usr/bin/env python3
"""Сверка версии в package.json и src-tauri/tauri.conf.json (и с тегом релиза).

Версия хранится в двух файлах и до сих пор синхронизировалась вручную —
README так и предписывал «продублировать версию». Разъехавшись, они дают
установщик одной версии с манифестом автообновления другой, причём молча.
Проверка ставится перед сборкой, чтобы релиз падал до публикации, а не после.

  python3 .github/scripts/check_version.py            # package.json == tauri.conf.json
  python3 .github/scripts/check_version.py --tag v1.2.3   # и то и другое == тегу
"""
import argparse
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]


def read(path: pathlib.Path, *keys: str) -> str:
    data = json.loads(path.read_text(encoding="utf-8"))
    for key in keys:
        data = data[key]
    return str(data)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", help="имя тега релиза, например v0.1.7")
    args = parser.parse_args()

    npm = read(ROOT / "package.json", "version")
    tauri = read(ROOT / "src-tauri" / "tauri.conf.json", "version")

    problems = []
    if npm != tauri:
        problems.append(
            f"package.json = {npm}, а src-tauri/tauri.conf.json = {tauri}"
        )

    if args.tag:
        tag = args.tag[1:] if args.tag.startswith("v") else args.tag
        if tag != npm:
            problems.append(f"тег = v{tag}, а package.json = {npm}")
        if tag != tauri:
            problems.append(f"тег = v{tag}, а src-tauri/tauri.conf.json = {tauri}")

    if problems:
        for line in problems:
            print(f"::error::версии разошлись: {line}")
        print("Поднимать версию: npm run version:set -- <patch|minor|major|X.Y.Z>")
        return 1

    print(f"версии совпадают: {npm}" + (f" (тег {args.tag})" if args.tag else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
