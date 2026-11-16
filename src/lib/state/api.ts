/**
 * Tauriのコマンドとイベントの呼び出し（imple 2章「Tauriのコマンドとイベント」）。
 * 型はRust側のコマンドに合わせる。
 */
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
	AppSnapshot,
	CalibrationProgress,
	OverlayImages,
	PreviewFrame,
	Settings,
	SettingsPatch,
	WindowInfo
} from './types.ts';

export const EVENT_STATE_CHANGED = 'state_changed';
export const EVENT_PREVIEW_FRAME = 'preview_frame';
export const EVENT_CALIBRATION_PROGRESS = 'calibration_progress';

export const api = {
	getSnapshot: () => invoke<AppSnapshot>('get_snapshot'),
	getSettings: () => invoke<Settings>('get_settings'),
	updateSettings: (patch: SettingsPatch) => invoke<Settings>('update_settings', { patch }),
	setDisplayEnabled: (enabled: boolean) => invoke<AppSnapshot>('set_display_enabled', { enabled }),
	listWindows: () => invoke<WindowInfo[]>('list_windows'),
	/** 保存していない映像の設定で取り込む（映像のページのプレビュー） */
	startVideo: (video: Settings['video']) => invoke<void>('start_video', { video }),
	setOverlayImages: (images: OverlayImages) => invoke<void>('set_overlay_images', { images }),
	completeSetup: (patch: SettingsPatch) => invoke<Settings>('complete_setup', { patch }),
	/** 位置合わせを始める。基準と側は、終えたときに設定へ保存される */
	startCalibration: (pose: Settings['pose']) => invoke<void>('start_calibration', { pose }),
	cancelCalibration: () => invoke<void>('cancel_calibration'),
	onCalibrationProgress: (handler: (progress: CalibrationProgress) => void): Promise<UnlistenFn> =>
		listen<CalibrationProgress>(EVENT_CALIBRATION_PROGRESS, (e) => handler(e.payload)),
	onStateChanged: (handler: (snapshot: AppSnapshot) => void): Promise<UnlistenFn> =>
		listen<AppSnapshot>(EVENT_STATE_CHANGED, (e) => handler(e.payload)),
	onPreviewFrame: (handler: (frame: PreviewFrame) => void): Promise<UnlistenFn> =>
		listen<PreviewFrame>(EVENT_PREVIEW_FRAME, (e) => handler(e.payload))
};

/** Tauriの中で動いているか（ブラウザでの開発では false） */
export const inTauri = (): boolean => isTauri();
