<script lang="ts">
	/**
	 * 設定（S03・S03a〜c）のタブと入力欄。
	 *
	 * - 表示・位置・入力・VRChat：下書きに入力し、「保存」で `update_settings` を呼ぶ（変えた項目だけ）。
	 *   保存せずに離れたら何も変わらない（imple 7章）
	 * - 一般（言語・テーマ・起動の設定）：保存ボタンはなく、変えた時点で保存し、すぐに適用する
	 * - スペック9a節：S03bのアバター用パッケージの要素は表示しない。S03cの末尾にライセンスのリンクを置く
	 * - 取り込み方法を変えたときの映像のページ（T3-7・T4-2・T7-3）と、ロック解除の案内（T6-3）はあとでつなぐ
	 */
	import { Button, Select, SelectItem, TextInput } from 'carbon-components-svelte';
	import Banner from './Banner.svelte';
	import LicenseDialog from './LicenseDialog.svelte';
	import SwitchField from './SwitchField.svelte';
	import Tabs from './Tabs.svelte';
	import { app } from '#lib/state/app.svelte.ts';
	import {
		SIZE_INCH_MAX,
		SIZE_INCH_MIN,
		SIZE_INCH_STEP,
		isDirty,
		toDraft,
		toPatch,
		validate,
		type Draft
	} from '#lib/state/settings_draft.ts';
	import type { SettingsPatch } from '#lib/state/types.ts';
	import { PUBLIC_LOCALES } from '#lib/i18n/resolve.ts';
	import { languageName } from '#lib/i18n/language_names.ts';
	import { m } from '#lib/paraglide/messages.js';

	type TabId = 'display' | 'position' | 'input' | 'vrchat' | 'general';
	const tabs = $derived([
		{ id: 'display', label: m['settings.tab.display']() },
		{ id: 'position', label: m['settings.tab.position']() },
		{ id: 'input', label: m['settings.tab.input']() },
		{ id: 'vrchat', label: m['settings.tab.vrchat']() },
		{ id: 'general', label: m['settings.tab.general']() }
	] as const);

	let tab = $state<TabId>('display');

	// ---- 下書き（表示・位置・入力・VRChat） ----
	let draft = $state<Draft>(toDraft(app.settings));
	const errors = $derived(validate(draft));
	const dirty = $derived(isDirty(draft, app.settings));
	let saveFailed = $state(false);
	let saving = $state(false);

	async function save() {
		saveFailed = false;
		if (Object.keys(errors).length > 0) {
			// 誤りのある入力欄があるのはVRChatのタブ。そこへ移って、誤りを見せる
			tab = 'vrchat';
			return;
		}
		const patch = toPatch(draft, app.settings);
		if (!patch) return;
		saving = true;
		try {
			await app.updateSettings(patch);
			draft = toDraft(app.settings);
		} catch (e) {
			console.error('設定を保存できません', e);
			saveFailed = true;
		} finally {
			saving = false;
		}
	}

	// ---- 一般：変えた時点で保存して適用する ----
	let generalFailed = $state(false);
	// 保存に失敗したら、入力欄を作り直して保存済みの値に戻す
	let generalKey = $state(0);

	async function saveGeneral(patch: NonNullable<SettingsPatch['general']>) {
		generalFailed = false;
		try {
			await app.updateSettings({ general: patch });
		} catch (e) {
			console.error('設定を保存できません', e);
			generalFailed = true;
			generalKey++;
		}
	}

	let licensesOpen = $state(false);
	const selectValue = (e: Event) => (e.currentTarget as HTMLSelectElement).value;
</script>

