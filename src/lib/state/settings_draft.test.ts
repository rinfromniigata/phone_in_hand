import { describe, expect, test } from 'bun:test';
import defaults from './default_settings.json';
import { isDirty, parsePort, toDraft, toPatch, validate } from './settings_draft.ts';
import type { Settings } from './types.ts';

const saved = defaults as Settings;

describe('settings_draft', () => {
	test('保存済みの設定から作った下書きは、変更なし', () => {
		const d = toDraft(saved);
		expect(isDirty(d, saved)).toBe(false);
		expect(toPatch(d, saved)).toBeNull();
		expect(validate(d)).toEqual({});
	});

	test('明るさだけを変えると、明るさだけの変更になる', () => {
		const d = { ...toDraft(saved), brightness_pct: 50 };
		expect(isDirty(d, saved)).toBe(true);
		expect(toPatch(d, saved)).toEqual({ display: { brightness: 0.5 } });
	});

	test('複数の節の変更をまとめる。切り抜きの矩形は送らない', () => {
		const d = { ...toDraft(saved), video_source: 'window' as const, crop_mode: 'manual' as const, pose_side: 'left' as const, send_port: ' 9100 ' };
		expect(toPatch(d, saved)).toEqual({
			video: { source: 'window', crop: { mode: 'manual' } },
			pose: { side: 'left' },
			vrchat: { send_port: 9100 }
		});
	});

	test('ポートの検証', () => {
		expect(parsePort('9000')).toBe(9000);
		expect(parsePort('65535')).toBe(65535);
		for (const bad of ['', '0', '65536', '-1', '90.5', 'abc', '１２３']) expect(parsePort(bad)).toBeNull();
		const d = { ...toDraft(saved), send_port: '0', receive_port: 'x' };
		expect(validate(d)).toEqual({ send_port: true, receive_port: true });
	});

	test('パラメータ名が空白だけなら誤り', () => {
		const d = { ...toDraft(saved), param_visible: '   ' };
		expect(validate(d)).toEqual({ param_visible: true });
	});

	test('元に戻せば変更なし', () => {
		const d = { ...toDraft(saved), keep_awake: !saved.input.keep_awake };
		expect(isDirty(d, saved)).toBe(true);
		expect(isDirty({ ...d, keep_awake: saved.input.keep_awake }, saved)).toBe(false);
	});
});
