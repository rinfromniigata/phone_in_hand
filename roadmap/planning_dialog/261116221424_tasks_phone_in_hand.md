# phone_in_hand 実装タスク（tasks）

設計：`261116221424_imple_phone_in_hand.md`。検証：`261116221424_walk_phone_in_hand.md`。
各Phaseは、Tn → Wn → 修正と再検証 → Wnがすべて通過したらPhase n+1、の順に進める。

## Phase 1: 基盤
Phaseの推奨: Opus・high以上
引き継ぎ: Phase 1は完了（W1-1・W1-2通過）。次はT2-1。Phase 2では、実装した画面のキーを`scripts/i18n_pending.json`から消す
- [x] T1-1 【エージェント】プロジェクトの骨格を作る：Tauri 2系＋Bun＋TypeScript＋SvelteKit（静的出力）＋carbon-components-svelte、Cargoのワークスペース（`crates/pih_core`、`src-tauri`）。識別子`com.rinontech.phone-in-hand`、ウィンドウ960×680、対象はWindows x64だけ
  - 推奨: Sonnet・high以上
  - 各ライブラリの版とAPI、初期化のコマンドは、インストールする版の公式ドキュメントで確かめる
  - 実施: Opus・low
  - メモ: SvelteKit 3.2は`svelte.config.js`を廃止し設定は`vite.config.ts`の`sveltekit()`に渡す。tsconfigは`$app/tsconfig`を継承、`#lib`は`package.json`の`imports`。TypeScriptはSvelteKitとsvelte-checkの要件で6.0系。Tauri CLI 2.12、`tauri init --ci`で生成し、バンドルはNSISだけ。アイコンは仮（T10-1で置き換え）
- [x] T1-2 【エージェント】トークンを配置する：`specs/`の`tokens_v1.css`を`src/lib/theme/tokens.css`へ、`brand_v1.md`と`icon_master_v1.svg`を`docs/brand/`へ版番号なしでコピーする。Carbonのg100・whiteのテーマを`data-theme`で切り替える経路を作り、コンポーネントはトークン経由でだけ値を参照させる。IBM Plex（Sans・Sans JP・Sans SC・Mono）を`static/fonts/`に同梱し、外部から読み込まない
  - 推奨: Sonnet・medium以上
  - 依存: T1-1
  - 実施: Opus・low
  - メモ: Carbonの`all.css`はIBMのCDNから書体を読むため、Viteのプラグイン（`scripts/vite_carbon_css.ts`）で`@font-face`を除き、書体の指定を`--pih-font-family-*`に置き換え、全体を`@layer carbon`に入れて`tokens.css`を優先させた。書体（Regular・SemiBold）は`@ibm/plex-*`から`scripts/copy_fonts.ts`で`static/fonts/`へコピーし、リポジトリには含めない
- [x] T1-3 【エージェント】i18nの基盤を作る：Paraglide JS 2系を導入し、モック5ファイルの辞書（`mock.*`以外のキー）を`messages/{ja,en,zh-Hans}.json`へ移す。ロケールの判定（設定の`auto`ならOSのロケール）・保持・jaへのフォールバック、`<html lang>`の更新、ロケールごとの書体、開発時だけ有効な疑似ロケールを用意する
  - 依存: T1-1
  - 実施: Opus・low
  - メモ: Paraglide 2.29.3（最新は3系のため2系に固定）。キーはモックのドット区切りのまま`m['nav.home']()`で引く。155キー、ロケール間の食い違いなし。`mobile.*`（S05）もPCの辞書に移し、T8-2でここからstring resourcesを作る前提とする。OSのロケールは`tauri-plugin-os`。疑似ロケール`en-XA`は`.pseudo/`の開発用プロジェクトで`vite dev`のときだけ加わり、公開版のビルドには含まれない
