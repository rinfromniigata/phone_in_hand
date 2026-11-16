<script lang="ts">
	/**
	 * 手動の切り抜きの範囲を動かす部品。映像のプレビュー（VideoPreview の `frame`）に重ねる。
	 *
	 * - 範囲の内側をドラッグすると全体が動き、4つの辺のつまみをドラッグすると辺が動く
	 * - つまみはキーボードでも動かせる（矢印キーで1%、Shiftを押しながらで5%）
	 * 座標は取り込んだ映像の元の大きさ（画素）で扱う。
	 */
	import type { Rect, Size } from '#lib/state/types.ts';
	import { m } from '#lib/paraglide/messages.js';

	let { size, rect, onchange }: { size: Size; rect: Rect; onchange: (rect: Rect) => void } = $props();

	type Edge = 'top' | 'right' | 'bottom' | 'left';
	/** 範囲の最小の大きさ（元の大きさに対する割合） */
	const MIN_SHARE = 0.05;
	const STEP_SHARE = 0.01;
	const BIG_STEP_SHARE = 0.05;

	let box = $state<HTMLDivElement>();

	const pct = (v: number, of: number) => `${(v / of) * 100}%`;
	const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

	/** 辺を動かした矩形（反対の辺は動かさない） */
	function moveEdge(r: Rect, edge: Edge, to: number): Rect {
		const minW = Math.max(1, Math.round(size.width * MIN_SHARE));
		const minH = Math.max(1, Math.round(size.height * MIN_SHARE));
		const right = r.x + r.width;
		const bottom = r.y + r.height;
		switch (edge) {
			case 'left': {
				const x = clamp(Math.round(to), 0, right - minW);
				return { ...r, x, width: right - x };
			}
			case 'right': {
				const rr = clamp(Math.round(to), r.x + minW, size.width);
				return { ...r, width: rr - r.x };
			}
			case 'top': {
				const y = clamp(Math.round(to), 0, bottom - minH);
				return { ...r, y, height: bottom - y };
			}
			case 'bottom': {
				const bb = clamp(Math.round(to), r.y + minH, size.height);
				return { ...r, height: bb - r.y };
			}
		}
	}

	function edgeValue(r: Rect, edge: Edge): number {
		switch (edge) {
			case 'left':
				return r.x;
			case 'right':
				return r.x + r.width;
			case 'top':
				return r.y;
			case 'bottom':
				return r.y + r.height;
		}
	}

	const horizontal = (edge: Edge) => edge === 'left' || edge === 'right';

	/** 画面上の移動量（CSSの画素）を、元の大きさの画素にする */
	function toFrame(dx: number, dy: number): [number, number] {
		const b = box?.getBoundingClientRect();
		if (!b || b.width === 0 || b.height === 0) return [0, 0];
		return [(dx * size.width) / b.width, (dy * size.height) / b.height];
	}

	function drag(e: PointerEvent, apply: (dx: number, dy: number, start: Rect) => Rect) {
		e.preventDefault();
		e.stopPropagation();
		const start = { ...rect };
		const [x0, y0] = [e.clientX, e.clientY];
		const target = e.currentTarget as HTMLElement;
		target.setPointerCapture(e.pointerId);
		const move = (ev: PointerEvent) => {
			const [dx, dy] = toFrame(ev.clientX - x0, ev.clientY - y0);
			onchange(apply(dx, dy, start));
		};
		const up = () => {
			target.removeEventListener('pointermove', move);
			target.removeEventListener('pointerup', up);
			target.removeEventListener('pointercancel', up);
		};
		target.addEventListener('pointermove', move);
		target.addEventListener('pointerup', up);
		target.addEventListener('pointercancel', up);
	}

	function startMove(e: PointerEvent) {
		drag(e, (dx, dy, s) => ({
			...s,
			x: Math.round(clamp(s.x + dx, 0, size.width - s.width)),
			y: Math.round(clamp(s.y + dy, 0, size.height - s.height))
		}));
	}

	function startEdge(e: PointerEvent, edge: Edge) {
		drag(e, (dx, dy, s) => moveEdge(s, edge, edgeValue(s, edge) + (horizontal(edge) ? dx : dy)));
	}

	function key(e: KeyboardEvent, edge: Edge) {
		const dim = horizontal(edge) ? size.width : size.height;
		const step = Math.max(1, Math.round(dim * (e.shiftKey ? BIG_STEP_SHARE : STEP_SHARE)));
		const keys: Record<string, number> = horizontal(edge)
			? { ArrowLeft: -step, ArrowRight: step }
			: { ArrowUp: -step, ArrowDown: step };
		const d = keys[e.key];
		if (d === undefined) return;
		e.preventDefault();
		onchange(moveEdge(rect, edge, edgeValue(rect, edge) + d));
	}

	const EDGES: readonly Edge[] = ['top', 'right', 'bottom', 'left'];
	function edgeLabel(edge: Edge): string {
		switch (edge) {
			case 'top':
				return m['wizard.video.crop_edge_top']();
			case 'right':
				return m['wizard.video.crop_edge_right']();
			case 'bottom':
				return m['wizard.video.crop_edge_bottom']();
			case 'left':
				return m['wizard.video.crop_edge_left']();
		}
	}
</script>

<div class="editor" bind:this={box}>
	<div
		class="area"
		role="presentation"
		style:left={pct(rect.x, size.width)}
		style:top={pct(rect.y, size.height)}
		style:width={pct(rect.width, size.width)}
		style:height={pct(rect.height, size.height)}
		onpointerdown={startMove}
	>
		{#each EDGES as edge (edge)}
			<span
				class="handle {edge}"
				role="slider"
				tabindex="0"
				aria-label={edgeLabel(edge)}
				aria-orientation={horizontal(edge) ? 'horizontal' : 'vertical'}
				aria-valuemin={0}
				aria-valuemax={horizontal(edge) ? size.width : size.height}
				aria-valuenow={edgeValue(rect, edge)}
				onpointerdown={(e) => startEdge(e, edge)}
				onkeydown={(e) => key(e, edge)}
			></span>
		{/each}
	</div>
</div>

<style>
	.editor {
		position: absolute;
		inset: 0;
		touch-action: none;
	}
	.area {
		position: absolute;
		border: var(--pih-focus-width) dashed var(--cds-interactive);
		box-sizing: border-box;
		cursor: move;
	}
	.handle {
		position: absolute;
		width: var(--cds-spacing-04);
		height: var(--cds-spacing-04);
		background: var(--cds-layer-01);
		border: var(--pih-focus-width) solid var(--cds-interactive);
		box-sizing: border-box;
	}
	.handle.top,
	.handle.bottom {
		left: 50%;
		translate: -50% 0;
		cursor: ns-resize;
	}
	.handle.left,
	.handle.right {
		top: 50%;
		translate: 0 -50%;
		cursor: ew-resize;
	}
	.handle.top {
		top: calc(var(--cds-spacing-04) / -2);
	}
	.handle.bottom {
		bottom: calc(var(--cds-spacing-04) / -2);
	}
	.handle.left {
		left: calc(var(--cds-spacing-04) / -2);
	}
	.handle.right {
		right: calc(var(--cds-spacing-04) / -2);
	}
</style>
