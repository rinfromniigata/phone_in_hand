<script lang="ts">
	/**
	 * 映像：ウィンドウを取り込む（S01g）。ウィンドウの一覧、黒い余白の自動の切り抜き、プレビューと切り抜く範囲。
	 *
	 * - 選んだウィンドウと切り抜きは、保存する前からプレビューに使う（`start_video` に下書きを渡す）。
	 *   保存せずに離れたら、保存済みの映像の設定に戻す
	 * - 自動の切り抜きを切ると手動になり、プレビューの上で範囲を動かせる
	 * - 単独で開いたとき（設定の表示タブから）は、ステップ表示の代わりに「戻る」「保存」を出す。
	 *   初回セットアップの枠（T5-3）では、`steps` と `actions` を差し込む
	 */
	import { onDestroy } from 'svelte';
	import type { Snippet } from 'svelte';
	import { Select, SelectItem } from 'carbon-components-svelte';
	import Banner from '#lib/components/Banner.svelte';
	import CropEditor from '#lib/components/CropEditor.svelte';
	import PageTitle from '#lib/components/PageTitle.svelte';
	import Segmented from '#lib/components/Segmented.svelte';
	import SwitchField from '#lib/components/SwitchField.svelte';
	import VideoPreview from '#lib/components/VideoPreview.svelte';
	import { api, inTauri } from '#lib/state/api.ts';
	import { app } from '#lib/state/app.svelte.ts';
	import type { Rect, Settings, VideoSourceKind, WindowInfo } from '#lib/state/types.ts';
	import { m } from '#lib/paraglide/messages.js';

	type Video = Settings['video'];

	let {
		steps,
		actions
	}: {
		/** 初回セットアップのステップ表示 */
		steps?: Snippet;
		/** 下のボタン。引数は、保存する映像の設定（ウィンドウを選んでいなければ null） */
		actions: Snippet<[Video | null]>;
	} = $props();

	// ---- 下書き ----
	const saved = $state.snapshot(app.settings.video) as Video;
	let chosen = $state<WindowInfo | null>(
		saved.window.title || saved.window.executable
			? { title: saved.window.title ?? '', executable: saved.window.executable ?? '' }
			: null
	);
	let crop = $state<Video['crop']>({ ...saved.crop });

	const draft = $derived<Video | null>(
		chosen
			? {
					...saved,
					source: 'window',
					window: { title: chosen.title, executable: chosen.executable },
					crop: { mode: crop.mode, rect: crop.mode === 'manual' ? crop.rect : null }
				}
			: null
	);

	// ---- ウィンドウの一覧 ----
	let windows = $state<WindowInfo[]>([]);
	let listFailed = $state(false);

	async function refresh() {
		if (!inTauri()) return;
		try {
			windows = await api.listWindows();
			listFailed = false;
		} catch (e) {
			console.error('ウィンドウの一覧を得られません', e);
			listFailed = true;
		}
	}
	refresh();

	/** 一覧の中で選んでいる位置（なければ -1）。タイトルが変わっていても、実行ファイルが同じなら選ぶ */
	const selectedIndex = $derived.by(() => {
		if (!chosen) return -1;
		const c = chosen;
		const exact = windows.findIndex((w) => w.title === c.title && w.executable === c.executable);
		if (exact >= 0) return exact;
		return windows.findIndex((w) => c.executable !== '' && w.executable === c.executable);
	});

	function choose(e: Event) {
		const i = Number((e.currentTarget as HTMLSelectElement).value);
		chosen = Number.isInteger(i) && windows[i] ? { ...windows[i] } : null;
	}

	// ---- 切り抜き ----
	function setAuto(auto: boolean) {
		if (auto) {
			crop = { mode: 'auto', rect: crop.rect };
			return;
		}
		// 手動にした時点の範囲は、いま自動で切り抜いている範囲から始める
		const f = app.preview;
		crop = { mode: 'manual', rect: crop.rect ?? f?.crop ?? null };
	}

	// 手動にしたのに範囲がまだないときは、映像が届いた時点で全体から始める
	$effect(() => {
		const f = app.preview;
		if (crop.mode === 'manual' && !crop.rect && f) {
			crop = { mode: 'manual', rect: { x: 0, y: 0, width: f.frame.width, height: f.frame.height } };
		}
	});

	function setRect(rect: Rect) {
		crop = { mode: 'manual', rect };
	}

	// ---- 下書きでのプレビュー ----
	let timer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const d = draft ? ($state.snapshot(draft) as Video) : null;
		if (!d || !inTauri()) return;
		clearTimeout(timer);
		timer = setTimeout(() => {
			api.startVideo(d).catch((e) => console.error('プレビューを始められません', e));
		}, 150);
	});

	onDestroy(() => {
		clearTimeout(timer);
		// 保存せずに離れたときも、保存したときも、保存済みの設定で取り込み直す
		if (inTauri()) {
			api.startVideo($state.snapshot(app.settings.video) as Video).catch(() => {});
		}
	});

	// ---- 取り込み方法 ----
	const sources = $derived(
		[
			// HDMIキャプチャ（T4-2）とAndroidのミラーリング（T7-3）のページはあとでつなぐ
			{ value: 'hdmi' as VideoSourceKind, label: m['video.source.hdmi'](), disabled: true },
			...(app.phoneKind === 'iphone'
				? []
				: [{ value: 'mirror' as VideoSourceKind, label: m['video.source.mirror'](), disabled: true }]),
			{ value: 'window' as VideoSourceKind, label: m['video.source.window']() }
		] as const
	);
