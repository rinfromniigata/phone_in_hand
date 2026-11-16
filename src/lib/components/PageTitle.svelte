<script lang="ts">
	/** 画面の見出し（モックの h2。heading-03） */
	import type { Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();
</script>

<h2>{@render children()}</h2>

<style>
	h2 {
		margin: 0;
		font-size: var(--cds-heading-03-font-size);
		line-height: var(--cds-heading-03-line-height);
		font-weight: var(--pih-font-weight-regular);
	}
</style>
