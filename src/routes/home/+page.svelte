<script lang="ts">
	// ホーム（S02）。中身はT2-1で実装する
	import PageTitle from '#lib/components/PageTitle.svelte';
	import { m } from '#lib/paraglide/messages.js';
</script>

<PageTitle>{m['nav.home']()}</PageTitle>