- [x] T1-4 【エージェント】Rust側のインターフェースと状態の骨格を作る：`pih_core`に型とトレイト（imple 2章）と偽の実装、`settings_store`（JSON、スキーマの版、既定値、一時ファイルからの置き換え）と単体テスト、`app_state`（`AppSnapshot`、`state_changed`イベント、imple 2章のコマンド。未実装のモジュールは偽の実装で「使用できません」を返す）。UI側に、スナップショットと設定を受け持つストア（`src/lib/state/`）を作る
  - 依存: T1-1
  - 実施: Opus・low
  - メモ: `settings_store`は`pih_core`に置いた（Tauri非依存）。読めないファイルは`*.broken`へ退避して既定値で続ける。`update_settings`・`complete_setup`はJSON Merge Patchを受ける。UIの既定値`src/lib/state/default_settings.json`はRustのテストで一致を検査する（`UPDATE_FIXTURES=1 cargo test -p pih_core`で更新）。`PoseFusion::update`はT3-4まで常に非表示を返す。センサーの待ち受けポートの既定値は39570とした（委任事項）
- [x] T1-5 【エージェント】アプリの枠を作る：UI Shellのサイドナビ（アイコンのレール、HomeとSettingsのアイコン、ツールチップと`aria-label`）、ホームと設定の経路、設定に従うテーマとロケールの適用、フォーカスの見える状態。開発時だけ有効な「状態の見本」の切り替え（imple 3章）を用意する
  - 推奨: Sonnet・high以上
  - 依存: T1-2, T1-3, T1-4
  - 画面: S02, S03（枠の部分だけ）
  - 実施: Opus・low
  - メモ: レールはモックのアイコンだけの形に合わせて自作（`src/lib/components/RailNav.svelte`。CarbonのSideNavのrailはホバーで広がるため）。ツールチップはホバーとキーボードのフォーカスで出す。モックのタイトルバーはOSの枠で代え、タイトルは辞書の`app.name`を設定する。開発用の切り替え（状態の見本S02a〜c、疑似ロケールを含む言語）は`src/lib/dev/`で、`vite dev`のときだけ右下に出る。`tauri dev`で起動を確認した
- [x] T1-6 【エージェント】checkコマンド`bun run check`を作る：型（svelte-check）、ESLint、`cargo fmt --check`・`clippy -D warnings`・`cargo test`、i18nの自動検査（キーの過不足、プレースホルダ、未使用キー、直書き）、トークンと`docs/brand/`の`specs/`との一致、`src/`の色・サイズの直書きの検出。失敗した部分だけを表示する
  - 依存: T1-5
  - 実施: Opus・low
  - メモ: `scripts/check.ts`が10項目を順に実行し、失敗した項目の出力だけを出す（公開版の辞書のコンパイル、svelte-kit sync、`bun test`も含む）。未実装の画面のキーは`scripts/i18n_pending.json`に画面IDごとに置き、各Phaseで画面を実装したら消す（使われているのに残っていれば検査が指摘する）。`mobile.*`はAndroid用として未使用キーの検査から除く。`src/lib/dev/`は直書きの検出から除く。試しのファイルで、直書き・サイズ・色・specsとの不一致を検出できることを確かめた
- [x] T1-7 【エージェント】`CONTRIBUTING.md`の骨格を作る：受け付け方針、開発環境、セットアップ、checkコマンド、リポジトリ構成（`roadmap/`・`docs/brand/`の役割を含む）
  - 推奨: Sonnet・medium以上
  - 依存: T1-6
  - 実施: Opus・low
  - メモ: 起動（`tauri dev`・ブラウザでの`dev`）、開発用の切り替え、守ること（値・文言・保留中のキー・設定の既定値）、データの保存場所も書いた。`build/`のないクローン直後でも`cargo clippy`が通ることを確かめた

