/**
 * Tauriのコマンドとイベントの呼び出し（imple 2章「Tauriのコマンドとイベント」）。
 * 型はRust側のコマンドに合わせる。
 */
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AppSnapshot, Settings, SettingsPatch, VideoSourceKind } from './types.ts';

export const EVENT_STATE_CHANGED = 'state_changed';

export const api = {
	getSnapshot: () => invoke<AppSnapshot>('get_snapshot'),
	getSettings: () => invoke<Settings>('get_settings'),
	updateSettings: (patch: SettingsPatch) => invoke<Settings>('update_settings', { patch }),
	setDisplayEnabled: (enabled: boolean) => invoke<AppSnapshot>('set_display_enabled', { enabled }),
	startVideo: (source: VideoSourceKind) => invoke<void>('start_video', { source }),
	completeSetup: (patch: SettingsPatch) => invoke<Settings>('complete_setup', { patch }),
	onStateChanged: (handler: (snapshot: AppSnapshot) => void): Promise<UnlistenFn> =>
		listen<AppSnapshot>(EVENT_STATE_CHANGED, (e) => handler(e.payload))
};

/** Tauriの中で動いているか（ブラウザでの開発では false） */
export const inTauri = (): boolean => isTauri();
