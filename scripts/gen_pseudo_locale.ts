// 開発時だけ使う疑似ロケール（en-XA）を作る。
// 英語の辞書から、文字列を約4割伸ばし、記号で囲んだ辞書を .pseudo/en-XA.json に書き、
// 疑似ロケールを加えた開発用の inlang プロジェクト .pseudo/project.inlang を作る（pathPattern は .pseudo/ からの相対）。
// 公開版のビルドは project.inlang（ja・en・zh-Hans）だけを使うため、疑似ロケールは含まれない。
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export const PSEUDO_LOCALE = 'en-XA';

const root = join(import.meta.dirname, '..');
const outDir = join(root, '.pseudo');

const ACCENTS: Record<string, string> = {
	a: 'á', c: 'ç', e: 'é', i: 'í', n: 'ñ', o: 'ó', u: 'ú', y: 'ý',
	A: 'Á', C: 'Ç', E: 'É', I: 'Í', N: 'Ñ', O: 'Ó', U: 'Ú', Y: 'Ý'
};

/** プレースホルダ（{name}）は変えずに、文字を置き換え、約4割伸ばして［］で囲む。 */
export function pseudo(text: string): string {
	const parts = text.split(/(\{[a-z_]+\})/g);
	const body = parts
		.map((p) => (/^\{[a-z_]+\}$/.test(p) ? p : [...p].map((ch) => ACCENTS[ch] ?? ch).join('')))
		.join('');
	const plain = text.replace(/\{[a-z_]+\}/g, '');
	const pad = '~'.repeat(Math.max(1, Math.round([...plain].length * 0.4)));
	return `［${body}${pad}］`;
}

/** .pseudo/ に疑似ロケールの辞書と開発用のプロジェクトを書く。 */
export function writePseudoLocale(): void {
	const en = JSON.parse(readFileSync(join(root, 'messages', 'en.json'), 'utf8')) as Record<string, string>;
	const out: Record<string, string> = { $schema: en.$schema };
	for (const [k, v] of Object.entries(en)) if (k !== '$schema') out[k] = pseudo(v);

	const settings = JSON.parse(readFileSync(join(root, 'project.inlang', 'settings.json'), 'utf8'));
	settings.locales = [...settings.locales, PSEUDO_LOCALE];
	settings['plugin.inlang.messageFormat'].pathPattern = ['../messages/{locale}.json', './{locale}.json'];

	mkdirSync(join(outDir, 'project.inlang'), { recursive: true });
	writeFileSync(join(outDir, `${PSEUDO_LOCALE}.json`), JSON.stringify(out, null, '\t') + '\n');
	writeFileSync(join(outDir, 'project.inlang', 'settings.json'), JSON.stringify(settings, null, '\t') + '\n');
}

if (import.meta.main) writePseudoLocale();