## Phase 2: ホームと設定
Phaseの推奨: Sonnet・high以上
引き継ぎ: Phase 2は完了（W2-1〜W2-3通過）。次はT3-1。開発用の切り替え（右下のDEV）を開くと、右下の保存・位置を合わせ直すボタンに重なる（開発時だけ）。映像のフレームの受け取り（`preview_frame`）はT4-2でつなぐ（`VideoPreview`の`frameUrl`）
- [x] T2-1 【エージェント】ホームを実装する：VRに表示のスイッチ、5つの状態の行（色と文字の両方）、映像のプレビュー、位置を合わせ直すボタン
  - 依存: T1-7
  - 画面: S02, S02a, S02b, S02c
  - 位置を合わせ直すボタンの遷移先（S01e）はT3-8でつなぐ。T2-1では配置だけを行い、押せない状態にする
  - 実施: Sonnet・high
  - メモ: 状態の行は`src/lib/state/status_rows.ts`（`AppSnapshot`→行と帯。S02a〜cの見本と一致することを単体テストで確認）。iPhoneのときはセンサーとワイヤレスデバッグの行を「使用できません」にする（S02c）。映像のプレビューは枠と空の状態まで（フレームの受け取りはT4-2）。開発用の見本のときだけ、見本の絵（`src/lib/dev/FakeScreen.svelte`）を出す。モックにない文言を6キー足した（`home.detail.adb_not_used`ほか）。位置合わせの結果が古いこと（`calibration: stale`）の表示はT3-8で扱う
- [x] T2-2 【エージェント】設定を実装する：タブ（表示・位置・入力・VRChat・一般）、保存、言語とテーマの即時の適用。一般タブの末尾にライセンスのリンクとダイアログ（スペック9a節。一覧は`src/lib/licenses`の一覧ファイルから表示する）
  - 依存: T1-7
  - 画面: S03, S03a, S03b, S03c
  - S04dとアバター用パッケージの要素は表示しない（スペック9a節）。取り込み方法を変えたときの映像のページはT3-7・T4-2・T7-3で、ロック解除の案内はT6-3でつなぐ
  - 実施: Sonnet・high
  - メモ: 表示・位置・入力・VRChatは下書き（`src/lib/state/settings_draft.ts`）で、保存で変えた項目だけを`update_settings`へ送る。一般は変えた時点で保存して適用する（S03cに保存ボタンがないため）。`src/lib/licenses/licenses.ts`を新設し、著作権表示は依存のLICENSEとscrcpyのREADMEで照合した（scrcpy-serverはPhase 6で同梱する）。`settings.vrchat.open_package`の辞書キーは、スペック9a節で表示しないため削除した。ロック解除の案内のリンクは押せない状態。位置の基準と側は「位置」、入力の2つは「入力」のタブに置いた（モックのS03aは表示タブに全項目を描いている。仕様変更でスペック9a節に反映：`development/261116220850_change_settings_tabs.md`）。fix: `roadmap/development/261116215733_fix_phase2.md`（Tauri外の設定変更の失敗）

## Phase 3: オーバーレイの縦切りと位置合わせ
Phaseの推奨: Opus・high以上
引き継ぎ: T3-1〜T3-9は完了、checkコマンドは通過。次はW3-1。この環境にはSteamVRがなく、オーバーレイの表示・遅延・入力の既定の割り当て（`src-tauri/resources/openvr/`）・自動起動は未確認で、W3-3〜W3-6（ユーザー）で確かめる。設定の表示タブで取り込み方法を「ウィンドウ」に変えるとページへ移り、タブの未保存の変更は破棄される
- [x] T3-1 【エージェント】オーバーレイの試作：`ovr_overlay`でSteamVRにつなぎ、D3D11のテクスチャをオーバーレイに出し、毎フレームの絶対配置で動かせるかを確かめる。足りない場合は、C++の薄いサイドカーへの切り替えを決め、実施メモに理由を書く
  - 依存: T2-2
  - 実施: Opus・high
  - メモ: `ovr_overlay`は不足と判断し、C++の薄い層に切り替えた。理由：crates.ioの版は0.0.0（2022年）のままで、D3D11のテクスチャの設定（`SetOverlayTexture`）・機器の姿勢・`IVRInput`・`IVRApplications`のAPIがなく、ビルドにlibclang（autocxx 0.21）が要る。OpenVR SDK v2.15.6（BSD-3）の`openvr.h`と`openvr_api.dll`を`src-tauri/vendor/openvr/`に置き、`src-tauri/native/pih_openvr.cpp`を`cc`でアプリに組み込む（別プロセスにせず同じプロセスに置いた。姿勢とテクスチャを毎フレーム渡すため）。DLLは実行時に`LoadLibrary`で読むため、SteamVRがなくてもアプリは起動する。Rust側は`src-tauri/src/openvr.rs`。この環境にはSteamVRがなく、DLLの読み込みと関数の解決、接続の失敗の扱いまでを単体テストで確かめた。VRでの表示と遅延はW3-3で確かめる
