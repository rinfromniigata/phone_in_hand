// ESLint（flat config）。eslint-plugin-svelte の README の TypeScript 向けの構成に従う
import js from '@eslint/js';
import paraglide from '@inlang/paraglide-js/eslint';
import svelte from 'eslint-plugin-svelte';
import { defineConfig } from 'eslint/config';
import globals from 'globals';
import ts from 'typescript-eslint';

export default defineConfig(
	{
		ignores: [
			'build/',
			'.svelte-kit/',
			'.pseudo/',
			'target/',
			'src-tauri/',
			'crates/',
			'node_modules/',
			'src/lib/paraglide/',
			'static/'
		]
	},
	js.configs.recommended,
	ts.configs.recommended,
	svelte.configs.recommended,
	{
		languageOptions: {
			parserOptions: { extraFileExtensions: ['.svelte'] },
			globals: { ...globals.browser, ...globals.node }
		}
	},
	{
		files: ['**/*.svelte', '**/*.svelte.ts'],
		languageOptions: {
			parserOptions: { projectService: true, parser: ts.parser }
		}
	},
	{
		plugins: { paraglide },
		rules: {
			// 辞書のキーは文字列の定数で引く（未使用キーの検査を効かせるため）
			'paraglide/no-dynamic-message-access': 'error'
		}
	}
);
