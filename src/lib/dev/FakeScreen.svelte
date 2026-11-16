<script lang="ts">
	/** 開発用：映像のプレビューの見本の絵（モックの `.fake-screen`）。状態の見本のときだけ使う */
</script>

<div class="fake-screen">
	<div class="bar"></div>
	<div class="line m"></div>
	<div class="line"></div>
	<div class="line s"></div>
	<div class="block"></div>
	<div class="line m"></div>
	<div class="line s"></div>
</div>

<style>
	.fake-screen {
		width: 100%;
		height: 100%;
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-03);
		padding: var(--cds-spacing-04);
		box-sizing: border-box;
		background: var(--cds-layer-01);
	}
	.bar {
		height: var(--cds-spacing-06);
		border-radius: var(--pih-radius-full);
		background: var(--cds-layer-02);
	}
	.line {
		height: var(--cds-spacing-03);
		border-radius: var(--pih-radius-full);
		background: var(--cds-layer-02);
	}
	.line.s {
		width: 60%;
	}
	.line.m {
		width: 85%;
	}
	.block {
		height: var(--cds-spacing-09);
		background: var(--cds-layer-02);
	}
</style>