- [x] T3-2 【エージェント】`window_capture_source`を実装する：Windows.Graphics.Capture APIでウィンドウをGPUのテクスチャとして受け取る。ウィンドウの一覧（`list_windows`）と、ウィンドウが閉じたときの状態
  - 依存: T3-1
  - 実施: Opus・high
  - メモ: `src-tauri/src/window_capture.rs`（WGC）と`gpu.rs`（D3D11のデバイス、ミップマップでの縮小と読み出し）。`windows` 0.62。ウィンドウは実行ファイル名を優先して探す（タイトルが変わっても続く）。閉じたら`error: source_lost`、見つからなければ`open_failed`。`Frame::Gpu`はテクスチャのポインタとフレームの保持者（`owner`）、間引いた縮小画像（200msごと、幅360以下）を持つ形にした。手元のウィンドウを取り込み、フレームと縮小画像が得られることを確かめた（`cargo test -- --ignored`の`captures_a_real_window`）
- [x] T3-3 【エージェント】`controller_pose_source`を実装する：左右のコントローラーの姿勢と追跡状態
  - 推奨: Sonnet・high以上
  - 依存: T3-1
  - 実施: Opus・high
  - メモ: `src-tauri/src/controller_pose.rs`。姿勢はOpenVRの説明にある求め方で、次に表示されるときの予測の値を使う（`pih_openvr.cpp`の`pih_vr_controller_pose`）。接続がない・機器がない・`TrackingResult_Running_OK`でないときは`Lost`。OpenVRの3×4行列と`Pose`の変換は`pih_core::geometry`に置き、単体テストで固定した
- [x] T3-4 【エージェント】`pose_fusion`のコントローラー固定を実装する：位置合わせのずれの適用、追跡が切れたら最後の姿勢に止めて薄く表示、戻れば元に戻す、`screen_off`なら隠す。単体テストで振る舞いを固定する
  - 依存: T1-4
  - 実施: Opus・high
  - メモ: `crates/pih_core/src/pose.rs`。`PanelState`に`screen_off`と`has_pose`を足し、出すチップ（`Chip`、辞書の`vr.chip.*`に対応）を`PanelState::chip`で決める。位置合わせの結果がない・基準や側が合わないときは既定のずれ（`default_offset`：コントローラーの7cm上、上端を20度向こうへ）を使う。隠している間も姿勢は追い、まだ一度も姿勢がないときはチップも出さない。単体テスト6件
- [x] T3-5 【エージェント】`frame_processor`を実装する：黒い余白の自動の切り抜きと手動の矩形、黒い画面と取り込み元の途絶の検出（間引いた縮小画像で判定）。単体テストを付ける
  - 推奨: Sonnet・high以上
  - 依存: T1-4
  - 実施: Opus・high
  - メモ: `crates/pih_core/src/frame.rs`の`BlackBarProcessor`。閾値は同じファイルの定数（黒＝明るさ24以下、黒い画面＝黒でない画素0.2%未満が1.5秒続く、切り抜き＝行・列の2%以上が黒でない範囲で、同じ候補が3回続いたら変える）。画面がつけばすぐ戻す。CPUのフレーム（Phase 4）は`Thumbnail::from_cpu`で縮小してから判定する（MJPEGは未対応）。取り込み元の途絶は、フレームが来ないこと（WGCは画面が変わらないと送らない）ではなく取り込みの状態で判定し、`frame::screen_off`にまとめた。単体テスト6件
