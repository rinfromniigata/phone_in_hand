import { paraglideVitePlugin } from '@inlang/paraglide-js';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin } from 'vite';
import { writePseudoLocale } from './scripts/gen_pseudo_locale.ts';
import { carbonCss } from './scripts/vite_carbon_css.ts';

// Tauri の開発サーバーの決まり（https://v2.tauri.app/start/frontend/sveltekit/）
const host = process.env.TAURI_DEV_HOST;

/** 開発時だけ、英語の辞書が変わったら疑似ロケールを作り直す。 */
function pseudoLocale(): Plugin {
	return {
		name: 'pih-pseudo-locale',
		apply: 'serve',
		configureServer(server) {
			server.watcher.add('messages/en.json');
			server.watcher.on('change', (file) => {
				if (file.replaceAll('\\', '/').endsWith('messages/en.json')) writePseudoLocale();
			});
		}
	};
}

export default defineConfig(({ command }) => {
	// 開発時は疑似ロケール（en-XA）を加えたプロジェクト、公開版のビルドは ja・en・zh-Hans だけのプロジェクト
	const dev = command === 'serve';
	if (dev) writePseudoLocale();
	return {
		plugins: [
			carbonCss(),
			pseudoLocale(),
			paraglideVitePlugin({
				project: dev ? './.pseudo/project.inlang' : './project.inlang',
				outdir: './src/lib/paraglide',
				strategy: ['globalVariable', 'baseLocale'],
				emitTsDeclarations: true
			}),
			sveltekit({
				// 静的出力（SPA）。Tauri の frontendDist は ../build
				adapter: adapter({ fallback: 'index.html' })
			})
		],
		clearScreen: false,
		server: {
			port: 5173,
			strictPort: true,
			host: host || false,
			hmr: host ? { protocol: 'ws', host, port: 5174 } : undefined,
			watch: { ignored: ['**/src-tauri/**', '**/crates/**', '**/target/**', '**/.pseudo/project.inlang/**'] }
		}
	};
});
