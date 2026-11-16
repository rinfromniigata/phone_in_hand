/**
 * 設定画面（S03a・S03b）の下書き。保存を押すまでは、保存済みの設定に触れない。
 *
 * - 入力欄の形（ポートは文字列、明るさは百分率）で持ち、保存のときに設定の形の変更（JSON Merge Patch）にする
 * - 変わった項目だけを送る（imple 7章「設定で明るさを変えて保存する」：明るさだけが変わる）
 * - 検証（ポートの範囲、パラメータ名）はRust側の `Settings::validate` と同じ条件にそろえる
 */
import type { Settings, SettingsPatch } from './types.ts';

export type Draft = {
	size_inch: number;
	/** 0〜100の整数（保存するときに0〜1にする） */
	brightness_pct: number;
	video_source: Settings['video']['source'];
	crop_mode: Settings['video']['crop']['mode'];
	pose_source: Settings['pose']['source'];
	pose_side: Settings['pose']['side'];
	show_touches: boolean;
	keep_awake: boolean;
	send_port: string;
	receive_port: string;
	param_visible: string;
	param_is_left: string;
};

export type DraftErrors = Partial<Record<'send_port' | 'receive_port' | 'param_visible' | 'param_is_left', true>>;

/** パネルの大きさのスライダーの範囲（インチ。委任事項） */
export const SIZE_INCH_MIN = 4;
export const SIZE_INCH_MAX = 8;
export const SIZE_INCH_STEP = 0.1;

const roundTenth = (v: number) => Math.round(v * 10) / 10;

export function toDraft(s: Settings): Draft {
	return {
		size_inch: roundTenth(s.display.size_inch),
		brightness_pct: Math.round(s.display.brightness * 100),
		video_source: s.video.source,
		crop_mode: s.video.crop.mode,
		pose_source: s.pose.source,
		pose_side: s.pose.side,
		show_touches: s.input.show_touches,
		keep_awake: s.input.keep_awake,
		send_port: String(s.vrchat.send_port),
		receive_port: String(s.vrchat.receive_port),
		param_visible: s.vrchat.param_visible,
		param_is_left: s.vrchat.param_is_left
	};
}

export function parsePort(text: string): number | null {
	const t = text.trim();
	if (!/^\d{1,5}$/.test(t)) return null;
	const n = Number(t);
	return n >= 1 && n <= 65535 ? n : null;
}

export function validate(d: Draft): DraftErrors {
	const errors: DraftErrors = {};
	if (parsePort(d.send_port) === null) errors.send_port = true;
	if (parsePort(d.receive_port) === null) errors.receive_port = true;
	if (d.param_visible.trim() === '') errors.param_visible = true;
	if (d.param_is_left.trim() === '') errors.param_is_left = true;
	return errors;
}

/** 保存済みの設定から変わった項目だけの変更。何も変わらなければ null。下書きに誤りがあれば呼ばない */
export function toPatch(d: Draft, saved: Settings): SettingsPatch | null {
	const base = toDraft(saved);
	const patch: Record<string, Record<string, unknown>> = {};
	const section = (k: 'display' | 'video' | 'pose' | 'input' | 'vrchat') => (patch[k] ??= {});

	if (d.size_inch !== base.size_inch) section('display').size_inch = d.size_inch;
	if (d.brightness_pct !== base.brightness_pct) section('display').brightness = d.brightness_pct / 100;
	if (d.video_source !== base.video_source) section('video').source = d.video_source;
	if (d.crop_mode !== base.crop_mode) section('video').crop = { mode: d.crop_mode };
	if (d.pose_source !== base.pose_source) section('pose').source = d.pose_source;
	if (d.pose_side !== base.pose_side) section('pose').side = d.pose_side;
	if (d.show_touches !== base.show_touches) section('input').show_touches = d.show_touches;
	if (d.keep_awake !== base.keep_awake) section('input').keep_awake = d.keep_awake;
	if (d.send_port.trim() !== base.send_port) section('vrchat').send_port = parsePort(d.send_port) ?? undefined;
	if (d.receive_port.trim() !== base.receive_port) section('vrchat').receive_port = parsePort(d.receive_port) ?? undefined;
	if (d.param_visible !== base.param_visible) section('vrchat').param_visible = d.param_visible.trim();
	if (d.param_is_left !== base.param_is_left) section('vrchat').param_is_left = d.param_is_left.trim();

	return Object.keys(patch).length > 0 ? (patch as SettingsPatch) : null;
}

/** 保存済みの設定と違う項目があるか（検証の結果によらず、入力欄の値の違い） */
export function isDirty(d: Draft, saved: Settings): boolean {
	return JSON.stringify(d) !== JSON.stringify(toDraft(saved));
}