- [x] T3-6 【エージェント】`overlay_presenter`と映像・姿勢のスレッドを実装する：映像は`video_source`→`frame_processor`→`overlay_presenter`と直接渡し、GPUのフレームはCPUを経由しない。表示・薄く表示・非表示、大きさと明るさ、状態のチップの文言（辞書から）。ホームのスイッチとプレビュー（間引いたJPEG）をつなぐ
  - 依存: T3-2, T3-3, T3-4, T3-5
  - 画面: S04, S04a, S04c, S04e
  - 実施: Opus・high
  - メモ: `src-tauri/src/overlay.rs`（パネルとチップの2つのオーバーレイ。映像はGPUの中で共有テクスチャへ写し、切り抜きはテクスチャの範囲で指定）、`runtime.rs`（映像のスレッドと姿勢のスレッド。SteamVRへの接続・つなぎ直し・終了の受け入れも姿勢のスレッド）。`app_state`は`RuntimeLink`でスレッドへ設定を伝え、`on_runtime_event`で映像と追跡の状態を受ける。パネルとチップの寸法・薄く表示の不透明度は`pih_core::overlay`の定数。チップと位置合わせの見本の画像は、UIが辞書の文言とトークンの値でキャンバスに描いて`set_overlay_images`で渡す（`src/lib/vr/overlay_images.ts`。言語・テーマを変えると描き直す）。`start_video`は保存していない映像の設定（`video`）を受ける形にした。プレビューは`preview_frame`（切り抜く前の縮小画像のJPEGと切り抜きの範囲）で、ホームは切り抜いた範囲を出す。HDMIとミラーリングは「使用できません」のまま（Phase 4・7）。手元のウィンドウで、映像のスレッドが状態とプレビューを送ることを確かめた（`runs_with_a_real_window`）
- [x] T3-7 【エージェント】ウィンドウを取り込むページを実装する：ウィンドウの一覧、プレビューと切り抜きの範囲（自動・手動の矩形の調整）。設定の表示タブで取り込み方法をウィンドウに変えたときに単独で開く
  - 推奨: Sonnet・high以上
  - 依存: T3-6
  - 画面: S01g, S03a
  - 実施: Opus・high
  - メモ: ページの本体は`src/lib/components/setup/WindowSource.svelte`（T5-3でステップ表示と「次へ」を差し込めるよう、`steps`・`actions`を受ける）、単独のページは`src/routes/video/window/`（「戻る」「保存」。保存で設定へ戻る）。選んだウィンドウと切り抜きは、保存前から`start_video`でプレビューに使い、離れると保存済みの設定に戻す。手動の範囲は`CropEditor.svelte`（ドラッグと、つまみの矢印キー）。取り込み方法の選択肢は`Segmented.svelte`で、HDMI・ミラーリングはページができるまで押せない。辞書に6キーを足した（`wizard.video.crop_edge_*`、`wizard.window.list_choose`・`list_empty`）。設定の表示タブは、保存済みと違う「ウィンドウ」を選んだ時点でこのページへ移る（タブの未保存の変更は破棄される）
