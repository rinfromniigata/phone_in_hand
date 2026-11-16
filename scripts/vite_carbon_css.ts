import type { Plugin } from 'vite';

/**
 * carbon-components-svelte の CSS を、このアプリの決まりに合わせて読み込むための Vite プラグイン。
 *
 * - `@font-face` を取り除く：Carbon の CSS は IBM の CDN から書体を読むため。書体は `static/fonts/` に同梱し、
 *   `src/lib/theme/fonts.css` で宣言する（外部から読み込まない）
 * - 書体の指定を `tokens.css` の `--pih-font-family-*` に置き換える：ロケールごとの書体の切り替えを部品にも効かせる
 * - 全体を `@layer carbon` に入れる：レイヤーに入らない `tokens.css` の `--cds-*` が、テーマの選択子の詳細度によらず優先される
 */
export function carbonCss(): Plugin {
	return {
		name: 'pih-carbon-css',
		enforce: 'pre',
		transform(code, id) {
			const path = id.split('?')[0].replaceAll('\\', '/');
			if (!path.endsWith('carbon-components-svelte/css/all.css')) return null;
			const css = code
				.replace(/@font-face\s*\{[^}]*\}/g, '')
				.replace(/font-family:\s*"?IBM Plex Sans"?,[^;}]*/g, 'font-family:var(--pih-font-family-sans)')
				.replace(/font-family:\s*"?IBM Plex Mono"?,[^;}]*/g, 'font-family:var(--pih-font-family-mono)');
			return { code: `@layer carbon{${css}}`, map: null };
		}
	};
}
