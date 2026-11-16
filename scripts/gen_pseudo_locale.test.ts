import { describe, expect, test } from 'bun:test';
import { pseudo } from './gen_pseudo_locale.ts';

describe('pseudo', () => {
	test('記号で囲み、約4割伸ばす', () => {
		const out = pseudo('Settings');
		expect(out.startsWith('［')).toBe(true);
		expect(out.endsWith('］')).toBe(true);
		expect([...out].length).toBeGreaterThanOrEqual(Math.round('Settings'.length * 1.3) + 2);
	});
	test('プレースホルダは変えない', () => {
		expect(pseudo('No data for {sec} seconds')).toContain('{sec}');
		expect(pseudo('{device} · {w}×{h}')).toContain('{device} · {w}×{h}');
	});
});