- [x] T3-8 【エージェント】位置合わせを実装する：位置合わせのページ（基準と側の選択、始める、VRで進行中の表示、終わると自動で進む）と、VRの中での見本の表示、もう一方のコントローラーでつかんで動かす操作、トリガーでの決定、`calibration.json`の保存。ホームの位置を合わせ直すボタンをつなぐ。途中でやめたら前の結果を残す
  - 依存: T3-6
  - 画面: S01e, S02
  - ハンドトラッキングの選択肢は、手の追跡がない間は選べない状態で示す（T9-4で使えるようにする）
  - 実施: Opus・high
  - メモ: 手順は`crates/pih_core/src/calibration.rs`（つかんでいる間はもう一方のコントローラーに付けて動かし、放すとスマホ側に付いたまま残る。トリガーは押した瞬間だけ決定にし、始めた時点で押していたものは離すまで無視。単体テスト5件）。姿勢のスレッドが入力（`IVRInput`。定義は`src-tauri/resources/openvr/`の`actions.json`と、Index・Touch・Vive・WMRの既定の割り当て）を読み、見本は表示のスイッチや画面の状態によらず出す（映像がなければUIが描いた見本の画像）。決定すると`calibration.json`を上書きし、選んだ基準と側を設定に保存して、`calibration_progress`で知らせる。ページは`src/lib/components/setup/Calibrate.svelte`と`src/routes/calibrate/`（戻る・離れるとやめて前の結果を残す）。ホームの「位置を合わせ直す」をつなぎ、位置合わせの結果が古い・ないときの帯を足した。辞書に3キーを足し、`wizard.calibrate.vr_hint`の言い回しに、グリップでつかんで動かすことを加えた。SteamVRがないため、VRでの操作と既定の割り当てはW3-4で確かめる
- [x] T3-9 【エージェント】起動の設定を実装する：SteamVRと一緒に起動する（アプリのマニフェストの登録と解除）、起動時にウィンドウを最小化する
  - 推奨: Sonnet・high以上
  - 依存: T3-1
  - 画面: S03c
  - 実施: Opus・high
  - メモ: `src-tauri/src/startup.rs`。起動のたびに、今の実行ファイルの場所で`phone_in_hand.vrmanifest`をアプリデータのフォルダに書き（アプリ名は辞書の`app.name`をビルド時に読む）、設定がオンならSteamVRに登録して自動起動を有効に、オフなら自動起動を切って登録を解除する。合わせるのは、設定を変えたときとSteamVRにつながったとき（つながっていない間の変更は次の接続で反映）。起動時の最小化は`setup`でメインのウィンドウを最小化する。SteamVRから起動されたときに2つ目のアプリが立ち上がるのを防ぐ仕組み（単一インスタンス）はない。あわせて、`CONTRIBUTING.md`にOpenVRの層・入力の定義・手元でのテストを、ライセンスの一覧にOpenVR SDKを足した。fix: `roadmap/development/261116232120_fix_phase3.md`（変化のないウィンドウでプレビューと黒い画面の判定が進まない）

## Phase 4: HDMIキャプチャ
Phaseの推奨: Opus・high以上
- [ ] T4-1 【エージェント】`hdmi_capture_source`を実装する：`nokhwa`（Media Foundation）でデバイスと解像度・フレームレートの一覧と取り込み。CPUのフレームはGPUへのアップロード1回にする。デバイスなし・信号なしの状態
  - 依存: T3-6
- [ ] T4-2 【エージェント】HDMIキャプチャのページを実装する：デバイスと解像度の選択、プレビューと切り抜き、見つからないときの理由と再試行。設定の表示タブで取り込み方法をHDMIキャプチャに変えたときに単独で開く
  - 推奨: Sonnet・high以上
  - 依存: T4-1
  - 画面: S01a, S01b, S03a

## Phase 5: VRChat連携とセットアップの枠
Phaseの推奨: Opus・high以上
- [ ] T5-1 【エージェント】`vrchat_bridge`を実装する：`vrchat_osc`でVRChatを見つけ、OSCQueryでパラメータの有無を調べる。`rosc`で表示中と持ち手を送り、表示・持ち手の変更とVRChatの再接続のたびに送り直す。偽のOSCQueryで単体テストを付ける
  - 依存: T3-6
- [ ] T5-2 【エージェント】VRChatとつなぐページ、設定のVRChatタブ（ポートとパラメータ名）、ホームのVRChatの行をつなぐ
  - 推奨: Sonnet・high以上
  - 依存: T5-1
  - 画面: S01d, S03b, S02
  - スペック9a節に従い、アバターへの組み込み方のリンクは出さず、説明文の言い回しを変える
