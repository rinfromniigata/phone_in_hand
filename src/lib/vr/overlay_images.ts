/**
 * VR内の表示に使う画像を描いて、Rust側（overlay_presenter）へ渡す（imple 3章 S04、4章「オーバーレイの文言」）。
 *
 * - 状態のチップ（S04a・b・c・e。モックの `.vr-chip`）：文言は辞書から、色・寸法・書体はトークンから引く
 * - 位置合わせの見本（映像がないときにパネルに出す。モックの `.vr-panel` と `.fake-screen`）
 *
 * 値は tokens.css の変数を、いま適用しているテーマ（data-theme）で読む。言語やテーマを変えたら描き直す。
 */
import { m } from '#lib/paraglide/messages.js';
import type { Chip, OverlayImages, RgbaImage } from '#lib/state/types.ts';

/** VRで読めるよう、画面の寸法を拡大して描く倍率 */
const CHIP_SCALE = 4;
const SAMPLE_SCALE = 3;

const CHIPS: readonly Chip[] = ['tracking', 'sensor_lost', 'hand_lost', 'screen_off'];

/** チップの点の色（モックの `.dot.ok`・`.dot.warn`・`.dot.err`・`.dot`） */
const DOT_COLOR: Record<Chip, string> = {
	tracking: '--cds-support-success',
	sensor_lost: '--cds-support-warning',
	hand_lost: '--cds-support-error',
	screen_off: '--cds-border-strong-01'
};

function label(chip: Chip): string {
	switch (chip) {
		case 'tracking':
			return m['vr.chip.tracking']();
		case 'sensor_lost':
			return m['vr.chip.sensor_lost']();
		case 'hand_lost':
			return m['vr.chip.hand_lost']();
		case 'screen_off':
			return m['vr.chip.screen_off']();
	}
}

type Tokens = { str: (name: string) => string; px: (name: string) => number };

function readTokens(): Tokens {
	const style = getComputedStyle(document.documentElement);
	const str = (name: string) => style.getPropertyValue(name).trim();
	return { str, px: (name: string) => parseFloat(str(name)) || 0 };
}

function toBase64(bytes: Uint8ClampedArray): string {
	let s = '';
	const CHUNK = 0x8000;
	for (let i = 0; i < bytes.length; i += CHUNK) {
		s += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
	}
	return btoa(s);
}

function toImage(canvas: HTMLCanvasElement): RgbaImage | null {
	const ctx = canvas.getContext('2d');
	if (!ctx || canvas.width === 0 || canvas.height === 0) return null;
	const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
	return { width: canvas.width, height: canvas.height, rgba: toBase64(data) };
}

function pill(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
	const radius = Math.min(r, h / 2, w / 2);
	ctx.beginPath();
	ctx.roundRect(x, y, w, h, radius);
	ctx.fill();
}

async function drawChip(chip: Chip, t: Tokens): Promise<RgbaImage | null> {
	const s = CHIP_SCALE;
	const text = label(chip);
	const fontSize = t.px('--cds-label-01-font-size') * s;
	const lineHeight = t.px('--cds-label-01-line-height') * s;
	const font = `${t.str('--pih-font-weight-regular')} ${fontSize}px ${t.str('--pih-font-family-sans')}`;
	await document.fonts.load(font, text).catch(() => []);

	const padY = t.px('--cds-spacing-02') * s;
	const padX = t.px('--cds-spacing-04') * s;
	const gap = t.px('--cds-spacing-03') * s;
	const dot = t.px('--pih-status-dot') * s;

	const canvas = document.createElement('canvas');
	const measure = canvas.getContext('2d');
	if (!measure) return null;
	measure.font = font;
	const textWidth = Math.ceil(measure.measureText(text).width);
	canvas.width = Math.ceil(padX * 2 + dot + gap + textWidth);
	canvas.height = Math.ceil(padY * 2 + lineHeight);

	const ctx = canvas.getContext('2d');
	if (!ctx) return null;
	ctx.fillStyle = t.str('--cds-layer-01');
	pill(ctx, 0, 0, canvas.width, canvas.height, t.px('--pih-radius-full') * s);
	ctx.fillStyle = t.str(DOT_COLOR[chip]);
	ctx.beginPath();
	ctx.arc(padX + dot / 2, canvas.height / 2, dot / 2, 0, Math.PI * 2);
	ctx.fill();
	ctx.font = font;
	ctx.fillStyle = t.str('--cds-text-secondary');
	ctx.textBaseline = 'middle';
	ctx.fillText(text, padX + dot + gap, canvas.height / 2);
	return toImage(canvas);
}

/** 位置合わせの見本：スマホの画面の形の枠に、モックの `.fake-screen` の見本を描く */
function drawSample(t: Tokens): RgbaImage | null {
	const s = SAMPLE_SCALE;
	// モックの `.vr-panel`（幅 spacing-09 × 3.2、縦横比 9:19.5）
	const width = Math.round(t.px('--cds-spacing-09') * 3.2 * s);
	const height = Math.round((width * 19.5) / 9);
	const canvas = document.createElement('canvas');
	canvas.width = width;
	canvas.height = height;
	const ctx = canvas.getContext('2d');
	if (!ctx) return null;

	const border = t.px('--pih-border-width') * s * 2;
	ctx.fillStyle = t.str('--cds-border-strong-01');
	ctx.fillRect(0, 0, width, height);
	ctx.fillStyle = t.str('--cds-layer-01');
	ctx.fillRect(border, border, width - border * 2, height - border * 2);

	const pad = t.px('--cds-spacing-04') * s;
	const gap = t.px('--cds-spacing-03') * s;
	const inner = width - pad * 2;
	const full = t.px('--pih-radius-full') * s;
	const rows: { h: number; w: number; r: number }[] = [
		{ h: t.px('--cds-spacing-06') * s, w: 1, r: full },
		{ h: t.px('--cds-spacing-03') * s, w: 0.85, r: full },
		{ h: t.px('--cds-spacing-03') * s, w: 1, r: full },
		{ h: t.px('--cds-spacing-03') * s, w: 0.6, r: full },
		{ h: t.px('--cds-spacing-09') * s, w: 1, r: 0 },
		{ h: t.px('--cds-spacing-03') * s, w: 0.85, r: full },
		{ h: t.px('--cds-spacing-03') * s, w: 0.6, r: full },
		{ h: t.px('--cds-spacing-09') * s, w: 1, r: 0 }
	];
	ctx.fillStyle = t.str('--cds-layer-02');
	let y = pad;
	for (const row of rows) {
		if (y + row.h > height - pad) break;
		if (row.r > 0) pill(ctx, pad, y, inner * row.w, row.h, row.r);
		else ctx.fillRect(pad, y, inner * row.w, row.h);
		y += row.h + gap;
	}
	return toImage(canvas);
}

/** いまの言語とテーマで、VR内の表示に使う画像を描く */
export async function renderOverlayImages(): Promise<OverlayImages> {
	const t = readTokens();
	const chips: OverlayImages['chips'] = {};
	for (const chip of CHIPS) {
		const img = await drawChip(chip, t);
		if (img) chips[chip] = img;
	}
	return { chips, sample: drawSample(t) };
}