</script>

{#if steps}{@render steps()}{/if}
<PageTitle>{m['wizard.video.title']()}</PageTitle>
<p class="desc">{m['wizard.window.desc']()}</p>

<div class="field">
	<span id="video-source-label">{m['video.source_label']()}</span>
	<Segmented options={sources} value="window" labelId="video-source-label" onchange={() => {}} />
</div>

<div class="two-col">
	<div class="col">
		<Select
			labelText={m['wizard.window.list_label']()}
			selected={selectedIndex >= 0 ? String(selectedIndex) : ''}
			on:change={choose}
			on:focus={refresh}
		>
			{#if windows.length === 0}
				<SelectItem value="" text={m['wizard.window.list_empty']()} disabled />
			{:else}
				{#if selectedIndex < 0}
					<SelectItem value="" text={m['wizard.window.list_choose']()} disabled />
				{/if}
				{#each windows as w, i (i)}
					<SelectItem value={String(i)} text={w.title} />
				{/each}
			{/if}
		</Select>
		<SwitchField checked={crop.mode === 'auto'} label={m['wizard.video.crop_auto']()} onchange={setAuto} />
		<Banner tone="info">{m['wizard.window.hint']()}</Banner>
		{#if listFailed}
			<Banner tone="err">{m['home.detail.video_error']()}</Banner>
		{/if}
	</div>
	<div class="field">
		<span>{m['wizard.video.preview_label']()}</span>
		<VideoPreview
			frame={app.preview}
			mode="frame"
			crop={crop.mode === 'manual' ? crop.rect : null}
			showCrop={crop.mode === 'auto'}
		>
			{#snippet overlay(f)}
				{#if crop.mode === 'manual' && crop.rect}
					<CropEditor size={f.frame} rect={crop.rect} onchange={setRect} />
				{/if}
			{/snippet}
		</VideoPreview>
		<span class="muted">{m['wizard.video.crop_hint']()}</span>
	</div>
</div>

{@render actions(draft)}

<style>
	.desc {
		margin: 0;
		color: var(--cds-text-secondary);
	}
	.two-col {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		gap: var(--cds-spacing-05);
	}
	.col {
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-05);
	}
	/* CarbonのSelectの外枠は伸びるため、内容の高さにとどめる */
	.col :global(.bx--form-item) {
		flex: none;
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
	.muted {
		color: var(--cds-text-helper);
		font-size: var(--cds-helper-text-01-font-size);
		line-height: var(--cds-helper-text-01-line-height);
	}
</style>
