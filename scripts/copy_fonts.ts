// IBM Plex を node_modules から static/fonts/ へコピーする（同梱用。実行時に外部から読まない）。
// 版は package.json の @ibm/plex-* で固定する。コピー先は .gitignore で除外している。
import { copyFileSync, existsSync, mkdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..');
const out = join(root, 'static', 'fonts');

/** [パッケージ, パッケージ内のフォルダ, ファイル名の接頭辞] */
const families: [string, string, string][] = [
	['@ibm/plex-sans', 'fonts/complete/woff2', 'IBMPlexSans'],
	['@ibm/plex-sans-jp', 'fonts/complete/woff2/hinted', 'IBMPlexSansJP'],
	['@ibm/plex-sans-sc', 'fonts/complete/woff2/hinted', 'IBMPlexSansSC'],
	['@ibm/plex-mono', 'fonts/complete/woff2', 'IBMPlexMono']
];
const weights = ['Regular', 'SemiBold'];

mkdirSync(out, { recursive: true });
let copied = 0;
for (const [pkg, dir, prefix] of families) {
	const base = join(root, 'node_modules', pkg);
	for (const w of weights) {
		const name = `${prefix}-${w}.woff2`;
		const src = join(base, dir, name);
		const dst = join(out, name);
		if (existsSync(dst) && statSync(dst).size === statSync(src).size) continue;
		copyFileSync(src, dst);
		copied++;
	}
	const license = join(out, `${prefix}-LICENSE.txt`);
	if (!existsSync(license)) copyFileSync(join(base, 'LICENSE.txt'), license);
}
if (copied > 0) console.log(`fonts: ${copied} files copied to static/fonts/`);
