// トークンの自動検査（checkコマンドの一部）。問題があれば一覧を出して終了コード1で終わる。
//
// - roadmap/specs/ の最新版と、コピー（src/lib/theme/tokens.css、docs/brand/）が一致すること
// - src/ のスタイル（.css、.svelte の <style>、style 属性）に、色・サイズの値が直書きされていないこと
//   （値の正本は tokens.css。tokens.css と生成物は除く）
import { readFileSync, readdirSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = join(import.meta.dirname, '..');
const specs = join(root, 'roadmap', 'specs');
const problems: string[] = [];
const rel = (p: string) => relative(root, p).replaceAll('\\', '/');
const read = (p: string) => readFileSync(p, 'utf8').replace(/\r\n/g, '\n');

/** roadmap/specs/ から `*_{name}_v{n}.{ext}` の最新版を探す */
function latestSpec(name: string, ext: string): string | undefined {
	const re = new RegExp(`_${name}_v(\\d+)\\.${ext}$`);
	return readdirSync(specs)
		.map((f) => ({ f, v: Number(re.exec(f)?.[1] ?? NaN) }))
		.filter((x) => !Number.isNaN(x.v))
		.sort((a, b) => b.v - a.v)[0]?.f;
}

// ---- 正本とコピーの一致 ----
const copies: [string, string, string][] = [
	['tokens', 'css', 'src/lib/theme/tokens.css'],
	['brand', 'md', 'docs/brand/brand.md'],
	['icon_master', 'svg', 'docs/brand/icon_master.svg']
];
for (const [name, ext, copy] of copies) {
	const spec = latestSpec(name, ext);
	if (!spec) {
		problems.push(`roadmap/specs/ に *_${name}_v{n}.${ext} がない`);
		continue;
	}
	let actual: string;
	try {
		actual = read(join(root, copy));
	} catch {
		problems.push(`${copy} がない（roadmap/specs/${spec} からコピーする）`);
		continue;
	}
	if (actual !== read(join(specs, spec))) problems.push(`${copy} が roadmap/specs/${spec} と一致しない`);
}

// ---- 色・サイズの直書き ----
const COLOR = /#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(/;
const SIZE = /(?<![\w-])(?:\d*\.)?\d+(?:px|rem|em|pt|pc|cm|mm|in|q)\b/i;
const NONZERO_SIZE = (v: string) => [...v.matchAll(new RegExp(SIZE, 'gi'))].some((m) => /[1-9]/.test(m[0]));

function checkCss(css: string, file: string, offsetLine: number): void {
	const clean = css.replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, ' '));
	// 宣言（property: value）の値だけを見る。セレクタの #id は対象外
	for (const m of clean.matchAll(/(^|[;{\s])([-\w]+)\s*:\s*([^;{}]+)/g)) {
		const [, , prop, value] = m;
		if (prop.startsWith('--')) continue; // 変数の定義は tokens.css だけにあるはずだが、別名づけは許す
		if (/^(src|unicode-range|font-display)$/.test(prop)) continue; // @font-face の記述
		const line = offsetLine + clean.slice(0, m.index).split('\n').length - 1 + (m[1] === '\n' ? 1 : 0);
		if (COLOR.test(value)) problems.push(`${file}:${line}: 色の直書き: ${prop}: ${value.trim()}`);
		else if (NONZERO_SIZE(value)) problems.push(`${file}:${line}: サイズの直書き: ${prop}: ${value.trim()}`);
	}
}

const excluded = (r: string) => r === 'src/lib/theme/tokens.css' || r.startsWith('src/lib/paraglide/');
for (const file of new Bun.Glob('src/**/*.{css,svelte}').scanSync({ cwd: root, absolute: true })) {
	const r = rel(file);
	if (excluded(r)) continue;
	const text = read(file);
	if (r.endsWith('.css')) {
		checkCss(text, r, 1);
		continue;
	}
	const lineAt = (i: number) => text.slice(0, i).split('\n').length;
	for (const m of text.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)) {
		checkCss(m[1], r, lineAt(m.index + m[0].indexOf('>') + 1));
	}
	for (const m of text.matchAll(/\bstyle(?::([-\w]+))?=(["'])([\s\S]*?)\2/g)) {
		const decl = m[1] ? `${m[1]}: ${m[3]}` : m[3];
		checkCss(decl, r, lineAt(m.index));
	}
}

if (problems.length > 0) {
	console.log(problems.join('\n'));
	process.exit(1);
}
