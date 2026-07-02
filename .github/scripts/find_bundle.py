"""Находит файл сборки Tauri по суффиксу пути (независимо от ОС/find —
'find' на macOS (BSD) не всегда находит файлы с кириллицей в имени внутри
src-tauri/target, glob.glob работает надёжно на всех платформах).
Печатает первый найденный путь или ничего (пусто), если нет совпадений."""
import argparse
import glob
import sys

# На Windows-раннере stdout по умолчанию в cp1252 — кириллица в имени файла
# (productName) валит print() UnicodeEncodeError. Форсируем UTF-8.
sys.stdout.reconfigure(encoding="utf-8")

parser = argparse.ArgumentParser()
parser.add_argument("suffix", help="например */bundle/dmg/*.dmg")
parser.add_argument("--root", default="src-tauri/target")
args = parser.parse_args()

matches = glob.glob(f"{args.root}/**/{args.suffix}", recursive=True)
if matches:
    print(matches[0])
