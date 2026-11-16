import { describe, expect, test } from 'bun:test';
import { samples } from '#lib/dev/samples.ts';
import { banners, statusRows } from './status_rows.ts';
import type { AppSnapshot } from './types.ts';

const sample = (id: string) => samples.find((s) => s.id === id)!;
const summary = (id: string) => {
	const s = sample(id);
	return statusRows(s.snapshot, s.phone_kind).map((r) => `${r.id}:${r.tone}`);
};

describe('status_rows（モックS02a〜cの状態の見本と一致）', () => {
	test('S02a 通常：センサーは使用しない、ほかは良好', () => {
		expect(summary('S02a')).toEqual(['video:ok', 'sensor:off', 'adb:ok', 'hand:ok', 'vrchat:ok']);
		expect(banners(sample('S02a').snapshot, 'android')).toEqual([]);
	});

	test('S02b ハンドトラッキング・一部切断：センサーは警告、VRChatは異常、帯が出る', () => {
		expect(summary('S02b')).toEqual(['video:ok', 'sensor:warn', 'adb:ok', 'hand:ok', 'vrchat:err']);
		expect(banners(sample('S02b').snapshot, 'android').map((b) => b.id)).toEqual(['sensor_lost']);
	});

	test('S02c iPhone：センサーとワイヤレスデバッグは使用できない、案内の帯が出る', () => {
		expect(summary('S02c')).toEqual(['video:ok', 'sensor:off', 'adb:off', 'hand:ok', 'vrchat:ok']);
		expect(banners(sample('S02c').snapshot, 'iphone').map((b) => b.id)).toEqual(['iphone']);
	});
});

describe('status_rows（状態ごとの文字と色）', () => {
	test('どの状態でも、色だけでなく状態の文字が入る', () => {
		const states: AppSnapshot[] = [
			{
				display_enabled: false,
				video: { state: 'no_signal' },
				sensor: { state: 'receiving' },
				adb: { state: 'disconnected' },
				tracking: { state: 'lost', side: 'left' },
				vrchat: { state: 'connected', params_found: false },
				calibration: 'missing'
			},
			{
				display_enabled: false,
				video: { state: 'error', error: 'source_lost' },
				sensor: { state: 'unavailable' },
				adb: { state: 'not_used' },
				tracking: { state: 'unavailable' },
				vrchat: { state: 'unavailable' },
				calibration: 'stale'
			}
		];
		for (const s of states) {
			for (const row of statusRows(s, 'android')) {
				expect(row.state.length).toBeGreaterThan(0);
				expect(row.name.length).toBeGreaterThan(0);
			}
		}
		const first = statusRows(states[0], 'android');
		expect(first.map((r) => r.tone)).toEqual(['warn', 'ok', 'warn', 'warn', 'warn']);
		const second = statusRows(states[1], 'android');
		expect(second.map((r) => r.tone)).toEqual(['err', 'off', 'off', 'off', 'off']);
	});

	test('位置の追跡の補足は、基準と側に対応する', () => {
		const base = sample('S02a').snapshot;
		const detail = (mode: 'controller' | 'hand', side: 'left' | 'right') =>
			statusRows({ ...base, tracking: { state: 'tracking', mode, side } }, 'android')[3].detail;
		const all = [detail('controller', 'left'), detail('controller', 'right'), detail('hand', 'left'), detail('hand', 'right')];
		expect(new Set(all).size).toBe(4);
	});
});
