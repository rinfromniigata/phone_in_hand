<script lang="ts">
	/**
	 * UI Shellのサイドナビ（アイコンだけのレール。幅は --pih-rail-width）。
	 * 名前はツールチップ（ホバーとキーボードのフォーカスで出す）と aria-label で示す。
	 */
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { m } from '#lib/paraglide/messages.js';
	import Home from 'carbon-icons-svelte/lib/Home.svelte';
	import Settings from 'carbon-icons-svelte/lib/Settings.svelte';

	const items = [
		{ href: resolve('/home'), label: () => m['nav.home'](), icon: Home },
		{ href: resolve('/settings'), label: () => m['nav.settings'](), icon: Settings }
	];

	const isCurrent = (href: string) => page.url.pathname.startsWith(href);
</script>

<nav class="rail">
	{#each items as item (item.href)}
		{@const Icon = item.icon}
		<a
			class="nav-item"
			href={item.href}
			aria-label={item.label()}
			aria-current={isCurrent(item.href) ? 'page' : undefined}
		>
			<Icon class="nav-icon" />
			<span class="tooltip" aria-hidden="true">{item.label()}</span>
		</a>
	{/each}
</nav>

<style>
	.rail {
		width: var(--pih-rail-width);
		flex: none;
		display: flex;
		flex-direction: column;
		background: var(--cds-background);
		border-right: var(--pih-border-width) solid var(--cds-border-subtle-01);
	}
	.nav-item {
		position: relative;
		box-sizing: border-box;
		width: var(--pih-rail-width);
		height: var(--pih-rail-width);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--cds-text-secondary);
		text-decoration: none;
		border-left: var(--pih-accent-border-width) solid transparent;
		border-right: var(--pih-accent-border-width) solid transparent;
	}
	.nav-item:hover {
		background: var(--cds-background-hover);
		color: var(--cds-text-primary);
	}
	.nav-item:focus-visible {
		outline: var(--pih-focus-width) solid var(--cds-focus);
		outline-offset: calc(var(--pih-focus-width) * -1);
	}
	.nav-item[aria-current='page'] {
		background: var(--cds-layer-selected-01);
		color: var(--cds-text-primary);
		border-left-color: var(--cds-border-interactive);
	}
	.nav-item :global(.nav-icon) {
		width: var(--pih-icon-size);
		height: var(--pih-icon-size);
	}
	.tooltip {
		position: absolute;
		left: calc(100% + var(--cds-spacing-03));
		top: 50%;
		transform: translateY(-50%);
		z-index: 1;
		padding: var(--cds-spacing-02) var(--cds-spacing-05);
		background: var(--cds-background-inverse);
		color: var(--cds-text-inverse);
		font-size: var(--cds-body-01-font-size);
		line-height: var(--cds-body-01-line-height);
		white-space: nowrap;
		pointer-events: none;
		opacity: 0;
		visibility: hidden;
		transition: opacity var(--cds-duration-fast-02) var(--cds-easing-productive-standard);
	}
	.nav-item:hover .tooltip,
	.nav-item:focus-visible .tooltip {
		opacity: 1;
		visibility: visible;
	}
</style>
