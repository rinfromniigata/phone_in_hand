/**
 * テーマの切り替え。tokens.css は `data-theme`（light・dark）で、Carbon の CSS は `theme`（white・g100）で
 * 色を切り替えるため、両方を同時に設定する。値そのものは tokens.css が正で、Carbon の CSS は
 * `@layer carbon` に入っているため、tokens.css にない変数だけが Carbon のテーマから補われる。
 */
export type ThemeSetting = 'system' | 'light' | 'dark';
export type ResolvedTheme = 'light' | 'dark';

const CARBON_THEME: Record<ResolvedTheme, string> = { light: 'white', dark: 'g100' };

const darkQuery = (): MediaQueryList | undefined =>
	typeof window === 'undefined' ? undefined : window.matchMedia('(prefers-color-scheme: dark)');

export function resolveTheme(setting: ThemeSetting): ResolvedTheme {
	if (setting !== 'system') return setting;
	return darkQuery()?.matches === false ? 'light' : 'dark';
}

function apply(theme: ResolvedTheme): void {
	const root = document.documentElement;
	root.dataset.theme = theme;
	root.setAttribute('theme', CARBON_THEME[theme]);
	root.style.colorScheme = theme;
}

let unsubscribe: (() => void) | undefined;

/** 設定に従ってテーマを適用する。`system` のときは OS の変更に追従する。 */
export function applyTheme(setting: ThemeSetting): void {
	unsubscribe?.();
	unsubscribe = undefined;
	apply(resolveTheme(setting));
	if (setting === 'system') {
		const q = darkQuery();
		if (!q) return;
		const onChange = () => apply(resolveTheme('system'));
		q.addEventListener('change', onChange);
		unsubscribe = () => q.removeEventListener('change', onChange);
	}
}
