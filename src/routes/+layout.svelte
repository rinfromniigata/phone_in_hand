<script lang="ts">
	import '#lib/theme/index.ts';
	import { applyTheme } from '#lib/theme/index.ts';
	import { applyLanguage } from '#lib/i18n/locale.svelte.ts';
	import { app } from '#lib/state/app.svelte.ts';
	import { api, inTauri } from '#lib/state/api.ts';
	import { renderOverlayImages } from '#lib/vr/overlay_images.ts';
	import { m } from '#lib/paraglide/messages.js';
	import RailNav from '#lib/components/RailNav.svelte';
	import { getCurrentWindow } from '@tauri-apps/api/window';

	let { children } = $props();

	app.init().catch((e) => console.error('状態を読み込めません', e));

	// VR内のチップと位置合わせの見本は、いまの言語とテーマで描いてRust側へ渡す（変わるたびに描き直す）
	let overlayQueued = false;
	function sendOverlayImages() {
		if (!inTauri() || overlayQueued) return;
		overlayQueued = true;
		requestAnimationFrame(() => {
			overlayQueued = false;
			renderOverlayImages()
				.then((images) => api.setOverlayImages(images))
				.catch((e) => console.error('VR内の表示の画像を渡せません', e));
		});
	}

	// 設定に従うテーマ（system は OS の変更に追従する）
	$effect(() => {
		applyTheme(app.settings.general.theme);
		sendOverlayImages();
	});
	$effect(() => {
		const q = window.matchMedia('(prefers-color-scheme: dark)');
		q.addEventListener('change', sendOverlayImages);
		return () => q.removeEventListener('change', sendOverlayImages);
	});

	// 設定に従う表示の言語と、ウィンドウのタイトル
	$effect(() => {
		const setting = app.language;
		applyLanguage(setting).then(() => {
			const title = m['app.name']();
			document.title = title;
			if (inTauri()) getCurrentWindow().setTitle(title).catch(() => {});
			sendOverlayImages();
		});
	});
</script>

<div class="shell">
	<RailNav />
	<main class="content">
		{@render children()}
	</main>
</div>

{#if import.meta.env.DEV}
	{#await import('#lib/dev/DevPanel.svelte') then dev}
		<dev.default />
	{/await}
{/if}

<style>
	.shell {
		display: flex;
		height: 100vh;
		min-height: 0;
		background: var(--cds-background);
	}
	.content {
		flex: 1;
		min-width: 0;
		overflow: auto;
		padding: var(--cds-spacing-06);
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-05);
	}
</style>
