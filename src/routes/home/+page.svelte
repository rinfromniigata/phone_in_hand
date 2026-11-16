<script lang="ts">
	// ホーム（S02・S02a〜c）：VRに表示のスイッチ、接続の状態の5つの行、映像のプレビュー、位置を合わせ直すボタン
	import { Button } from 'carbon-components-svelte';
	import PageTitle from '#lib/components/PageTitle.svelte';
	import SwitchField from '#lib/components/SwitchField.svelte';
	import StatusRow from '#lib/components/StatusRow.svelte';
	import Banner from '#lib/components/Banner.svelte';
	import VideoPreview from '#lib/components/VideoPreview.svelte';
	import { app } from '#lib/state/app.svelte.ts';
	import { banners, statusRows } from '#lib/state/status_rows.ts';
	import { m } from '#lib/paraglide/messages.js';

	const rows = $derived(statusRows(app.snapshot, app.phoneKind));
	const notices = $derived(banners(app.snapshot, app.phoneKind));

	// 切り替えに失敗したら、スイッチを作り直して実際の状態に戻す
	let failed = $state(false);
	let switchKey = $state(0);

	async function toggleDisplay(enabled: boolean) {
		failed = false;
		try {
			await app.setDisplayEnabled(enabled);
		} catch (e) {
			console.error('VRへの表示を切り替えられません', e);
			failed = true;
			switchKey++;
		}
	}
</script>

<div class="head">
	<PageTitle>{m['nav.home']()}</PageTitle>
	{#key switchKey}
		<SwitchField checked={app.snapshot.display_enabled} label={m['home.show_panel']()} onchange={toggleDisplay} />
	{/key}
</div>

{#if failed}
	<Banner tone="err">{m['home.display_failed']()}</Banner>
{/if}
{#each notices as notice (notice.id)}
	<Banner tone={notice.tone}>{notice.text}</Banner>
{/each}

<div class="two-col">
	<section class="card" aria-labelledby="status-title">
		<h3 id="status-title">{m['home.status_title']()}</h3>
		<div class="status-list">
			{#each rows as row (row.id)}
				<StatusRow {row} />
			{/each}
		</div>
	</section>
	<div class="field">
		<span>{m['home.preview']()}</span>
		<VideoPreview placeholder={import.meta.env.DEV && app.sample && app.snapshot.video.state === 'running' ? fake : undefined} />
	</div>
</div>

<div class="row end">
	<!-- 位置合わせの画面（S01e）はT3-8でつなぐ。それまでは押せない -->
	<Button kind="secondary" disabled>{m['home.recalibrate']()}</Button>
</div>

{#snippet fake()}
	{#await import('#lib/dev/FakeScreen.svelte') then dev}
		<dev.default />
	{/await}
{/snippet}

<style>
	.head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--cds-spacing-05);
		flex-wrap: wrap;
	}
	.two-col {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		gap: var(--cds-spacing-05);
	}
	.card {
		background: var(--cds-layer-01);
		padding: var(--cds-spacing-05);
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-04);
		min-width: 0;
	}
	h3 {
		margin: 0;
		font-size: var(--cds-heading-compact-01-font-size);
		line-height: var(--cds-heading-compact-01-line-height);
		font-weight: var(--pih-font-weight-semibold);
	}
	.status-list {
		display: flex;
		flex-direction: column;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-03);
	}
	.field > span {
		font-size: var(--cds-label-01-font-size);
		line-height: var(--cds-label-01-line-height);
		letter-spacing: var(--cds-label-01-letter-spacing);
		color: var(--cds-text-secondary);
	}
	.row.end {
		display: flex;
		justify-content: flex-end;
		margin-top: auto;
	}
</style>