<Tabs {tabs} bind:selected={tab} label={m['nav.settings']()}>
	{#snippet panel(id)}
		{#if id === 'display'}
			<div class="form-grid">
				<label class="field">
					<span>{m['settings.display.size']()}</span>
					<input
						type="range"
						class="range"
						min={SIZE_INCH_MIN}
						max={SIZE_INCH_MAX}
						step={SIZE_INCH_STEP}
						bind:value={draft.size_inch}
						aria-valuetext={m['settings.display.size_value']({ inch: draft.size_inch.toFixed(1) })}
					/>
					<span class="muted">{m['settings.display.size_value']({ inch: draft.size_inch.toFixed(1) })}</span>
				</label>
				<label class="field">
					<span>{m['settings.display.brightness']()}</span>
					<input type="range" class="range" min="0" max="100" step="1" bind:value={draft.brightness_pct} />
				</label>
				<Select labelText={m['video.source_label']()} bind:selected={draft.video_source}>
					<SelectItem value="hdmi" text={m['video.source.hdmi']()} />
					{#if app.phoneKind !== 'iphone' || draft.video_source === 'mirror'}
						<SelectItem value="mirror" text={m['video.source.mirror']()} />
					{/if}
					<SelectItem value="window" text={m['video.source.window']()} />
				</Select>
				<Select labelText={m['settings.display.crop']()} bind:selected={draft.crop_mode}>
					<SelectItem value="auto" text={m['settings.display.crop_auto']()} />
					{#if draft.crop_mode === 'manual'}
						<SelectItem value="manual" text={m['settings.display.crop_manual']()} />
					{/if}
				</Select>
			</div>
		{:else if id === 'position'}
			<div class="form-grid">
				<Select labelText={m['wizard.calibrate.source_label']()} bind:selected={draft.pose_source}>
					<SelectItem value="controller" text={m['pose.source.controller']()} />
					<SelectItem value="hand" text={m['pose.source.hand']()} />
				</Select>
				<Select labelText={m['wizard.calibrate.side_label']()} bind:selected={draft.pose_side}>
					<SelectItem value="left" text={m['side.left']()} />
					<SelectItem value="right" text={m['side.right']()} />
				</Select>
			</div>
		{:else if id === 'input'}
			<SwitchField
				checked={draft.show_touches}
				label={m['settings.input.show_touches']()}
				onchange={(v) => (draft.show_touches = v)}
			/>
			<SwitchField
				checked={draft.keep_awake}
				label={m['settings.input.keep_awake']()}
				onchange={(v) => (draft.keep_awake = v)}
			/>
		{:else if id === 'vrchat'}
			<div class="form-grid mono">
				<TextInput
					labelText={m['settings.vrchat.send_port']()}
					bind:value={draft.send_port}
					inputmode="numeric"
					invalid={errors.send_port}
					invalidText={m['settings.vrchat.port_invalid']()}
				/>
				<TextInput
					labelText={m['settings.vrchat.receive_port']()}
					bind:value={draft.receive_port}
					inputmode="numeric"
					invalid={errors.receive_port}
					invalidText={m['settings.vrchat.port_invalid']()}
				/>
				<TextInput
					labelText={m['settings.vrchat.param_visible']()}
					bind:value={draft.param_visible}
					invalid={errors.param_visible}
					invalidText={m['settings.vrchat.param_invalid']()}
				/>
				<TextInput
					labelText={m['settings.vrchat.param_hand']()}
					bind:value={draft.param_is_left}
					invalid={errors.param_is_left}
					invalidText={m['settings.vrchat.param_invalid']()}
				/>
			</div>
			<Banner tone="info">{m['settings.vrchat.bits_note']()}</Banner>
		{:else}
			{#key generalKey}
				<div class="form-grid">
					<Select
						labelText={m['settings.general.language']()}
						selected={app.settings.general.language}
						on:change={(e) => saveGeneral({ language: selectValue(e) as typeof app.settings.general.language })}
					>
						<SelectItem value="auto" text={m['settings.general.language_auto']()} />
						{#each PUBLIC_LOCALES as locale (locale)}
							<SelectItem value={locale} text={languageName(locale)} />
						{/each}
					</Select>
					<Select
						labelText={m['settings.general.theme']()}
						selected={app.settings.general.theme}
						on:change={(e) => saveGeneral({ theme: selectValue(e) as typeof app.settings.general.theme })}
					>
						<SelectItem value="system" text={m['settings.general.theme_system']()} />
						<SelectItem value="light" text={m['settings.general.theme_light']()} />
						<SelectItem value="dark" text={m['settings.general.theme_dark']()} />
					</Select>
				</div>
				<SwitchField
					checked={app.settings.general.start_with_steamvr}
					label={m['settings.general.start_with_steamvr']()}
					onchange={(v) => saveGeneral({ start_with_steamvr: v })}
				/>
				<SwitchField
					checked={app.settings.general.start_minimized}
					label={m['settings.general.start_minimized']()}
					onchange={(v) => saveGeneral({ start_minimized: v })}
				/>
			{/key}
			<!-- ロック解除の案内（S01h）はT6-3でつなぐ。それまでは押せない -->
			<button type="button" class="link" disabled>{m['settings.general.unlock_help']()}</button>
			<button type="button" class="link" onclick={() => (licensesOpen = true)}>{m['settings.general.licenses']()}</button>
			{#if generalFailed}
				<Banner tone="err">{m['settings.save_failed']()}</Banner>
			{/if}
		{/if}
	{/snippet}
</Tabs>

{#if tab !== 'general'}
	{#if saveFailed}
		<Banner tone="err">{m['settings.save_failed']()}</Banner>
	{/if}
	<div class="row end">
		<Button kind="primary" disabled={!dirty || saving} on:click={save}>{m['common.save']()}</Button>
	</div>
{/if}

<LicenseDialog bind:open={licensesOpen} />

<style>
	.form-grid {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		gap: var(--cds-spacing-06) var(--cds-spacing-05);
	}
	.form-grid.mono :global(input) {
		font-family: var(--pih-font-family-mono);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-03);
	}
	.field > span:first-child {
		font-size: var(--cds-label-01-font-size);
		line-height: var(--cds-label-01-line-height);
		letter-spacing: var(--cds-label-01-letter-spacing);
		color: var(--cds-text-secondary);
	}
	.range {
		width: 100%;
		accent-color: var(--cds-interactive);
	}
	.muted {
		color: var(--cds-text-helper);
		font-size: var(--cds-helper-text-01-font-size);
		line-height: var(--cds-helper-text-01-line-height);
	}
	.link {
		font: inherit;
		align-self: flex-start;
		padding: 0;
		background: none;
		border: 0;
		color: var(--cds-link-primary);
		cursor: pointer;
	}
	.link:hover:not(:disabled) {
		color: var(--cds-link-primary-hover);
		text-decoration: underline;
	}
	.link:disabled {
		color: var(--cds-text-disabled);
		cursor: not-allowed;
	}
	.row.end {
		display: flex;
		justify-content: flex-end;
		margin-top: auto;
	}
</style>
