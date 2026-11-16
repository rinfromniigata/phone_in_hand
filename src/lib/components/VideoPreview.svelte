<script lang="ts">
	/**
	 * 映像のプレビュー（モックの `.preview`：16:9の枠）。
	 *
	 * - `cropped`（ホーム）：切り抜いた範囲だけを、スマホの画面の形（`.phone-preview`）で出す
	 * - `frame`（映像のページ）：取り込んだ映像の全体を出し、VRに出す範囲を点線（`.crop`）で示す。
	 *   `overlay` に、範囲を動かす部品（CropEditor）を重ねられる
	 * 映像のフレームが届いていなければ、空の状態の文を出す。
	 */
	import type { Snippet } from 'svelte';
	import type { PreviewFrame, Rect } from '#lib/state/types.ts';
	import { m } from '#lib/paraglide/messages.js';

	let {
		frame = null,
		mode = 'cropped',
		crop = null,
		showCrop = true,
		placeholder,
		overlay
	}: {
		frame?: PreviewFrame | null;
		mode?: 'cropped' | 'frame';
		/** 点線で示す範囲（元の大きさの座標）。なければフレームの切り抜き */
		crop?: Rect | null;
		/** `frame` のときに点線を出すか（範囲を動かす部品を重ねるときは出さない） */
		showCrop?: boolean;
		/** 開発用の見本の絵（状態の見本のときだけ渡す） */
		placeholder?: Snippet;
		/** `frame` のときに映像に重ねるもの。引数は映像の元の大きさ */
		overlay?: Snippet<[PreviewFrame]>;
	} = $props();

	const src = $derived(frame ? `data:image/jpeg;base64,${frame.jpeg}` : null);
	const shownCrop = $derived(crop ?? frame?.crop ?? null);
	const pct = (v: number, of: number) => `${(v / of) * 100}%`;

	// 切り抜いた範囲だけを、キャンバスに描く
	let canvas = $state<HTMLCanvasElement>();
	$effect(() => {
		if (mode !== 'cropped' || !frame || !src || !canvas) return;
		const f = frame;
		const c = canvas;
		const img = new Image();
		let cancelled = false;
		img.src = src;
		img
			.decode()
			.then(() => {
				if (cancelled) return;
				const sx = f.width / f.frame.width;
				const sy = f.height / f.frame.height;
				const w = Math.max(1, Math.round(f.crop.width * sx));
				const h = Math.max(1, Math.round(f.crop.height * sy));
				c.width = w;
				c.height = h;
				c.getContext('2d')?.drawImage(img, f.crop.x * sx, f.crop.y * sy, w, h, 0, 0, w, h);
			})
			.catch(() => {});
		return () => {
			cancelled = true;
		};
	});
</script>

<div class="preview" class:empty={!frame && !placeholder}>
	{#if frame && mode === 'cropped'}
		<div class="phone-preview fit" style:--ratio={frame.crop.width / Math.max(1, frame.crop.height)}>
			<canvas bind:this={canvas}></canvas>
		</div>
	{:else if frame && src}
		<div class="frame" style:--ratio={frame.frame.width / frame.frame.height}>
			<img {src} alt="" />
			{#if shownCrop && showCrop}
				<div
					class="crop"
					style:left={pct(shownCrop.x, frame.frame.width)}
					style:top={pct(shownCrop.y, frame.frame.height)}
					style:width={pct(shownCrop.width, frame.frame.width)}
					style:height={pct(shownCrop.height, frame.frame.height)}
				></div>
			{/if}
			{#if overlay}{@render overlay(frame)}{/if}
		</div>
	{:else if placeholder}
		<div class="phone-preview">{@render placeholder()}</div>
	{:else}
		{m['home.preview_empty']()}
	{/if}
</div>

<style>
	.preview {
		aspect-ratio: 16 / 9;
		background: var(--cds-layer-01);
		display: flex;
		align-items: center;
		justify-content: center;
		position: relative;
		overflow: hidden;
		container-type: size;
	}
	.preview.empty {
		color: var(--cds-text-helper);
		font-size: var(--cds-label-01-font-size);
	}
	.phone-preview {
		height: 100%;
		max-width: 100%;
		aspect-ratio: 9 / 19.5;
		margin: 0 auto;
		border-radius: var(--cds-spacing-05);
		border: var(--pih-border-width) solid var(--cds-border-strong-01);
		background: var(--cds-background);
		overflow: hidden;
	}
	canvas,
	img {
		width: 100%;
		height: 100%;
		display: block;
	}
	/* 映像の縦横比を保ったまま、枠に収まる大きさにする */
	.phone-preview.fit {
		height: auto;
		margin: 0;
		aspect-ratio: var(--ratio);
		width: min(100cqw, 100cqh * var(--ratio));
	}
	.frame {
		position: relative;
		aspect-ratio: var(--ratio);
		width: min(100cqw, 100cqh * var(--ratio));
	}
	.crop {
		position: absolute;
		border: var(--pih-focus-width) dashed var(--cds-interactive);
		box-sizing: border-box;
		pointer-events: none;
	}
</style>
