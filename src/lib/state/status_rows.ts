/**
 * ホーム（S02）の「接続の状態」の5つの行と、上部の帯（バナー）。
 *
 * `AppSnapshot` の状態の列挙を、そのまま辞書のキーに対応させる（imple 3章。UIで文を組み立てない）。
 * 状態は、色（tone）と文字（state）の両方で示す。色だけに頼らない。
 * 文言は呼ばれたときに辞書から引く（ロケールの切り替えに追従する）。
 */
import { m } from '#lib/paraglide/messages.js';
import type { AppSnapshot, Settings } from './types.ts';

/** 点の色。off は未使用・使用できない（灰色） */
export type Tone = 'ok' | 'warn' | 'err' | 'off';

export type StatusRowId = 'video' | 'sensor' | 'adb' | 'hand' | 'vrchat';

export type StatusRow = {
	id: StatusRowId;
	tone: Tone;
	name: string;
	/** 補足の行。なければ null */
	detail: string | null;
	/** 状態を表す文字 */
	state: string;
};

export type Banner = { id: 'sensor_lost' | 'iphone'; tone: 'warn' | 'info'; text: string };

type Part = Pick<StatusRow, 'tone' | 'detail' | 'state'>;

function video(s: AppSnapshot['video']): Part {
	switch (s.state) {
		case 'unavailable':
			return { tone: 'off', state: m['state.unavailable'](), detail: null };
		case 'idle':
			return { tone: 'off', state: m['state.waiting'](), detail: null };
		case 'no_device':
			return { tone: 'err', state: m['state.disconnected'](), detail: m['wizard.video.no_device']() };
		case 'no_signal':
			return { tone: 'warn', state: m['state.disconnected'](), detail: m['wizard.video.no_signal']() };
		case 'running':
			return {
				tone: 'ok',
				state: m['state.connected'](),
				detail: m['home.detail.video']({
					device: s.device,
					w: String(s.size.width),
					h: String(s.size.height),
					fps: String(s.fps)
				})
			};
		case 'error':
			return { tone: 'err', state: m['state.disconnected'](), detail: m['home.detail.video_error']() };
	}
}

function sensor(s: AppSnapshot['sensor']): Part {
	switch (s.state) {
		case 'unavailable':
			return { tone: 'off', state: m['state.unavailable'](), detail: null };
		case 'not_used':
			return { tone: 'off', state: m['state.not_used'](), detail: m['home.detail.sensor_not_needed']() };
		case 'receiving':
			return { tone: 'ok', state: m['state.connected'](), detail: null };
		case 'lost':
			return { tone: 'warn', state: m['state.fallback'](), detail: m['home.detail.sensor_lost']({ sec: String(s.seconds) }) };
	}
}

function adb(s: AppSnapshot['adb']): Part {
	switch (s.state) {
		case 'unavailable':
			return { tone: 'off', state: m['state.unavailable'](), detail: null };
		case 'not_used':
			return { tone: 'off', state: m['state.not_used'](), detail: m['home.detail.adb_not_used']() };
		case 'disconnected':
			return { tone: 'warn', state: m['state.disconnected'](), detail: null };
		case 'connected':
			return {
				tone: 'ok',
				state: m['state.connected'](),
				detail: s.show_touches ? m['home.detail.adb']() : m['home.detail.adb_touches_off']()
			};
	}
}

function hand(s: AppSnapshot['tracking']): Part {
	switch (s.state) {
		case 'unavailable':
			return { tone: 'off', state: m['state.unavailable'](), detail: null };
		case 'lost':
			return { tone: 'warn', state: m['state.lost'](), detail: null };
		case 'tracking': {
			const controller = s.mode === 'controller';
			const detail =
				s.side === 'left'
					? controller
						? m['pose.detail.controller_left']()
						: m['pose.detail.hand_left']()
					: controller
						? m['pose.detail.controller_right']()
						: m['pose.detail.hand_right']();
			return { tone: 'ok', state: m['state.tracking'](), detail };
		}
	}
}

function vrchat(s: AppSnapshot['vrchat']): Part {
	switch (s.state) {
		case 'unavailable':
			return { tone: 'off', state: m['state.unavailable'](), detail: null };
		case 'not_running':
			return { tone: 'err', state: m['state.disconnected'](), detail: m['home.detail.vrchat_off']() };
		case 'connected':
			return s.params_found
				? { tone: 'ok', state: m['state.connected'](), detail: m['home.detail.vrchat']() }
				: { tone: 'warn', state: m['state.connected'](), detail: m['home.detail.vrchat_no_params']() };
	}
}

/** iPhoneでは、センサーとワイヤレスデバッグは使えない（S02c） */
function notForIphone(): Part {
	return { tone: 'off', state: m['state.unavailable'](), detail: m['home.detail.iphone']() };
}

export function statusRows(snapshot: AppSnapshot, phoneKind: Settings['phone_kind']): StatusRow[] {
	const iphone = phoneKind === 'iphone';
	return [
		{ id: 'video', name: m['status.video'](), ...video(snapshot.video) },
		{ id: 'sensor', name: m['status.sensor'](), ...(iphone ? notForIphone() : sensor(snapshot.sensor)) },
		{ id: 'adb', name: m['status.adb'](), ...(iphone ? notForIphone() : adb(snapshot.adb)) },
		{ id: 'hand', name: m['status.hand'](), ...hand(snapshot.tracking) },
		{ id: 'vrchat', name: m['status.vrchat'](), ...vrchat(snapshot.vrchat) }
	];
}

export function banners(snapshot: AppSnapshot, phoneKind: Settings['phone_kind']): Banner[] {
	const list: Banner[] = [];
	if (phoneKind !== 'iphone' && snapshot.sensor.state === 'lost') {
		list.push({ id: 'sensor_lost', tone: 'warn', text: m['home.warn.sensor_lost']() });
	}
	if (phoneKind === 'iphone') {
		list.push({ id: 'iphone', tone: 'info', text: m['home.info.iphone']() });
	}
	return list;
}
