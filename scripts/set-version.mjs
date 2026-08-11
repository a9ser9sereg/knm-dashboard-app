#!/usr/bin/env node
/**
 * Поднимает версию сразу в package.json и src-tauri/tauri.conf.json.
 *
 * Раньше это делалось в два приёма: `npm version patch`, а потом руками
 * правился tauri.conf.json. Забытая вторая правка ничем себя не выдаёт до
 * самого релиза, поэтому оба файла двигаются одной командой.
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
const FILES = ['package.json', 'src-tauri/tauri.conf.json'];
const SEMVER = /^\d+\.\d+\.\d+$/;

const arg = process.argv[2];
if (!arg) {
  console.error('Укажи, куда двигать версию: patch | minor | major | X.Y.Z');
  process.exit(1);
}

/** Читает файл целиком, чтобы сохранить порядок ключей и форматирование. */
function load(rel) {
  const path = join(ROOT, rel);
  const raw = readFileSync(path, 'utf8');
  return { path, raw, json: JSON.parse(raw) };
}

const files = FILES.map(load);

const current = files[0].json.version;
const mismatch = files.find((f) => f.json.version !== current);
if (mismatch) {
  console.error(
    `Версии уже разошлись: ${FILES[0]} = ${current}, ${mismatch.path} = ${mismatch.json.version}.\n` +
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

for (const { path, raw, json } of files) {
  // Точечная замена значения, а не JSON.stringify всего файла: так не
  // переедут отступы и не потеряются комментарии там, где они допустимы.
  const updated = raw.replace(
    new RegExp(`("version"\\s*:\\s*)"${json.version.replace(/\./g, '\\.')}"`),
    `$1"${next}"`
  );
  if (updated === raw) {
    console.error(`Не нашёл поле version в ${path} — правь вручную.`);
    process.exit(1);
  }
  writeFileSync(path, updated);
}

console.log(`${current} -> ${next}`);
console.log(FILES.map((f) => `  обновлён ${f}`).join('\n'));
console.log(`\nДальше: git commit -am "v${next}" && git tag v${next} && git push && git push --tags`);
