<script lang="ts" generics="Id extends string">
	/**
	 * タブ（モックの `.tabs`）。矢印キー・Home・Endで移動し、フォーカスしたタブを選ぶ（ARIAのtabパターン）。
	 * 中身は、選んだタブのIDを受け取る snippet で描く。
	 */
	import type { Snippet } from 'svelte';

	let {
		tabs,
		selected = $bindable(),
		label,
		panel
	}: {
		tabs: readonly { id: Id; label: string }[];
		selected: Id;
		/** タブの並びの名前（スクリーンリーダー用） */
		label: string;
		panel: Snippet<[Id]>;
	} = $props();

	const uid = $props.id();
	const tabId = (id: Id) => `${uid}-tab-${id}`;
	const panelId = (id: Id) => `${uid}-panel-${id}`;

	function onkeydown(e: KeyboardEvent) {
		const i = tabs.findIndex((t) => t.id === selected);
		let next = -1;
		if (e.key === 'ArrowRight') next = (i + 1) % tabs.length;
		else if (e.key === 'ArrowLeft') next = (i - 1 + tabs.length) % tabs.length;
		else if (e.key === 'Home') next = 0;
		else if (e.key === 'End') next = tabs.length - 1;
		if (next < 0) return;
		e.preventDefault();
		selected = tabs[next].id;
		document.getElementById(tabId(selected))?.focus();
	}
</script>

<div class="tabs" role="tablist" aria-label={label} tabindex="-1" {onkeydown}>
	{#each tabs as t (t.id)}
		<button
			type="button"
			role="tab"
			class="tab"
			id={tabId(t.id)}
			aria-selected={selected === t.id}
			aria-controls={panelId(t.id)}
			tabindex={selected === t.id ? 0 : -1}
			onclick={() => (selected = t.id)}
		>
			{t.label}
		</button>
	{/each}
</div>
<div role="tabpanel" id={panelId(selected)} aria-labelledby={tabId(selected)} class="panel">
	{@render panel(selected)}
</div>

<style>
	.tabs {
		display: flex;
	}
	.tabs::after {
		content: '';
		flex: 1;
		border-bottom: var(--pih-focus-width) solid var(--cds-border-subtle-01);
	}
	.tab {
		font: inherit;
		background: transparent;
		border: 0;
		border-bottom: var(--pih-focus-width) solid var(--cds-border-subtle-01);
		padding: var(--cds-spacing-04) var(--cds-spacing-05) var(--cds-spacing-03);
		color: var(--cds-text-secondary);
		cursor: pointer;
		white-space: nowrap;
	}
	.tab:hover {
		background: var(--cds-background-hover);
	}
	.tab[aria-selected='true'] {
		color: var(--cds-text-primary);
		border-bottom-color: var(--cds-border-interactive);
		font-weight: var(--pih-font-weight-semibold);
	}
	.panel {
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-05);
		min-width: 0;
	}
</style>
