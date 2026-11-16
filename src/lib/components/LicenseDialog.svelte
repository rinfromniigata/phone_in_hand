<script lang="ts">
	/**
	 * ライセンス表示のダイアログ（スペック7章・9a節）。同梱するソフトウェアのライセンスと著作権表示を一覧する。
	 * 一覧は `src/lib/licenses/licenses.ts`。名称・ライセンス・著作権表示は原文のまま。
	 */
	import { Modal } from 'carbon-components-svelte';
	import { licenses } from '#lib/licenses/licenses.ts';
	import { m } from '#lib/paraglide/messages.js';

	let { open = $bindable(false) }: { open: boolean } = $props();
</script>

<Modal
	passiveModal
	bind:open
	size="sm"
	modalHeading={m['settings.general.licenses']()}
	iconDescription={m['common.close']()}
	on:close={() => (open = false)}
>
	<p class="desc">{m['settings.general.licenses_desc']()}</p>
	<ul class="list">
		{#each licenses as item (item.name)}
			<li>
				<div class="name">{item.name}{item.version ? ` ${item.version}` : ''}</div>
				<div class="meta mono">{item.license}</div>
				{#each item.copyright as line (line)}
					<div class="meta">{line}</div>
				{/each}
				<div class="meta mono">{item.url}</div>
			</li>
		{/each}
	</ul>
</Modal>

<style>
	.desc {
		margin: 0 0 var(--cds-spacing-05);
		color: var(--cds-text-secondary);
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
	}
	li {
		padding: var(--cds-spacing-04) 0;
		border-bottom: var(--pih-border-width) solid var(--cds-border-subtle-01);
	}
	li:last-child {
		border-bottom: 0;
	}
	.name {
		font-weight: var(--pih-font-weight-semibold);
	}
	.meta {
		color: var(--cds-text-helper);
		font-size: var(--cds-helper-text-01-font-size);
		line-height: var(--cds-helper-text-01-line-height);
		overflow-wrap: anywhere;
	}
	.mono {
		font-family: var(--pih-font-family-mono);
	}
</style>
