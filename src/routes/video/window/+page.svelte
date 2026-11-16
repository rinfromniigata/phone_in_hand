<script lang="ts">
	// 映像：ウィンドウを取り込む（S01g）を、設定の表示タブから単独で開いたとき。保存すると設定へ戻る
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { Button } from 'carbon-components-svelte';
	import Banner from '#lib/components/Banner.svelte';
	import WindowSource from '#lib/components/setup/WindowSource.svelte';
	import { app } from '#lib/state/app.svelte.ts';
	import type { Settings } from '#lib/state/types.ts';
	import { m } from '#lib/paraglide/messages.js';

	let saving = $state(false);
	let failed = $state(false);

	async function save(video: Settings['video']) {
		failed = false;
		saving = true;
		try {
			await app.updateSettings({
				video: { source: 'window', window: video.window, crop: video.crop }
			});
			await goto(resolve('/settings'));
		} catch (e) {
			console.error('設定を保存できません', e);
			failed = true;
		} finally {
			saving = false;
		}
	}
</script>

{#if app.ready}
	<WindowSource>
		{#snippet actions(video)}
			{#if failed}
				<Banner tone="err">{m['settings.save_failed']()}</Banner>
			{/if}
			<div class="row end">
				<Button kind="secondary" on:click={() => goto(resolve('/settings'))}>{m['common.back']()}</Button>
				<Button kind="primary" disabled={!video || saving} on:click={() => video && save(video)}>{m['common.save']()}</Button>
			</div>
		{/snippet}
	</WindowSource>
{/if}

<style>
	.row.end {
		display: flex;
		justify-content: flex-end;
		gap: var(--cds-spacing-01);
		margin-top: auto;
	}
</style>
