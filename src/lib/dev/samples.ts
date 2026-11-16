/**
 * 開発時だけ使う、AppSnapshot の見本（モックのS02a・S02b・S02c）。機器なしで状態の表示を確かめる。
 * 公開版には含めない（DevPanel から動的に読み込む）。
 */
import type { AppSnapshot, Settings } from '#lib/state/types.ts';

export type Sample = {
	id: 'S02a' | 'S02b' | 'S02c';
	snapshot: AppSnapshot;
	/** 見本を描くのに要る設定（保存はしない） */
	phone_kind: Settings['phone_kind'];
};

export const samples: Sample[] = [
	{
		id: 'S02a',
		phone_kind: 'android',
		snapshot: {
			display_enabled: true,
			video: { state: 'running', device: 'MS2130', size: { width: 1920, height: 1080 }, fps: 60 },
			sensor: { state: 'not_used' },
			adb: { state: 'connected', device: 'Pixel 8', show_touches: true },
			tracking: { state: 'tracking', mode: 'controller', side: 'right' },
			vrchat: { state: 'connected', params_found: true },
			calibration: 'valid'
		}
	},
	{
		id: 'S02b',
		phone_kind: 'android',
		snapshot: {
			display_enabled: true,
			video: {
				state: 'running',
				device: 'USB3.0 HD Video Capture (MS2130) Rev.B — Generic UVC Composite Device',
				size: { width: 1920, height: 1080 },
				fps: 60
			},
			sensor: { state: 'lost', seconds: 12 },
			adb: { state: 'connected', device: 'Pixel 8', show_touches: true },
			tracking: { state: 'tracking', mode: 'hand_fallback', side: 'left' },
			vrchat: { state: 'not_running' },
			calibration: 'valid'
		}
	},
	{
		id: 'S02c',
		phone_kind: 'iphone',
		snapshot: {
			display_enabled: true,
			video: { state: 'running', device: 'MS2130', size: { width: 1920, height: 1080 }, fps: 60 },
			sensor: { state: 'not_used' },
			adb: { state: 'not_used' },
			tracking: { state: 'tracking', mode: 'controller', side: 'right' },
			vrchat: { state: 'connected', params_found: true },
			calibration: 'valid'
		}
	}
];
