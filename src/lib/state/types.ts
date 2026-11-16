/**
 * Rust側（crates/pih_core）の serde の形に合わせた型。
 * 正本はRustの型で、形を変えたらここも合わせる（既定値は default_settings.json をRustのテストで検査する）。
 */

export type Side = 'left' | 'right';
export type Size = { width: number; height: number };
export type Rect = { x: number; y: number; width: number; height: number };

// ---- 設定（settings.json） ----

export type VideoSourceKind = 'hdmi' | 'mirror' | 'window';
export type PoseSourceKind = 'controller' | 'hand';
export type LanguageSettingValue = 'auto' | 'ja' | 'en' | 'zh-Hans';
export type ThemeSettingValue = 'system' | 'light' | 'dark';

export type Settings = {
	schema_version: number;
	setup_completed: boolean;
	phone_kind: 'android' | 'iphone';
	video: {
		source: VideoSourceKind;
		hdmi: { device_id: string | null; width: number | null; height: number | null; fps: number | null };
		mirror: { connection: 'usb' | 'wifi'; codec: 'auto' | 'h264' | 'h265'; serial: string | null };
		window: { title: string | null; executable: string | null };
		crop: { mode: 'auto' | 'manual'; rect: Rect | null };
	};
	pose: { source: PoseSourceKind; side: Side };
	display: { enabled: boolean; size_inch: number; brightness: number };
	input: { show_touches: boolean; keep_awake: boolean };
	vrchat: { send_port: number; receive_port: number; param_visible: string; param_is_left: string };
	general: {
		language: LanguageSettingValue;
		theme: ThemeSettingValue;
		start_with_steamvr: boolean;
		start_minimized: boolean;
	};
	sensor: { token: string | null; port: number };
	adb_devices: { serial: string; name: string; last_address: string | null }[];
};

/** 設定の部分的な変更（JSON Merge Patch） */
export type DeepPartial<T> = T extends readonly unknown[]
	? T
	: T extends object
		? { [K in keyof T]?: DeepPartial<T[K]> | null }
		: T;
export type SettingsPatch = DeepPartial<Settings>;

// ---- 状態（AppSnapshot） ----

export type PanelMode = 'controller' | 'hand' | 'hand_fallback';

export type VideoStatus =
	| { state: 'unavailable' }
	| { state: 'idle' }
	| { state: 'no_device' }
	| { state: 'no_signal' }
	| { state: 'running'; device: string; size: Size; fps: number }
	| { state: 'error'; error: 'source_lost' | 'open_failed' };

export type SensorStatus =
	| { state: 'unavailable' }
	| { state: 'not_used' }
	| { state: 'receiving' }
	| { state: 'lost'; seconds: number };

export type AdbStatus =
	| { state: 'unavailable' }
	| { state: 'not_used' }
	| { state: 'disconnected' }
	| { state: 'connected'; device: string; show_touches: boolean };

export type TrackingStatus =
	| { state: 'unavailable' }
	| { state: 'tracking'; mode: PanelMode; side: Side }
	| { state: 'lost'; side: Side };

export type VrchatLink =
	| { state: 'unavailable' }
	| { state: 'not_running' }
	| { state: 'connected'; params_found: boolean };

export type CalibrationStatus = 'missing' | 'valid' | 'stale';

export type AppSnapshot = {
	display_enabled: boolean;
	video: VideoStatus;
	sensor: SensorStatus;
	adb: AdbStatus;
	tracking: TrackingStatus;
	vrchat: VrchatLink;
	calibration: CalibrationStatus;
};

// ---- コマンドの失敗 ----

export type ErrorCode = 'unavailable' | 'not_found' | 'invalid' | 'corrupt' | 'io';
export type CommandError = { code: ErrorCode };

export function isCommandError(e: unknown): e is CommandError {
	return typeof e === 'object' && e !== null && typeof (e as { code?: unknown }).code === 'string';
}
