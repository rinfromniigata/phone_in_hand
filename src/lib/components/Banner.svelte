<script lang="ts">
	/** 帯（モックの `.banner`）：左の色の線と点、本文。色だけでなく、本文で内容を伝える */
	import type { Snippet } from 'svelte';

	let { tone, children }: { tone: 'warn' | 'err' | 'info'; children: Snippet } = $props();
</script>

<div class="banner {tone}" role="status">
	<span class="dot" aria-hidden="true"></span>
	<span>{@render children()}</span>
</div>

<style>
	.banner {
		display: flex;
		gap: var(--cds-spacing-04);
		align-items: flex-start;
		padding: var(--cds-spacing-04) var(--cds-spacing-05);
		background: var(--cds-layer-01);
		border-left: var(--pih-accent-border-width) solid var(--cds-support-info);
		font-size: var(--cds-body-01-font-size);
		line-height: var(--cds-body-01-line-height);
	}
	.banner.warn {
		border-left-color: var(--cds-support-warning);
	}
	.banner.err {
		border-left-color: var(--cds-support-error);
	}
	.dot {
		width: var(--pih-status-dot);
		height: var(--pih-status-dot);
		border-radius: var(--pih-radius-full);
		flex: none;
		margin-top: var(--cds-spacing-02);
		background: var(--cds-support-info);
	}
	.warn .dot {
		background: var(--cds-support-warning);
	}
	.err .dot {
		background: var(--cds-support-error);
	}
</style>
