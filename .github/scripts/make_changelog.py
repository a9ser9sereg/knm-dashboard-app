"""Строит CHANGELOG.json (история версий) из аннотаций git-тегов vX.Y.Z —
источник данных для блока «История версий» в панели «Настройки» десктоп-
приложения. Требует полной истории тегов (checkout с fetch-depth: 0)."""
import argparse
import json
import subprocess

FIELD_SEP = "\x1f"
RECORD_SEP = "\x1e"

parser = argparse.ArgumentParser()
parser.add_argument("--out", required=True)
args = parser.parse_args()

fmt = f"%(refname:short){FIELD_SEP}%(creatordate:short){FIELD_SEP}%(contents){RECORD_SEP}"
raw = subprocess.run(
    ["git", "for-each-ref", "--sort=-creatordate", f"--format={fmt}", "refs/tags/v*"],
    capture_output=True, text=True, check=True, encoding="utf-8",
).stdout

entries = []
for record in raw.split(RECORD_SEP):
    record = record.strip("\n")
    if not record:
        continue
    version, date, notes = record.split(FIELD_SEP, 2)
    entries.append({
        "version": version.lstrip("v"),
        "date": date,
        "notes": notes.strip(),
    })

with open(args.out, "w", encoding="utf-8") as f:
    json.dump(entries, f, ensure_ascii=False, indent=2)
