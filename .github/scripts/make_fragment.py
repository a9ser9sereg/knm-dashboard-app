"""Собирает фрагмент манифеста автообновления (updater latest.json) для
одной или нескольких platform-key, указывающих на один и тот же файл
обновления (macOS universal-билд отдаёт один .app.tar.gz на darwin-x86_64
и darwin-aarch64 сразу)."""
import argparse
import json

parser = argparse.ArgumentParser()
parser.add_argument("--out", required=True)
parser.add_argument("--url", required=True)
parser.add_argument("--sig-file", required=True)
parser.add_argument("--key", action="append", required=True)
args = parser.parse_args()

with open(args.sig_file, encoding="utf-8") as f:
    signature = f.read().strip()

fragment = {key: {"signature": signature, "url": args.url} for key in args.key}

with open(args.out, "w", encoding="utf-8") as f:
    json.dump(fragment, f)
