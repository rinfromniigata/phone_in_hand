<script lang="ts">
	// 位置合わせ（S01e）を、ホームの「位置を合わせ直す」から単独で開いたとき。終わるとホームへ戻る
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import Calibrate from '#lib/components/setup/Calibrate.svelte';
	import { app } from '#lib/state/app.svelte.ts';

	const home = () => goto(resolve('/home'));
</script>

{#if app.ready}
	<Calibrate onfinished={home} onback={home} />
{/if}
