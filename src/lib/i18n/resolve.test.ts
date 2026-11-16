import { describe, expect, test } from 'bun:test';
import { matchLocale, pickLocale } from './resolve.ts';

describe('matchLocale', () => {
	test('日本語・英語は地域によらず寄せる', () => {
		expect(matchLocale('ja-JP')).toBe('ja');
		expect(matchLocale('en-US')).toBe('en');
		expect(matchLocale('en_GB')).toBe('en');
	});
	test('中国語は簡体字だけを zh-Hans にする', () => {
		expect(matchLocale('zh')).toBe('zh-Hans');
		expect(matchLocale('zh-CN')).toBe('zh-Hans');
		expect(matchLocale('zh-Hans-CN')).toBe('zh-Hans');
		expect(matchLocale('zh-TW')).toBeUndefined();
		expect(matchLocale('zh-Hant')).toBeUndefined();
	});
	test('対応しない言語は undefined', () => {
		expect(matchLocale('fr-FR')).toBeUndefined();
		expect(matchLocale('')).toBeUndefined();
	});
});

describe('pickLocale', () => {
	test('最初に対応する候補を選ぶ', () => {
		expect(pickLocale(['fr-FR', 'en-US', 'ja-JP'])).toBe('en');
	});
	test('どれも対応しなければ ja', () => {
		expect(pickLocale(['fr-FR', 'zh-TW', null])).toBe('ja');
		expect(pickLocale([])).toBe('ja');
	});
});
