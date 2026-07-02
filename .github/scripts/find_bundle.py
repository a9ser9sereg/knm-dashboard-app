"""Находит файл сборки Tauri по суффиксу пути (независимо от ОС/find —
'find' на macOS (BSD) не всегда находит файлы с кириллицей в имени внутри
src-tauri/target, glob.glob работает надёжно на всех платформах).
Печатает первый найденный путь или ничего (пусто), если нет совпадений."""
import argparse
import glob

parser = argparse.ArgumentParser()
parser.add_argument("suffix", help="например */bundle/dmg/*.dmg")
parser.add_argument("--root", default="src-tauri/target")
args = parser.parse_args()

matches = glob.glob(f"{args.root}/**/{args.suffix}", recursive=True)
if matches:
    print(matches[0])