- [ ] T5-3 【エージェント】初回セットアップの枠を実装する：ステップ表示（映像→スマホ→VRChat→位置合わせ）、戻る・次へ、`setup_completed`がfalseなら起動時に開く、完了で設定をまとめて保存する。映像のステップは取り込み方法の選択から各ページ（S01a・S01g）へ進む。このPhaseでは、スマホのステップを除いた3つのステップを並べる
  - 依存: T5-2, T4-2, T3-8
  - 画面: S01
  - スマホのステップはT6-3で、Androidのミラーリングの選択肢はT7-3で加える

## Phase 6: ADBとスマホのステップ
Phaseの推奨: Opus・high以上
- [ ] T6-1 【エージェント】ADBの調査：`adb_client`の最新版で、adbサーバーなしにペア設定・接続・インストール・設定の変更ができるかを、公式の資料と試作で確かめる。結果と、未決定項目（adbの本体を(a)同梱するか(b)利用者に入れてもらうか）の推奨をユーザーに報告して停止する。決定はスペックの記述を変えるため、仕様変更（change）として扱う
  - 依存: T5-3
- [ ] T6-2 【エージェント】`adb_controller`を実装する：鍵の作成、ペア設定、接続（USB・Wi-Fi）、`adb_devices`の保存、「タップを表示」と画面を消さない設定の切り替え（変更前の値を`restore_state.json`へ保存）、終了・切断・VRChatとの切断での復元、異常終了後の復元
  - 依存: T6-1
- [ ] T6-3 【エージェント】スマホのステップを加える：スマホとつなぐページ（ワイヤレスデバッグのペア設定、あとで設定する、iPhoneを使うボタン）、ロック解除の準備のページ、設定の入力タブ、設定の一般タブのロック解除の案内、ホームのワイヤレスデバッグの行とiPhoneのときの表示
  - 推奨: Sonnet・high以上
  - 依存: T6-2
  - 画面: S01, S01c, S01h, S03a, S03c, S02, S02c
  - S01cのセンサー送信アプリの節は、T9-4で加える

## Phase 7: Androidのミラーリング
Phaseの推奨: Opus・high以上
- [ ] T7-1 【エージェント】scrcpy-server 4.1を取得するスクリプト（版とチェックサムの固定）と、`src-tauri/resources/`への配置、ライセンスの一覧への追加
  - 推奨: Sonnet・medium以上
  - 依存: T6-3
- [ ] T7-2 【エージェント】`android_mirror_source`を実装する：ADBでscrcpy-serverを送って起動し、映像のソケットからH.264・H.265を受けて、Media Foundationのデコーダーで共有テクスチャに出す。USB・Wi-Fi、形式の自動（H.265を優先）、途絶の状態
  - 依存: T7-1
- [ ] T7-3 【エージェント】Androidのミラーリングのページを実装し、セットアップの映像のステップと設定の表示タブの取り込み方法に加える
  - 推奨: Sonnet・high以上
  - 依存: T7-2
  - 画面: S01f, S01, S03a

## Phase 8: Androidのセンサー送信アプリ
Phaseの推奨: Opus・high以上
- [ ] T8-1 【ユーザー】Androidの開発環境（Android StudioかAndroid SDKのコマンドラインツールと、Android Gradle Pluginが求める版のJDK）を入れる
- [ ] T8-2 【エージェント】`android/`のプロジェクトを作る：Kotlin＋Jetpack Compose、パッケージ名`com.rinontech.phoneinhand`。`tokens.css`から`Tokens.kt`を生成するスクリプト、モックS05の辞書からstring resources（`values`・`values-en`・`values-b+zh+Hans`）、開発ビルドの疑似ロケール。checkコマンドにGradleのlint・単体テスト、生成物とstring resourcesの検査を加える
  - 依存: T8-1, T7-3
- [ ] T8-3 【エージェント】センサー送信サービスを実装する：フォアグラウンドサービス（種類`connectedDevice`。前提条件を公式ドキュメントで確かめる）、`TYPE_GAME_ROTATION_VECTOR`を60Hz目安でUDP送信、送信中の通知、接続情報の受け取り（ADBから・QRコード・IPアドレス）と保存
  - 依存: T8-2
