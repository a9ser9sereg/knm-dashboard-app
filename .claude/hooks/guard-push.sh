#!/bin/bash
# Хук PreToolUse (Bash): не пускает push в master из облачной сессии.
#
# ЗАЧЕМ. В CLAUDE.md записано: правки из облака идут в ветку claude/<тема>,
# слияние в master — с Mac, после просмотра. Правило верное, но выполняется
# только пока о нём помнят, а цена ошибки — непросмотренный код сразу в
# основной ветке, откуда его забирает релизный workflow.
#
# На Mac хук не мешает: там Алексей мержит сам и видит, что делает.
#
# Код возврата 2 = отказать в вызове и передать причину модели; всё остальное
# (в том числе любая внутренняя ошибка хука) пропускает команду дальше.
set -uo pipefail

[ "$(uname)" = "Darwin" ] && exit 0

payload=$(cat)
command=$(printf '%s' "$payload" | python3 -c \
  'import json,sys; print(json.load(sys.stdin).get("tool_input", {}).get("command", ""))' \
  2>/dev/null) || exit 0

case "$command" in
  *"git push"*) ;;
  *) exit 0 ;;
esac

# Берём кусок команды от последнего `git push` до конца или до разделителя:
# в `git push -u origin claude/x && git checkout master` нас интересует push,
# а не то, что стоит за ним.
push_part=$(printf '%s' "$command" | sed -n 's/.*\(git push\)/\1/p' | sed 's/[;&|].*//')

deny() {
  echo "Пуш в $1 из облака запрещён (правило из CLAUDE.md)." >&2
  echo "Работай в ветке claude/<тема> и пушь в неё; слияние в $1 — с Mac, после просмотра." >&2
  exit 2
}

# Ветка названа явно: `origin master`, `HEAD:master`, `:main`. Отделяем именно
# имя ветки, чтобы не спотыкаться о claude/fix-master-bug.
if printf '%s' "$push_part" | grep -Eq '(^|[[:space:]:])(master|main)([[:space:]]|$)'; then
  deny master
fi

# Ветка не названа — push пойдёт в текущую. Проверяем, где стоим.
branch=$(git -C "${CLAUDE_PROJECT_DIR:-.}" symbolic-ref --short HEAD 2>/dev/null) || exit 0
case "$branch" in
  master|main) deny "$branch" ;;
esac

exit 0
