<script lang="ts">
	/**
	 * 開発時だけ出す切り替え：状態の見本（S02a・S02b・S02c）と、疑似ロケールを含む表示の言語。
	 * 利用者向けの画面ではないため、文言は辞書に置かない（i18nの直書きの検出から除外している）。
	 */
	import { selectableLocales } from '#lib/i18n/locale.svelte.ts';
	import { app } from '#lib/state/app.svelte.ts';
	import { samples } from './samples.ts';

	let open = $state(false);
	let sampleId = $state('');
	let locale = $state('');

	$effect(() => {
		const s = samples.find((x) => x.id === sampleId);
		app.sample = s ? s.snapshot : null;
		app.samplePhoneKind = s ? s.phone_kind : null;
	});
	$effect(() => {
		app.localeOverride = locale === '' ? null : locale;
	});
</script>

<aside class="dev" class:open>
	<button type="button" class="toggle" onclick={() => (open = !open)} aria-expanded={open}>DEV</button>
	{#if open}
		<label>
			<span>State sample</span>
			<select bind:value={sampleId}>
				<option value="">live</option>
				{#each samples as s (s.id)}<option value={s.id}>{s.id}</option>{/each}
			</select>
		</label>
		<label>
			<span>Locale</span>
			<select bind:value={locale}>
				<option value="">settings</option>
				{#each selectableLocales() as l (l)}<option value={l}>{l}</option>{/each}
			</select>
		</label>
	{/if}
</aside>

<style>
	.dev {
		position: fixed;
		right: var(--cds-spacing-03);
		bottom: var(--cds-spacing-03);
		z-index: 9000;
		display: flex;
		gap: var(--cds-spacing-03);
		align-items: end;
		padding: var(--cds-spacing-02);
		background: var(--cds-layer-02);
		border: var(--pih-border-width) dashed var(--cds-support-warning);
		font-family: var(--pih-font-family-mono);
		font-size: var(--cds-label-01-font-size);
		color: var(--cds-text-primary);
	}
	label {
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-01);
	}
	.toggle,
	select {
		font: inherit;
		color: inherit;
		background: var(--cds-field-01);
		border: var(--pih-border-width) solid var(--cds-border-strong-01);
	}
</style>
