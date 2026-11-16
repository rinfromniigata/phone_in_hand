<script lang="ts">
	/**
	 * 位置合わせ（S01e）。位置の基準とスマホを付ける側を選び、VRの中で位置を合わせる。
	 *
	 * - 「位置合わせを始める」で、VRの中に画面の見本を出す。もう一方のコントローラーのグリップでつかんで動かし、
	 *   トリガーで決定すると、結果（calibration.json）と基準・側が保存され、`onfinished` を呼ぶ（自動で進む）
	 * - 途中で「戻る」・ページを離れると、やめて前の結果を残す
	 * - ハンドトラッキングは、手の追跡（T9-4）ができるまで選べない
	 * - 初回セットアップの枠（T5-3）では、`steps` にステップ表示を差し込む
	 */
	import { onDestroy, onMount } from 'svelte';
	import type { Snippet } from 'svelte';
	import { Button, InlineLoading } from 'carbon-components-svelte';
	import Banner from '#lib/components/Banner.svelte';
	import PageTitle from '#lib/components/PageTitle.svelte';
	import Segmented from '#lib/components/Segmented.svelte';
	import { api, inTauri } from '#lib/state/api.ts';
	import { app } from '#lib/state/app.svelte.ts';
	import type { PoseSourceKind, Side } from '#lib/state/types.ts';
	import { m } from '#lib/paraglide/messages.js';

	let {
		steps,
		onfinished,
		onback
	}: { steps?: Snippet; onfinished: () => void; onback: () => void } = $props();

	let source = $state<PoseSourceKind>(app.settings.pose.source);
	let side = $state<Side>(app.settings.pose.side);
	let running = $state(false);
	let failed = $state(false);

	const sources = $derived([
		{ value: 'controller' as PoseSourceKind, label: m['pose.source.controller']() },
		{ value: 'hand' as PoseSourceKind, label: m['pose.source.hand'](), disabled: true }
	]);
	const sides = $derived([
		{ value: 'left' as Side, label: m['side.left']() },
		{ value: 'right' as Side, label: m['side.right']() }
	]);

	let unlisten: (() => void) | undefined;
	onMount(() => {
		if (!inTauri()) return;
		api
			.onCalibrationProgress((p) => {
				switch (p.state) {
					case 'running':
						running = true;
						failed = false;
						break;
					case 'finished':
						running = false;
						onfinished();
						break;
					case 'cancelled':
						running = false;
						break;
					case 'failed':
						running = false;
						failed = true;
						break;
				}
			})
			.then((u) => (unlisten = u));
	});

	onDestroy(() => {
		unlisten?.();
		if (running) api.cancelCalibration().catch(() => {});
	});

	async function start() {
		failed = false;
		if (!inTauri()) {
			failed = true;
			return;
		}
		try {
			await api.startCalibration({ source, side });
			running = true;
		} catch (e) {
			console.error('位置合わせを始められません', e);
			failed = true;
		}
	}

	async function back() {
		if (running) {
			running = false;
			await api.cancelCalibration().catch(() => {});
		}
		onback();
	}
</script>

{#if steps}{@render steps()}{/if}
<PageTitle>{m['wizard.calibrate.title']()}</PageTitle>
<p class="desc">{m['wizard.calibrate.desc']()}</p>

<div class="field">
	<span id="calibrate-source-label">{m['wizard.calibrate.source_label']()}</span>
	<Segmented
		options={sources}
		value={source}
		labelId="calibrate-source-label"
		onchange={(v) => (source = v)}
	/>
</div>
<div class="field">
	<span id="calibrate-side-label">{m['wizard.calibrate.side_label']()}</span>
	<Segmented options={sides} value={side} labelId="calibrate-side-label" onchange={(v) => (side = v)} />
</div>

{#if running}
	<div class="card">
		<InlineLoading description={m['wizard.calibrate.in_progress']()} />
		<span class="muted">{m['wizard.calibrate.vr_hint']()}</span>
	</div>
{/if}
{#if failed}
	<Banner tone="err">{m['wizard.calibrate.failed']()}</Banner>
{/if}

<div class="row end">
	<Button kind="secondary" on:click={back}>{m['common.back']()}</Button>
	<Button kind="primary" disabled={running} on:click={start}>{m['wizard.calibrate.start']()}</Button>
</div>

<style>
	.desc {
		margin: 0;
		color: var(--cds-text-secondary);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-03);
	}
	.field > span:first-child {
		font-size: var(--cds-label-01-font-size);
		line-height: var(--cds-label-01-line-height);
		letter-spacing: var(--cds-label-01-letter-spacing);
		color: var(--cds-text-secondary);
	}
	.card {
		background: var(--cds-layer-01);
		padding: var(--cds-spacing-05);
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-04);
	}
	.muted {
		color: var(--cds-text-helper);
		font-size: var(--cds-helper-text-01-font-size);
		line-height: var(--cds-helper-text-01-line-height);
	}
	.row.end {
		display: flex;
		justify-content: flex-end;
		gap: var(--cds-spacing-01);
		margin-top: auto;
	}
</style>
