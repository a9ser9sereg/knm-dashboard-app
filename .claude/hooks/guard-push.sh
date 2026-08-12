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
# Правило касается только этого репозитория. Волт с памятью (`knm-vault`)
# живёт в ветке `main`, и облачная сессия обязана пушить его сама — иначе
# разговор не доедет ни до Mac, ни до памяти. Поэтому хук сначала выясняет,
# в какой репозиторий идёт push, и молчит, если это не knm-dashboard-app.
#
# Код возврата 2 = отказать в вызове и передать причину модели; всё остальное
# (в том числе любая внутренняя ошибка хука) пропускает команду дальше.
set -uo pipefail

PROTECTED_REPO="knm-dashboard-app"

[ "$(uname)" = "Darwin" ] && exit 0

payload=$(cat)
command=$(printf '%s' "$payload" | python3 -c \
  'import json,sys; print(json.load(sys.stdin).get("tool_input", {}).get("command", ""))' \
  2>/dev/null) || exit 0

# Между `git` и `push` могут стоять глобальные ключи: `git -C <путь> push`,
# `git -c user.name=… push`. Приводим их к простому `git push`, иначе такой
# вызов проходил бы мимо хука.
normalized=$(printf '%s' "$command" | sed -E \
  's/git([[:space:]]+(-[cC][[:space:]]+[^[:space:]]+|--git-dir=[^[:space:]]+|--work-tree=[^[:space:]]+))+[[:space:]]+push/git push/g')

case "$normalized" in
  *"git push"*) ;;
  *) exit 0 ;;
esac

# В какой репозиторий идёт push: явный `git -C <путь>`, ведущий `cd <путь> &&`
# или, если ничего не указано, каталог проекта.
repo_dir=$(printf '%s' "$command" | sed -n 's/.*git -C \([^ ]*\).*/\1/p')
[ -n "$repo_dir" ] || repo_dir=$(printf '%s' "$command" | sed -n 's/^[[:space:]]*cd \([^ &;|]*\).*/\1/p')
repo_dir="${repo_dir:-${CLAUDE_PROJECT_DIR:-.}}"

origin=$(git -C "$repo_dir" remote get-url origin 2>/dev/null) || exit 0
case "$origin" in
  *"$PROTECTED_REPO"*) ;;
  *) exit 0 ;;
esac

# Берём кусок команды от последнего `git push` до конца или до разделителя:
# в `git push -u origin claude/x && git checkout master` нас интересует push,
# а не то, что стоит за ним.
push_part=$(printf '%s' "$normalized" | sed -n 's/.*\(git push\)/\1/p' | sed 's/[;&|].*//')

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
branch=$(git -C "$repo_dir" symbolic-ref --short HEAD 2>/dev/null) || exit 0
case "$branch" in
  master|main) deny "$branch" ;;
esac

exit 0
