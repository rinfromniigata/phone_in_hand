/**
 * アプリ内のライセンス表示の一覧（スペック7章・9a節）。設定の「ライセンス」のダイアログが表示する。
 *
 * - 配布物に含まれるソフトウェアだけを載せる（開発用のツールは載せない）
 * - 名称・版・ライセンス・著作権表示は、原文（英語）のまま。翻訳しない
 * - 依存の版を上げたら、ここの版も合わせる。ライセンスの全文はNOTICEに置く（T10-3）
 * - scrcpy-serverは、Phase 6（Androidのミラーリング）で同梱する。同梱するまでは、配布物に含まれない
 */
export type LicenseEntry = {
	name: string;
	/** 版。同梱する版が決まっていないものは省く */
	version?: string;
	/** SPDXのライセンス識別子 */
	license: string;
	/** 著作権表示（原文のまま） */
	copyright: string[];
	url: string;
};

export const licenses: readonly LicenseEntry[] = [
	{
		name: 'scrcpy-server',
		license: 'Apache-2.0',
		copyright: ['Copyright (C) 2018 Genymobile', 'Copyright (C) 2018-2026 Romain Vimont'],
		url: 'https://github.com/Genymobile/scrcpy'
	},
	{
		name: 'IBM Plex Sans',
		version: '1.1.0',
		license: 'OFL-1.1',
		copyright: ['Copyright © 2017 IBM Corp. with Reserved Font Name "Plex"'],
		url: 'https://github.com/IBM/plex'
	},
	{
		name: 'IBM Plex Sans JP',
		version: '3.0.0',
		license: 'OFL-1.1',
		copyright: ['Copyright © 2017 IBM Corp. with Reserved Font Name "Plex"'],
		url: 'https://github.com/IBM/plex'
	},
	{
		name: 'IBM Plex Sans SC',
		version: '1.1.0',
		license: 'OFL-1.1',
		copyright: ['Copyright © 2017 IBM Corp. with Reserved Font Name "Plex"'],
		url: 'https://github.com/IBM/plex'
	},
	{
		name: 'IBM Plex Mono',
		version: '2.5.0',
		license: 'OFL-1.1',
		copyright: ['Copyright © 2017 IBM Corp. with Reserved Font Name "Plex"'],
		url: 'https://github.com/IBM/plex'
	},
	{
		name: 'Carbon Components Svelte',
		version: '0.114.1',
		license: 'Apache-2.0',
		copyright: ['Copyright 2019 IBM'],
		url: 'https://github.com/carbon-design-system/carbon-components-svelte'
	},
	{
		name: 'Carbon Icons Svelte',
		version: '13.18.0',
		license: 'Apache-2.0',
		copyright: ['Copyright 2019 IBM'],
		url: 'https://github.com/carbon-design-system/carbon-icons-svelte'
	},
	{
		name: 'Tauri',
		version: '2.12',
		license: 'Apache-2.0 OR MIT',
		copyright: ['Copyright (c) 2017 - Present Tauri Apps Contributors'],
		url: 'https://github.com/tauri-apps/tauri'
	},
	{
		name: 'Svelte',
		version: '5.57.3',
		license: 'MIT',
		copyright: ['Copyright (c) 2016-2025 Svelte Contributors'],
		url: 'https://github.com/sveltejs/svelte'
	},
	{
		name: 'Paraglide JS',
		version: '2.29.3',
		license: 'MIT',
		copyright: ['Copyright (c) 2025 Opral US Inc.'],
		url: 'https://github.com/opral/paraglide-js'
	}
];
