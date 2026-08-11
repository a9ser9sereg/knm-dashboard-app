"""Сливает фрагменты платформ (windows.json + macos.json) в единый
latest.json для tauri-plugin-updater."""
import argparse
import datetime
import json
import re

# Заметки показываются пользователю в диалоге «Доступна новая версия».
# Диалог — модальное окно, поэтому длинный текст в него не помещается:
# берём начало и обрезаем по границе строки.
NOTES_LIMIT = 700

parser = argparse.ArgumentParser()
parser.add_argument("--version", required=True)
parser.add_argument("--out", required=True)
parser.add_argument(
    "--notes-file",
    help="файл с описанием релиза (сообщение тега). Пусто или нет файла — "
         "в манифест уйдёт одно название с версией, как было раньше.",
)
parser.add_argument("fragments", nargs="+")
args = parser.parse_args()


def read_notes(path: str, version: str) -> str:
    """Готовит текст тега к показу в диалоге обновления."""
    try:
        with open(path, encoding="utf-8") as f:
            text = f.read().strip()
    except OSError:
        return ""
    # Первая строка сообщения тега обычно начинается с самой версии
    # («v0.1.7: чиним трей») — в диалоге она уже есть строкой выше.
    text = re.sub(rf"^v?{re.escape(version)}\s*[:\-—]?\s*", "", text)
    if len(text) > NOTES_LIMIT:
        cut = text[:NOTES_LIMIT].rsplit("\n", 1)[0].rstrip()
        text = f"{cut}\n…"
    return text.strip()


platforms = {}
for path in args.fragments:
    with open(path, encoding="utf-8") as f:
        platforms.update(json.load(f))

notes = read_notes(args.notes_file, args.version) if args.notes_file else ""

manifest = {
    "version": args.version,
    "notes": notes or f"Дашборд ГСН МО {args.version}",
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": platforms,
}

with open(args.out, "w", encoding="utf-8") as f:
    json.dump(manifest, f, ensure_ascii=False, indent=2)
