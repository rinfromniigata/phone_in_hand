<script lang="ts">
	// 設定（S03・S03a〜c）。保存済みの設定が読み込まれてから、下書きを作って表示する
	import PageTitle from '#lib/components/PageTitle.svelte';
	import SettingsForm from '#lib/components/SettingsForm.svelte';
	import { app } from '#lib/state/app.svelte.ts';
	import { m } from '#lib/paraglide/messages.js';
</script>

<PageTitle>{m['nav.settings']()}</PageTitle>

{#if app.ready}
	<SettingsForm />
{/if}
