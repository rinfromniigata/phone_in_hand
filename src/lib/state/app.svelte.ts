/**
 * スナップショットと設定を受け持つストア。
 *
 * - Tauriの中では、起動時に `get_settings`・`get_snapshot` を読み、`state_changed` で更新する
 * - ブラウザでの開発（Tauriの外）では、既定値の設定とメモリ上の変更だけで動かす（機器は使用できない）
 * - 開発時だけ、`AppSnapshot` の見本（状態の見本）で実際の状態を置き換えられる（src/lib/dev/）
 */
import { api, inTauri } from './api.ts';
import defaultSettings from './default_settings.json';
import type { AppSnapshot, PreviewFrame, Settings, SettingsPatch } from './types.ts';

/** 機器の状態がまだ届いていない、またはTauriの外のときの状態 */
const UNAVAILABLE: AppSnapshot = {
	display_enabled: false,
	video: { state: 'unavailable' },
	sensor: { state: 'unavailable' },
	adb: { state: 'unavailable' },
	tracking: { state: 'unavailable' },
	vrchat: { state: 'unavailable' },
	calibration: 'missing'
};

function clone<T>(v: T): T {
	return structuredClone(v);
}

/** RFC 7386（JSON Merge Patch）。Tauriの外で設定の変更を当てるときに使う */
function mergePatch(target: unknown, patch: unknown): unknown {
	if (typeof patch !== 'object' || patch === null || Array.isArray(patch)) return patch;
	const base: Record<string, unknown> =
		typeof target === 'object' && target !== null && !Array.isArray(target)
			? { ...(target as Record<string, unknown>) }
			: {};
	for (const [k, v] of Object.entries(patch)) {
		if (v === null) delete base[k];
		else base[k] = mergePatch(base[k], v);
	}
	return base;
}

class AppStore {
	/** Rust側から届いた実際の状態 */
	#live = $state<AppSnapshot>(clone(UNAVAILABLE));
	/** 開発時だけの状態の見本。null なら実際の状態を表示する */
	sample = $state<AppSnapshot | null>(null);
	/** 開発時だけ、見本を描くのに使うスマホの種類 */
	samplePhoneKind = $state<Settings['phone_kind'] | null>(null);
	/** 開発時だけ、設定によらず使う表示の言語（疑似ロケールを含む） */
	localeOverride = $state<string | null>(null);
	settings = $state<Settings>(clone(defaultSettings as Settings));
	ready = $state(false);
	/** 映像のプレビューの最新の1枚（取り込みが動いていなければ null） */
	#preview = $state.raw<PreviewFrame | null>(null);

	/** 画面に出す状態（見本があれば見本） */
	get snapshot(): AppSnapshot {
		return this.sample ?? this.#live;
	}

	/** 画面に出すプレビュー。見本の表示中と、映像が止まっているときは出さない */
	get preview(): PreviewFrame | null {
		if (this.sample) return null;
		return this.#live.video.state === 'running' ? this.#preview : null;
	}

	/** 画面に出すスマホの種類（見本があれば見本の値） */
	get phoneKind(): Settings['phone_kind'] {
		return this.samplePhoneKind ?? this.settings.phone_kind;
	}

	/** 適用する表示の言語の設定 */
	get language(): string {
		return this.localeOverride ?? this.settings.general.language;
	}

	async init(): Promise<void> {
		if (this.ready) return;
		if (inTauri()) {
			await api.onStateChanged((s) => {
				this.#live = s;
				if (s.video.state !== 'running') this.#preview = null;
			});
			await api.onPreviewFrame((f) => (this.#preview = f));
			const [settings, snapshot] = await Promise.all([api.getSettings(), api.getSnapshot()]);
			this.settings = settings;
			this.#live = snapshot;
		}
		this.ready = true;
	}

	/** 部分的な変更を当てて保存する。失敗したら CommandError を投げ、設定は変えない */
	async updateSettings(patch: SettingsPatch): Promise<Settings> {
		if (inTauri()) {
			this.settings = await api.updateSettings(patch);
		} else {
			// $state のプロキシは structuredClone できないため、素のオブジェクトにしてから当てる
			this.settings = mergePatch($state.snapshot(this.settings), patch) as Settings;
		}
		return this.settings;
	}

	async setDisplayEnabled(enabled: boolean): Promise<void> {
		if (inTauri()) {
			this.#live = await api.setDisplayEnabled(enabled);
			this.settings.display.enabled = enabled;
		} else {
			await this.updateSettings({ display: { enabled } });
			this.#live = { ...this.#live, display_enabled: enabled };
		}
	}
}

export const app = new AppStore();