- [ ] T8-4 【エージェント】Androidアプリの画面を実装する：未接続（QRコードを読み取る、手動、バッテリーの最適化の設定を開く）、送信中、エラー（確認すべき点と再試行）
  - 推奨: Sonnet・high以上
  - 依存: T8-3
  - 画面: S05, S05a, S05b, S05c
- [ ] T8-5 【エージェント】`CONTRIBUTING.md`にAndroidの開発環境、ビルド、checkコマンドの前提を加える
  - 推奨: Sonnet・medium以上
  - 依存: T8-4

## Phase 9: ハンドトラッキング
Phaseの推奨: Opus・high以上
- [ ] T9-1 【エージェント】`phone_orientation_source`を実装する：UDPで受け、トークンが一致しないパケットと古い連番を捨てる。単体テストを付ける
  - 推奨: Sonnet・high以上
  - 依存: T8-5
- [ ] T9-2 【エージェント】`hand_pose_source`を実装する：Skeletal Inputから手の位置と向き、追跡状態。手の追跡がないヘッドセットでは使用できない状態を返す
  - 依存: T9-1
- [ ] T9-3 【エージェント】`pose_fusion`のハンドトラッキングを実装する：手の位置＋スマホの回転、ヨーを手の向きを基準に緩やかに補正、センサーが1秒途絶えたら手の向きで代用。単体テストで振る舞いを固定する
  - 依存: T9-2
- [ ] T9-4 【エージェント】センサー送信アプリとの接続をつなぐ：接続トークンの作成と保存、QRコードの内容、ADBでのインストールと接続情報の送信、スマホとつなぐページのセンサー送信アプリの節、ホームのセンサーと位置の追跡の行、位置合わせのページのハンドトラッキングの選択肢、VR内のセンサー途絶のチップ
  - 依存: T9-3
  - 画面: S01c, S01e, S02, S02b, S04b

## Phase 10: アイコンの展開とリリース準備
Phaseの推奨: Sonnet・high以上
- [ ] T10-1 【エージェント】アイコンを展開する：マスターSVGからWindowsの`.ico`とインストーラー用（`tauri icon`等）、Androidのアダプティブアイコン（前景は`01_palm`〜`04_thumb`、背景は`00_bg`）。生成したアイコンはリポジトリに含める（CIでは生成しない）。コマンドと入力形式は、インストール済みの版の公式ドキュメントで確かめる
  - 推奨: Sonnet・medium以上
  - 依存: T9-4
- [ ] T10-2 【ユーザー】Androidの署名の鍵を作って保管し、GitHubのSecretsに登録する（登録する名前はエージェントがT10-3で示す）
  - 依存: T10-1
- [ ] T10-3 【エージェント】リリースのビルドを整える：署名済みAPKのビルドと、PCアプリの`resources/`への同梱、NOTICEとライセンスの一覧の完成（同梱物をすべて含め、GPLを含まないことを確かめる）、`CONTRIBUTING.md`のビルドとリリースの手順
  - 依存: T10-2
- [ ] T10-4 【エージェント】CIのワークフローを作る：checkコマンドを呼び、`v*`タグのpushで、tauri-actionのWindows x64のインストーラー（スペックに従いmacOSは作らない）と署名済みAPKを作ってドラフトリリースに置く
  - 依存: T10-3
- [ ] T10-5 【エージェント】READMEとGitHubのAbout欄の文案を作る：特徴、入手方法、動作環境、使い方、注意事項（SmartScreen、データの保存場所、外部へ送信しないこと、スペック3章の既知の制限、パススルーでの位置合わせ）。スクリーンショットは実物から撮る
  - 依存: T10-4

Phase 10のwalkがすべて通過したら、ユーザーの指示を受けて、リリース準備（CHANGELOG、版`0.1.0`をPCアプリとAndroidアプリのすべての版表記にそろえる、`timeline.md`へのリリース準備の行、コミットメッセージとタグのコマンドの提示）を、スキルの`release.md`の手順で行う（推奨: Sonnet・high以上）。
