#!/usr/bin/env node
/**
 * Поднимает версию во всех файлах, где она записана.
 *
 * Раньше это делалось в два приёма: `npm version patch`, а потом руками
 * правился tauri.conf.json. Забытая вторая правка ничем себя не выдаёт до
 * самого релиза, поэтому все файлы двигаются одной командой.
 *
 *   npm run version:set -- patch      # 0.1.6 -> 0.1.7
 *   npm run version:set -- minor      # 0.1.6 -> 0.2.0
 *   npm run version:set -- major      # 0.1.6 -> 1.0.0
 *   npm run version:set -- 0.2.3      # ровно эта версия
 *
 * Тег и коммит не создаются намеренно: сначала посмотри diff, потом
 *   git commit -am "vX.Y.Z" && git tag vX.Y.Z && git push && git push --tags
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const SEMVER = /^\d+\.\d+\.\d+$/;

// У каждого файла свой синтаксис, поэтому и шаблон свой. Шаблон обязан
// находить ровно одно место — замена идёт по первому совпадению, без флага g.
// Он же служит проверкой: не нашёлся или нашёл чужую версию — значит файлы
// разъехались, и поднимать версию нельзя.
const TARGETS = [
  {
    rel: 'package.json',
    read: /"version"\s*:\s*"(\d+\.\d+\.\d+)"/,
  },
  {
    rel: 'src-tauri/tauri.conf.json',
    read: /"version"\s*:\s*"(\d+\.\d+\.\d+)"/,
  },
  {
    // Версия из [package]. У зависимостей поле version записано внутри
    // фигурных скобок (tauri = { version = "..." }) и под шаблон с началом
    // строки не подходит.
    //
    // Для сборки эта версия не используется — главной считается та, что в
    // tauri.conf.json. Двигаем её, чтобы `cargo` и «О программе» не
    // показывали разное, и чтобы не заводить ещё одно место с тихим
    // расхождением.
    rel: 'src-tauri/Cargo.toml',
    read: /^version = "(\d+\.\d+\.\d+)"$/m,
  },
  {
    // Блок самого пакета в локе. Без него `cargo check --locked` в CI падает:
    // Cargo.toml разъехался бы с Cargo.lock.
    rel: 'src-tauri/Cargo.lock',
    read: /name = "app"\r?\nversion = "(\d+\.\d+\.\d+)"/,
  },
];

const arg = process.argv[2];
if (!arg) {
  console.error('Укажи, куда двигать версию: patch | minor | major | X.Y.Z');
  process.exit(1);
}

/** Читает файл целиком, чтобы сохранить порядок ключей и форматирование. */
function load(target) {
  const path = join(ROOT, target.rel);
  const raw = readFileSync(path, 'utf8');
  const match = raw.match(target.read);
  if (!match) {
    console.error(`Не нашёл версию в ${target.rel} — правь вручную.`);
    process.exit(1);
  }
  return { ...target, path, raw, match, version: match[1] };
}

const files = TARGETS.map(load);

const current = files[0].version;
const mismatch = files.find((f) => f.version !== current);
if (mismatch) {
  console.error(
    `Версии уже разошлись: ${files[0].rel} = ${current}, ${mismatch.rel} = ${mismatch.version}.\n` +
    'Приведи их к одной вручную, потом поднимай.'
  );
  process.exit(1);
}

let next;
if (SEMVER.test(arg)) {
  next = arg;
} else {
  const [major, minor, patch] = current.split('.').map(Number);
  if (arg === 'patch') next = `${major}.${minor}.${patch + 1}`;
  else if (arg === 'minor') next = `${major}.${minor + 1}.0`;
  else if (arg === 'major') next = `${major + 1}.0.0`;
  else {
    console.error(`Не понимаю «${arg}». Ожидаю patch | minor | major | X.Y.Z`);
    process.exit(1);
  }
}

for (const { path, raw, match } of files) {
  // Точечная замена внутри найденного куска, а не перезапись файла целиком:
  // так не переедут отступы и не потеряется всё, что вокруг.
  const patched = match[0].replace(match[1], next);
  const updated = raw.slice(0, match.index) + patched + raw.slice(match.index + match[0].length);
  writeFileSync(path, updated);
}

console.log(`${current} -> ${next}`);
console.log(TARGETS.map((t) => `  обновлён ${t.rel}`).join('\n'));
console.log(`\nДальше: git commit -am "v${next}" && git tag v${next} && git push && git push --tags`);
