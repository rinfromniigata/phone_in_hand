/**
 * ロケールの判定（純粋な関数。テストしやすいように Svelte や Tauri に依存させない）。
 */

/** 公開版で選べる言語。原文は ja。 */
export const PUBLIC_LOCALES = ['ja', 'en', 'zh-Hans'] as const;
export type PublicLocale = (typeof PUBLIC_LOCALES)[number];

/** 開発時だけ選べる疑似ロケール（scripts/gen_pseudo_locale.ts）。 */
export const PSEUDO_LOCALE = 'en-XA';

/** 設定の `general.language`。 */
export type LanguageSetting = 'auto' | PublicLocale;

export const FALLBACK_LOCALE: PublicLocale = 'ja';

/**
 * BCP 47 の言語タグを、対応する言語に寄せる。対応しないものは undefined。
 * 中国語は簡体字（zh-Hans、zh-CN、zh-SG、zh）だけを zh-Hans とし、繁体字は対応しない。
 */
export function matchLocale(tag: string): PublicLocale | undefined {
	const t = tag.trim().replace(/_/g, '-').toLowerCase();
	if (t === '') return undefined;
	const [lang, ...rest] = t.split('-');
	if (lang === 'ja') return 'ja';
	if (lang === 'en') return 'en';
	if (lang === 'zh') {
		if (rest.length === 0) return 'zh-Hans';
		if (rest.includes('hant') || rest.includes('tw') || rest.includes('hk') || rest.includes('mo')) {
			return undefined;
		}
		return 'zh-Hans';
	}
	return undefined;
}

/** OS の言語タグの候補から、最初に対応するものを選ぶ。どれも対応しなければ ja。 */
export function pickLocale(candidates: readonly (string | null | undefined)[]): PublicLocale {
	for (const c of candidates) {
		if (!c) continue;
		const m = matchLocale(c);
		if (m) return m;
	}
	return FALLBACK_LOCALE;
}
