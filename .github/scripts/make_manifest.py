"""Сливает фрагменты платформ (windows.json + macos.json) в единый
latest.json для tauri-plugin-updater."""
import argparse
import datetime
import json

parser = argparse.ArgumentParser()
parser.add_argument("--version", required=True)
parser.add_argument("--out", required=True)
parser.add_argument("fragments", nargs="+")
args = parser.parse_args()

platforms = {}
for path in args.fragments:
    with open(path, encoding="utf-8") as f:
        platforms.update(json.load(f))

manifest = {
    "version": args.version,
    "notes": f"Дашборд ГСН МО {args.version}",
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": platforms,
}

with open(args.out, "w", encoding="utf-8") as f:
    json.dump(manifest, f, ensure_ascii=False, indent=2)
