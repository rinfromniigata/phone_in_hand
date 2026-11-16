/**
 * ロケールの判定・保持・適用。
 *
 * - 設定の `general.language` が `auto` なら OS のロケールから選び、対応しない言語は ja にする
 * - 現在のロケールは Svelte の状態に持ち、Paraglide の `getLocale` をそれに差し替える。
 *   メッセージ関数がこの状態を読むため、切り替えると画面の文言が再描画される
 * - `<html lang>` を更新する。書体は tokens.css の `:root:lang()` で切り替わる
 * - 設定そのものの保存は、設定のストア（src/lib/state/）が受け持つ
 */
import { locales, overwriteGetLocale, type Locale } from '#lib/paraglide/runtime.js';
import { isTauri } from '@tauri-apps/api/core';
import { locale as osLocale } from '@tauri-apps/plugin-os';
import {
	FALLBACK_LOCALE,
	PSEUDO_LOCALE,
	PUBLIC_LOCALES,
	pickLocale,
	type LanguageSetting
} from './resolve.ts';

let current = $state<Locale>(FALLBACK_LOCALE);

overwriteGetLocale(() => current);

/** 開発時は疑似ロケールを加えた一覧、公開版は ja・en・zh-Hans。 */
export function selectableLocales(): readonly string[] {
	return import.meta.env.DEV && (locales as readonly string[]).includes(PSEUDO_LOCALE)
		? [...PUBLIC_LOCALES, PSEUDO_LOCALE]
		: PUBLIC_LOCALES;
}

export function currentLocale(): Locale {
	return current;
}

async function systemLocales(): Promise<string[]> {
	const found: string[] = [];
	if (isTauri()) {
		try {
			const l = await osLocale();
			if (l) found.push(l);
		} catch {
			// 取れなければ WebView の言語に任せる
		}
	}
	if (typeof navigator !== 'undefined') found.push(...navigator.languages);
	return found;
}

function apply(next: Locale): void {
	current = next;
	if (typeof document !== 'undefined') document.documentElement.lang = next;
}

/** 設定の値（auto・ja・en・zh-Hans。開発時は疑似ロケールも）に従ってロケールを適用する。 */
export async function applyLanguage(setting: LanguageSetting | string): Promise<Locale> {
	let next: string;
	if (setting === 'auto') next = pickLocale(await systemLocales());
	else if (selectableLocales().includes(setting)) next = setting;
	else next = FALLBACK_LOCALE;
	apply(next as Locale);
	return current;
}
