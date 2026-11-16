<script lang="ts" generics="T extends string">
	/**
	 * 選択肢のボタンの並び（モックの `.seg`）。押している選択肢は `aria-pressed` で示す。
	 * ラベル（`label`）は、まとまりの名前として `aria-labelledby` で結びつける。
	 */
	let {
		options,
		value,
		labelId,
		onchange
	}: {
		options: readonly { value: T; label: string; disabled?: boolean }[];
		value: T;
		labelId: string;
		onchange: (value: T) => void;
	} = $props();
</script>

<div class="seg" role="group" aria-labelledby={labelId}>
	{#each options as o (o.value)}
		<button
			type="button"
			aria-pressed={o.value === value}
			disabled={o.disabled}
			onclick={() => o.value !== value && onchange(o.value)}>{o.label}</button
		>
	{/each}
</div>

<style>
	.seg {
		display: inline-flex;
		align-self: flex-start;
		border: var(--pih-border-width) solid var(--cds-border-strong-01);
	}
	button {
		font: inherit;
		font-size: var(--cds-label-01-font-size);
		color: var(--cds-text-secondary);
		background: transparent;
		border: 0;
		border-right: var(--pih-border-width) solid var(--cds-border-strong-01);
		padding: var(--cds-spacing-02) var(--cds-spacing-04);
		cursor: pointer;
	}
	button:last-child {
		border-right: 0;
	}
	button[aria-pressed='true'] {
		background: var(--cds-layer-selected-01);
		color: var(--cds-text-primary);
	}
	button:hover:not(:disabled):not([aria-pressed='true']) {
		background: var(--cds-layer-hover-01);
	}
	button:disabled {
		color: var(--cds-text-disabled);
		cursor: not-allowed;
	}
</style>
