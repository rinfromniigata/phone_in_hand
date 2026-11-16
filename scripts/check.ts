// checkコマンド（bun run check）。型・lint・テスト・i18n・トークンの検査を順に実行し、
// 通った項目は1行だけ、失敗した項目はその出力を表示する。CIも同じコマンドを呼ぶ。
import { join } from 'node:path';

const root = join(import.meta.dirname, '..');

type Step = { name: string; cmd: string[] };

const steps: Step[] = [
	// 公開版の辞書（ja・en・zh-Hans）で型を作る。開発サーバーが疑似ロケール入りで作った出力を上書きする
	{ name: 'paraglide（公開版の辞書をコンパイル）', cmd: ['bun', 'x', 'paraglide-js', 'compile', '--project', './project.inlang', '--outdir', './src/lib/paraglide'] },
	{ name: 'svelte-kit sync', cmd: ['bun', 'x', 'svelte-kit', 'sync'] },
	{ name: '型（svelte-check）', cmd: ['bun', 'x', 'svelte-check', '--fail-on-warnings', '--output', 'human'] },
	{ name: 'ESLint', cmd: ['bun', 'x', 'eslint', '.'] },
	{ name: 'テスト（UIとスクリプト）', cmd: ['bun', 'test', 'src', 'scripts'] },
	{ name: 'i18n（キーの過不足・プレースホルダ・未使用キー・直書き）', cmd: ['bun', 'scripts/check_i18n.ts'] },
	{ name: 'トークン（specsとの一致・色とサイズの直書き）', cmd: ['bun', 'scripts/check_tokens.ts'] },
	{ name: 'cargo fmt --check', cmd: ['cargo', 'fmt', '--all', '--check'] },
	{ name: 'cargo clippy -D warnings', cmd: ['cargo', 'clippy', '--workspace', '--all-targets', '--quiet', '--', '-D', 'warnings'] },
	{ name: 'cargo test', cmd: ['cargo', 'test', '--workspace', '--quiet'] }
];

/** 失敗したときに表示する行数の上限（長い出力は末尾だけ） */
const MAX_LINES = 80;

let failed = 0;
for (const step of steps) {
	const started = performance.now();
	const proc = Bun.spawnSync(step.cmd, { cwd: root, stdout: 'pipe', stderr: 'pipe', env: { ...process.env, FORCE_COLOR: '0', CARGO_TERM_COLOR: 'never' } });
	const sec = ((performance.now() - started) / 1000).toFixed(1);
	if (proc.exitCode === 0) {
		console.log(`✓ ${step.name}（${sec}s）`);
		continue;
	}
	failed++;
	console.log(`✗ ${step.name}（${sec}s）`);
	const out = (proc.stdout.toString() + proc.stderr.toString()).trimEnd().split(/\r?\n/);
	const shown = out.length > MAX_LINES ? ['…', ...out.slice(-MAX_LINES)] : out;
	console.log(shown.map((l) => `    ${l}`).join('\n'));
}

if (failed > 0) {
	console.log(`\n${failed} 項目が失敗しました`);
	process.exit(1);
}
console.log('\nすべて通りました');
