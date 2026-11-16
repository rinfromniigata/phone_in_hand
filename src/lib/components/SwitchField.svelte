<script lang="ts">
	/**
	 * スイッチ（モックの `.switch`：つまみの右にラベル）。CarbonのToggleを使い、
	 * 状態を示す既定の英語の文字（Off / On）は、ラベルの文字に置き換える。
	 */
	import { Toggle } from 'carbon-components-svelte';

	let {
		checked,
		label,
		disabled = false,
		onchange
	}: {
		checked: boolean;
		label: string;
		disabled?: boolean;
		onchange: (checked: boolean) => void;
	} = $props();
</script>

<div class="switch">
	<Toggle
		toggled={checked}
		labelText={label}
		hideLabel
		labelA={label}
		labelB={label}
		{disabled}
		on:toggle={(e) => onchange(e.detail.toggled)}
	/>
</div>

<style>
	.switch :global(.bx--toggle__text--off),
	.switch :global(.bx--toggle__text--on) {
		font-size: var(--cds-body-01-font-size);
		line-height: var(--cds-body-01-line-height);
	}
</style>
