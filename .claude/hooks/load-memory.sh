#!/bin/bash
# Хук SessionStart: делает память доступной там, где сессия запущена.
#
# ЗАЧЕМ. Волт с памятью лежит в приватном репозитории, а адрес его нельзя
# записать в CLAUDE.md — этот репозиторий публичный. Из-за этого облачная
# сессия начиналась с просьбы назвать адрес: ровно в том сценарии, когда
# задача ставится с телефона, память не работала.
#
# Решение: адрес живёт в переменной окружения KNM_VAULT_REPO (в настройках
# окружения Claude Code, не в git), а хук сам приводит волт на место и
# сообщает, где он лежит.
#
# Порядок поиска:
#   1. Волт уже на диске (Mac) — используем его, ничего не качаем.
#   2. Задан KNM_VAULT_REPO — клонируем (мелкой копией) во временную папку.
#   3. Ничего нет — честно говорим, что памяти в этой сессии не будет.
#
# Ничего не выводит в stderr и всегда завершается успешно: сорванный клон не
# должен мешать началу работы.
set -uo pipefail

VAULT_LOCAL="${KNM_VAULT_DIR:-$HOME/Documents/IT/obsidian/Progect}"
VAULT_TMP="${KNM_VAULT_TMP:-/tmp/knm-vault}"

say() { printf '%s\n' "$*"; }

if [ -d "$VAULT_LOCAL/claude-memory" ]; then
  say "Волт с памятью на диске: $VAULT_LOCAL"
  say "Начни с claude-memory/MEMORY.md — это индекс, дальше по его указаниям."
  exit 0
fi

if [ -z "${KNM_VAULT_REPO:-}" ]; then
  say "Волта с памятью в этой сессии нет: локально не найден, переменная KNM_VAULT_REPO не задана."
  say "Спроси у Алексея адрес приватного репозитория волта и склонируй его в $VAULT_TMP."
  exit 0
fi

if [ -d "$VAULT_TMP/claude-memory" ]; then
  git -C "$VAULT_TMP" pull --quiet --rebase 2>/dev/null || true
else
  rm -rf "$VAULT_TMP"
  if ! git clone --quiet --depth 1 "$KNM_VAULT_REPO" "$VAULT_TMP" 2>/dev/null; then
    say "Волт склонировать не удалось (KNM_VAULT_REPO задан, но клон не прошёл)."
    say "Проверь адрес и доступ; память в этой сессии недоступна."
    exit 0
  fi
fi

if [ -d "$VAULT_TMP/claude-memory" ]; then
  say "Волт с памятью склонирован: $VAULT_TMP"
  say "Начни с claude-memory/MEMORY.md — это индекс, дальше по его указаниям."
  say "Правки памяти из облака нужно коммитить и пушить самому: фонового агента здесь нет."
else
  say "В $VAULT_TMP нет папки claude-memory — похоже, KNM_VAULT_REPO указывает не на волт."
fi
exit 0
