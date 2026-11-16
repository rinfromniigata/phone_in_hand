<script lang="ts">
	/** 状態の行（モックの `.status-row`）：色の点・名前と補足・状態の文字。状態は色と文字の両方で示す */
	import type { StatusRow } from '#lib/state/status_rows.ts';

	let { row }: { row: StatusRow } = $props();
</script>

<div class="status-row" data-status={row.id} data-tone={row.tone}>
	<span class="dot {row.tone}" aria-hidden="true"></span>
	<span>
		<span class="name">{row.name}</span>
		{#if row.detail}<span class="detail">{row.detail}</span>{/if}
	</span>
	<span class="state-text">{row.state}</span>
</div>

<style>
	.status-row {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		gap: var(--cds-spacing-04);
		align-items: center;
		padding: var(--cds-spacing-04) 0;
		border-bottom: var(--pih-border-width) solid var(--cds-layer-02);
	}
	.status-row:last-child {
		border-bottom: 0;
	}
	.detail {
		display: block;
		color: var(--cds-text-helper);
		font-size: var(--cds-helper-text-01-font-size);
		line-height: var(--cds-helper-text-01-line-height);
		overflow-wrap: anywhere;
	}
	.state-text {
		font-size: var(--cds-label-01-font-size);
		color: var(--cds-text-secondary);
		text-align: right;
	}
	.dot {
		width: var(--pih-status-dot);
		height: var(--pih-status-dot);
		border-radius: var(--pih-radius-full);
		flex: none;
		background: var(--cds-border-strong-01);
	}
	.dot.ok {
		background: var(--cds-support-success);
	}
	.dot.warn {
		background: var(--cds-support-warning);
	}
	.dot.err {
		background: var(--cds-support-error);
	}
</style>
