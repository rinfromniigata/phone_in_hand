<script lang="ts">
	/**
	 * 映像のプレビュー（モックの `.preview`：16:9の枠の中に、スマホの縦長の画面）。
	 * 映像のフレームが届いていなければ、空の状態の文を出す。フレームの受け取りはT4-2でつなぐ。
	 */
	import type { Snippet } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';

	let {
		frameUrl = null,
		placeholder
	}: {
		/** 映像のフレームの画像のURL。なければ空の状態 */
		frameUrl?: string | null;
		/** 開発用の見本の絵（状態の見本のときだけ渡す） */
		placeholder?: Snippet;
	} = $props();
</script>

<div class="preview" class:empty={!frameUrl && !placeholder}>
	{#if frameUrl}
		<div class="phone-preview"><img src={frameUrl} alt="" /></div>
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
	}
	.preview.empty {
		color: var(--cds-text-helper);
		font-size: var(--cds-label-01-font-size);
	}
	.phone-preview {
		height: 100%;
		aspect-ratio: 9 / 19.5;
		margin: 0 auto;
		border-radius: var(--cds-spacing-05);
		border: var(--pih-border-width) solid var(--cds-border-strong-01);
		background: var(--cds-background);
		overflow: hidden;
	}
	img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
</style>
