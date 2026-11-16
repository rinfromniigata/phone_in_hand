// i18nの自動検査（checkコマンドの一部）。問題があれば一覧を出して終了コード1で終わる。
//
// - ロケール間のキーの過不足と、プレースホルダ（{name}）の一致
// - 未使用キー：どこからも引かれていないキー。ただし、
//   - `mobile.*` は Android のセンサー送信アプリ用（T8-2で string resources に変換する）
//   - scripts/i18n_pending.json のキーは、未実装の画面で使う予定のもの（使い始めたら消す）
// - UIの文字列の直書き：.svelte のテキストと、利用者に見える属性の文字列（src/lib/dev/ は開発用のため除く）
// キーの型検査（存在しないキーの参照）は Paraglide の型と svelte-check が受け持つ。
import { readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { parse } from 'svelte/compiler';

const root = join(import.meta.dirname, '..');
const LOCALES = ['ja', 'en', 'zh-Hans'] as const;
const BASE = 'ja';
const ANDROID_PREFIX = 'mobile.';
/** 利用者に見える文字列を取る属性（Carbonの部品の文字列のプロパティを含む） */
const TEXT_ATTRS = new Set([
	'aria-label',
	'aria-description',
	'title',
	'alt',
	'placeholder',
	'label',
	'labelText',
	'helperText',
	'invalidText',
	'warnText',
	'subtitle',
	'caption',
	'iconDescription',
	'primaryButtonText',
	'secondaryButtonText',
	'tooltipText'
]);

const problems: string[] = [];
const rel = (p: string) => relative(root, p).replaceAll('\\', '/');

function load(locale: string): Record<string, string> {
	const raw = JSON.parse(readFileSync(join(root, 'messages', `${locale}.json`), 'utf8'));
	delete raw.$schema;
	return raw;
}
const dicts = Object.fromEntries(LOCALES.map((l) => [l, load(l)])) as Record<string, Record<string, string>>;
const baseKeys = Object.keys(dicts[BASE]);

// ---- キーの過不足とプレースホルダ ----
const placeholders = (s: string) => [...s.matchAll(/\{([a-z_][a-z0-9_]*)\}/g)].map((m) => m[1]).sort().join(',');
for (const l of LOCALES) {
	if (l === BASE) continue;
	for (const k of baseKeys) {
		if (!(k in dicts[l])) problems.push(`messages/${l}.json: キーがない: ${k}`);
		else if (placeholders(dicts[l][k]) !== placeholders(dicts[BASE][k]))
			problems.push(`messages/${l}.json: プレースホルダが ja と違う: ${k}（ja: {${placeholders(dicts[BASE][k])}} / ${l}: {${placeholders(dicts[l][k])}}）`);
	}
	for (const k of Object.keys(dicts[l])) if (!(k in dicts[BASE])) problems.push(`messages/${l}.json: ja にないキー: ${k}`);
}
for (const k of baseKeys) {
	if (!/^[a-z0-9_]+(\.[a-z0-9_]+)*$/.test(k)) problems.push(`messages/ja.json: キーの形が規則と違う（ピリオド区切りのスネークケース）: ${k}`);
}

// ---- 使われているキー ----
const sources = [
	...new Bun.Glob('src/**/*.{svelte,ts}').scanSync({ cwd: root, absolute: true }),
	...new Bun.Glob('src-tauri/src/**/*.rs').scanSync({ cwd: root, absolute: true })
].filter((p) => !rel(p).startsWith('src/lib/paraglide/'));
const used = new Set<string>();
for (const file of sources) {
	const text = readFileSync(file, 'utf8');
	for (const m of text.matchAll(/m\[\s*['"]([a-z0-9_.]+)['"]\s*\]/g)) used.add(m[1]);
	if (file.endsWith('.rs')) for (const m of text.matchAll(/"([a-z0-9_]+(?:\.[a-z0-9_]+)+)"/g)) if (m[1] in dicts[BASE]) used.add(m[1]);
}
for (const k of used) if (!(k in dicts[BASE])) problems.push(`辞書にないキーを参照している: ${k}`);

const pendingDoc = JSON.parse(readFileSync(join(root, 'scripts', 'i18n_pending.json'), 'utf8')) as Record<string, unknown>;
const pending = new Set<string>();
for (const [group, keys] of Object.entries(pendingDoc)) {
	if (group.startsWith('$')) continue;
	for (const k of keys as string[]) {
		pending.add(k);
		if (!(k in dicts[BASE])) problems.push(`scripts/i18n_pending.json: 辞書にないキー: ${k}`);
		else if (used.has(k)) problems.push(`scripts/i18n_pending.json: 使われているため消す: ${k}（${group}）`);
	}
}
for (const k of baseKeys) {
	if (used.has(k) || pending.has(k) || k.startsWith(ANDROID_PREFIX)) continue;
	problems.push(`未使用のキー: ${k}`);
}

// ---- 直書きの検出 ----
type Node = { type?: string; [k: string]: unknown };
const hasLetters = (s: string) => /\p{L}/u.test(s);

function walk(node: unknown, visit: (n: Node) => void): void {
	if (Array.isArray(node)) return node.forEach((n) => walk(n, visit));
	if (!node || typeof node !== 'object') return;
	const n = node as Node;
	if (typeof n.type === 'string') visit(n);
	// 属性の値は visit で TEXT_ATTRS だけを調べる（class などは対象外）
	if (n.type === 'Attribute') return;
	for (const [k, v] of Object.entries(n)) {
		if (k === 'parent' || k === 'metadata') continue;
		if (n.type === 'Root' && (k === 'instance' || k === 'module' || k === 'css')) continue;
		if (n.type === 'RegularElement' && (n.name === 'style' || n.name === 'script')) continue;
		walk(v, visit);
	}
}

for (const file of new Bun.Glob('src/**/*.svelte').scanSync({ cwd: root, absolute: true })) {
	const r = rel(file);
	if (r.startsWith('src/lib/dev/')) continue;
	const source = readFileSync(file, 'utf8');
	const lineOf = (pos: number) => source.slice(0, pos).split('\n').length;
	const ast = parse(source, { modern: true });
	walk(ast.fragment, (n) => {
		if (n.type === 'Text' && hasLetters(String(n.data))) {
			problems.push(`${r}:${lineOf(Number(n.start))}: 文字列の直書き: ${String(n.data).trim().slice(0, 40)}`);
		}
		if (n.type === 'Attribute' && TEXT_ATTRS.has(String(n.name)) && Array.isArray(n.value)) {
			for (const v of n.value as Node[]) {
				if (v.type === 'Text' && hasLetters(String(v.data))) {
					problems.push(`${r}:${lineOf(Number(v.start))}: 属性 ${String(n.name)} の直書き: ${String(v.data).slice(0, 40)}`);
				}
			}
		}
	});
}

if (problems.length > 0) {
	console.log(problems.join('\n'));
	process.exit(1);
}
