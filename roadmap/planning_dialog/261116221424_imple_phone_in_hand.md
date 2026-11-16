# phone_in_hand 実装設計（imple）

正本：`roadmap/specs/phone_in_hand_SPEC_v2.md`（振る舞い・要件）、`phone_in_hand_tokens_v2.css`（値）、`phone_in_hand_mock_{画面ID}_v2.html`（画面構成）。この文書は、それらから導いた設計で、値と文言を複写しない。

## 1. リポジトリ構成

```
phone_in_hand/
├─ package.json            Bunのスクリプト（check、dev、build、i18n、tokens）
├─ src/                    SvelteKit（静的出力）のUI
│  ├─ lib/theme/           tokens.css（specsからのコピー）、Carbonのテーマの接続、書体
│  ├─ lib/i18n/            ロケールの判定・保持・フォールバック、lang属性
│  ├─ lib/state/           app_stateのイベントを受けるストア、Tauriコマンドの呼び出し
│  ├─ lib/components/      画面をまたいで使う部品（状態の行、ステップ表示など）
│  └─ routes/              home（S02）、settings（S03）、setup（S01）、calibrate（S01e単独）
├─ messages/               Paraglideの辞書（ja・en・zh-Hans。jaが原文）
├─ static/fonts/           IBM Plex（同梱。実行時に外部から読まない）
├─ crates/pih_core/        Tauriに依存しない純粋なロジック（型、トレイト、pose_fusion、frame_processorの判定、設定のモデル）。単体テストの中心
├─ src-tauri/              Tauriアプリと、OS・機器に依存する実装（OpenVR、Media Foundation、WGC、ADB、OSC、UDP）
│  └─ resources/           scrcpy-server、センサー送信アプリのAPK（ビルド時に配置）
├─ android/                センサー送信アプリ（Kotlin＋Jetpack Compose。Phase 7で作る）
├─ scripts/                checkコマンドの部品（i18nの検査、トークンの検査、直書きの検出、Android用トークンの生成）
└─ docs/brand/             brand.md、icon_master.svg（specsからのコピー。版番号なし）
```

- Cargoはワークスペース（`crates/pih_core`、`src-tauri`）。対象はWindows x64だけ
- `pih_core`の各トレイトには、テスト用の偽の実装（`fake`モジュール）を置く

## 2. Rust側のコンポーネントとインターフェース

モジュールの間はトレイトでつなぐ（スペック5章）。次は責務と主な型の定義で、細部は実装で確かめて決める。

```rust
// 映像
pub enum Frame {
    Gpu { texture: SharedTexture, size: Size },          // D3D11の共有テクスチャ（WGC、MFデコーダー）
    Cpu { format: PixelFormat, size: Size, data: Bytes }, // UVC（MJPEG・NV12・YUY2など）
}
pub enum VideoSourceKind { Hdmi, AndroidMirror, Window }
pub enum VideoStatus { Idle, NoDevice, NoSignal, Running { device: String, size: Size, fps: f32 }, Error(VideoError) }
pub trait VideoSource: Send {
    fn kind(&self) -> VideoSourceKind;
    fn start(&mut self, config: &VideoConfig) -> Result<()>;
    fn next_frame(&mut self, timeout: Duration) -> Result<Option<Frame>>; // Noneはタイムアウト
    fn stop(&mut self);
    fn status(&self) -> VideoStatus;
}

// 切り抜きと黒い画面・信号断の検出（取り込み方法によらず共通）
pub struct FrameInfo { pub crop: Rect, pub screen_off: bool }
pub trait FrameProcessor: Send {
    fn analyze(&mut self, frame: &Frame, now: Instant) -> FrameInfo; // 間引いた縮小画像で判定する
}

// 姿勢
pub enum TrackingState { Tracking, Lost }
pub struct PoseSample { pub pose: Option<Pose>, pub state: TrackingState } // Poseは位置＋四元数（SteamVRのstanding空間）
pub trait PoseSource: Send { fn sample(&mut self, side: Side) -> PoseSample; }

pub struct OrientationSample { pub rotation: Quat, pub received_at: Instant }
pub trait PhoneOrientationSource: Send { fn latest(&self) -> Option<OrientationSample>; }

pub enum PanelVisibility { Shown, Dimmed, Hidden }
pub enum PanelMode { Controller, Hand, HandFallback }
pub struct PanelState { pub pose: Pose, pub visibility: PanelVisibility, pub mode: PanelMode }
pub struct PoseFusion { /* 最後の姿勢、ヨー補正の状態 */ }
impl PoseFusion {
    pub fn update(&mut self, base: &PoseSample, orientation: Option<&OrientationSample>,
                  calibration: &Calibration, screen_off: bool, now: Instant) -> PanelState;
}

// 表示
pub trait OverlayPresenter: Send {
    fn set_frame(&mut self, frame: &Frame, crop: Rect) -> Result<()>;
    fn set_state(&mut self, state: &PanelState) -> Result<()>;
    fn set_appearance(&mut self, size_inch: f32, brightness: f32) -> Result<()>;
    fn set_enabled(&mut self, enabled: bool) -> Result<()>; // ホームのスイッチ
}

// ADB（Androidのみ）
pub trait AdbController: Send {
    fn pair(&mut self, host: &str, port: u16, code: &str) -> Result<()>;
    fn connect(&mut self, target: &AdbTarget) -> Result<DeviceInfo>;   // USBかWi-Fi
    fn install(&mut self, apk: &Path) -> Result<()>;
    fn apply_session_settings(&mut self, want: SessionSettings) -> Result<()>; // タップを表示、画面を消さない。変更前の値をrestore_stateに記録してから変える
    fn restore(&mut self) -> Result<()>;                               // 記録した値に戻し、記録を消す
    fn start_mirror(&mut self, opts: &MirrorOptions) -> Result<MirrorStream>; // scrcpy-serverの送信と起動
    fn send_sensor_config(&mut self, cfg: &SensorConfig) -> Result<()>;
}

// VRChat
pub struct VrchatStatus { pub osc_found: bool, pub params_found: bool }
pub trait VrchatBridge: Send {
    fn send_visible(&mut self, v: bool) -> Result<()>;
    fn send_is_left(&mut self, v: bool) -> Result<()>;
    fn query(&mut self) -> VrchatStatus; // OSCQueryとmDNS
}

// 保存
pub trait SettingsStore: Send {
    fn load(&self) -> Result<Stored>;   // ないときは既定値
    fn save_settings(&self, s: &Settings) -> Result<()>;
    fn save_calibration(&self, c: &Calibration) -> Result<()>;
    fn save_restore_state(&self, r: Option<&RestoreState>) -> Result<()>;
}
```

- **映像の経路**：映像のスレッドが`video_source.next_frame`→`frame_processor.analyze`→`overlay_presenter.set_frame`と直接渡す。GPUのフレームはテクスチャのまま渡し、CPUを経由しない。UVCのCPUのフレームは、GPUへのアップロード1回だけにする。UIのプレビューには、間引いて縮小したJPEGだけをイベントで送る（5fps程度）
- **姿勢の経路**：SteamVRのフレームに合わせて回る姿勢のスレッドが、`pose_source.sample`と`phone_orientation_source.latest`を読み、`pose_fusion.update`の結果を`overlay_presenter.set_state`に渡す。オーバーレイはデバイス（コントローラー）への相対配置ではなく、絶対配置で毎フレーム更新する（ハンドトラッキングと共通にするため。試作T3-1で遅延を確かめ、不足ならコントローラー固定だけ相対配置にする）
- **pose_fusion**：コントローラー固定は「コントローラーの姿勢×位置合わせのずれ」。ハンドトラッキングは「手の位置＋スマホの回転」で、ヨーは手の向きを基準に緩やかに補正する。センサー値が1秒届かなければ手の向きで代用（HandFallback）。追跡が切れたら最後の姿勢に止めてDimmed、戻ればShown。`screen_off`ならHidden。補正のアルゴリズムと閾値は委任事項で、`pih_core`の定数に置き、単体テストで振る舞いを固定する
- **位置合わせ（S01e）**：「位置合わせを始める」で、VRの中に画面の見本（映像の入ったパネル）を既定のずれで出す。利用者は、もう一方のコントローラーのグリップで見本をつかんで動かし・回し、パススルー（SteamVRのルームビュー、Questのパススルーなど）で見える実物のスマホに重ねて、トリガーで決定する。決定した時点の「コントローラーの姿勢に対する見本の相対姿勢」を`Calibration`として保存する。パススルーがない場合は、手の感覚で合わせる（READMEに書く）
- **overlay_presenter**：`ovr_overlay`で実装する。試作で足りなければ、C++の薄いサイドカー（OpenVRのC++ API）にトレイトの実装を差し替える（スペック4章で許可済み）
- **adb_controller**：`adb_client`（最新版）で実装する。adbの本体の要否はT5-1の調査で決める。設定の切り替えは、変更前の値を`restore_state.json`に保存してから行い、終了・切断・VRChatとの切断で戻す。異常終了した場合は、次の起動時に`restore_state.json`が残っていれば戻す
- **android_mirror_source**：同梱のscrcpy-server（4.1に固定）を送り、映像のソケットからH.264・H.265を受けて、Media Foundationのデコーダーで共有テクスチャに出す。版はPCアプリと一緒に上げる
- **phone_orientation_source**：UDPで回転センサー値を受ける。パケットは`{トークン, 連番, 時刻, 四元数}`で、トークンが一致しないものと、連番が古いものは捨てる。受けるのは同じLANかUSB（ADBのポート転送）の経路だけ
- **vrchat_bridge**：`vrchat_osc`でVRChatを見つけ、`rosc`で2つのBoolを送る。表示のオン・オフと持ち手の変更、VRChatの再接続のたびに送り直す
- **app_state**：各モジュールの状態をまとめた`AppSnapshot`を持ち、変化のたびに`state_changed`イベントでUIへ送る。UIからの操作は、Tauriのコマンドで受ける

### Tauriのコマンドとイベント（UIとの境界）

- コマンド：`get_snapshot`、`get_settings`、`update_settings(patch)`、`set_display_enabled(bool)`、`list_capture_devices`、`list_windows`、`start_video(source)`、`adb_pair(host, port, code)`、`adb_connect(target)`、`adb_install_sensor_app`、`sensor_qr_payload`、`start_calibration`、`finish_calibration`、`cancel_calibration`、`complete_setup`、`open_licenses`
- イベント：`state_changed(AppSnapshot)`、`preview_frame(jpeg)`、`calibration_progress`
- UIは、`AppSnapshot`の状態の列挙をそのまま辞書のキーに対応させて表示する（UIで文を組み立てない）

## 3. UI（SvelteKit＋carbon-components-svelte）

- **枠**：UI Shellのサイドナビ（幅48pxのアイコンのレール。ホーム・設定。アイコンは`@carbon/icons-svelte`のHomeとSettings、名前はツールチップと`aria-label`）。ウィンドウは960×680を基準にし、固定幅を避ける
- **テーマ**：Carbonのg100（ダーク）とwhite（ライト）を`data-theme`で切り替え、既定はシステムに合わせる。値は`src/lib/theme/tokens.css`の変数だけを参照する。コンポーネントに色・サイズを直書きしない
- **書体**：IBM Plex Sans・Sans JP・Sans SC・Monoを`static/fonts/`に同梱し、`tokens.css`の`:lang()`で切り替える。ライセンス（OFL）はライセンス表示に含める
- **画面の対応**
  - S01（初回セットアップ）：`routes/setup`。ステップ表示（映像→スマホ→VRChat→位置合わせ）と各ステップのページ。映像のページ（S01a・S01b・S01f・S01g）とS01hは、設定から単独でも開ける（S03aで取り込み方法を変えたとき、S03cの「ロック解除の案内」）
  - S01e（位置合わせ）：セットアップの最後のステップ。ホームの「位置を合わせ直す」からも単独で開ける
  - S02（ホーム）：表示のスイッチ、5つの状態の行、映像のプレビュー、位置を合わせ直す
  - S03（設定）：タブ（表示・位置・入力・VRChat・一般）。項目の振り分けはスペック9a節（表示：パネルの大きさ・明るさ・取り込み方法・切り抜き、位置：位置の基準・側、入力：`input.*`、VRChat：`vrchat.*`、一般：`general.*`）。表示・位置・入力・VRChatは下書きを持ち、保存で変えた項目だけを`update_settings`に送る。一般は変えた時点で保存して適用する。S03cの末尾にライセンスのリンク（スペック9a節）
  - S04（VR内の表示）：PCの画面ではなくオーバーレイの見え方。状態のチップ（追跡中・センサー途絶・追跡が切れた・画面が消えた）の文言は辞書から引き、オーバーレイに描く（S04dは初版のスコープ外）
- **開発用の状態の見本**：疑似ロケールと同じく開発時だけ有効な切り替えで、`AppSnapshot`の見本（S02a・S02b・S02c）をUIに流し、機器なしで状態の表示を確かめられるようにする。公開版には含めない

## 4. 国際化

- **PC**：Paraglide JS 2系。辞書は`messages/{ja,en,zh-Hans}.json`で、原文はja。モック5ファイルの辞書から`mock.*`以外のキーを移す。キーはモックのキー（機能ごとの階層をピリオドで区切り、各階層はスネークケース）をそのまま使う。Paraglideでのキーの扱いとAPIは、インストールした版の公式ドキュメントで確かめる
- **ロケール**：設定の`general.language`が`auto`ならOSのロケールから選び、ja・en・zh-Hans以外はjaへフォールバックする。`<html lang>`を更新する。言語は3つあるので、切り替えUI（S03c）を出す
- **疑似ロケール**：辞書から生成し（3〜4割伸ばし、記号で囲む）、開発時だけ選べるようにする
- **Android**：string resources（`values`＝ja、`values-en`、`values-b+zh+Hans`）。リソース名はピリオドを使えないため、`mobile.idle_title`→`mobile_idle_title`のように置き換え、対応表を検査スクリプトに持たせる。疑似ロケールはAndroid標準（`en-XA`）を開発ビルドで有効にする
- **オーバーレイの文言**（S04のチップ）：PCの辞書から引いて描く

## 5. トークン

- `roadmap/specs/phone_in_hand_tokens_v2.css`を`src/lib/theme/tokens.css`へコピーする。checkコマンドで両者が一致することを確かめる
- Android：`scripts/gen_android_tokens.ts`で`tokens.css`から`android/.../theme/Tokens.kt`（Composeの色・寸法）を生成する。checkコマンドで、生成物が最新であることを確かめる
- `brand.md`と`icon_master.svg`は`docs/brand/`へ版番号なしでコピーする

## 6. checkコマンド

`bun run check`の1つで次を実行し、失敗した部分だけを表示する。CIも同じコマンドを呼ぶ。

- `svelte-check`（型）、ESLint
- `cargo fmt --check`、`cargo clippy --workspace -- -D warnings`、`cargo test --workspace`
- i18n：ロケール間のキーの過不足とプレースホルダの一致、未使用キー、UIの文字列の直書きの検出（キーの型検査はParaglideの型で行う）
- トークン：`specs`と`src/lib/theme/tokens.css`、`docs/brand/`の一致。`src/`（tokens.cssを除く）の色・サイズの直書きの検出
- Androidの`android/`ができてから（Phase 7）：Gradleの`lint`・単体テスト、Android用トークンの生成物が最新であること、string resourcesのキーの過不足とプレースホルダ。Android SDKがない環境では、その旨を表示して失敗にする

## 7. データモデル

PCのアプリデータのフォルダ（Tauriの`app_data_dir`）に、次のファイルだけを置く。外部へは何も送らない。

- `settings.json`：設定（スキーマの版つき）
  - `setup_completed`、`phone_kind`（android・iphone）
  - `video`：`source`（hdmi・mirror・window）、`hdmi`（デバイスID、幅、高さ、fps）、`mirror`（つなぎ方 usb・wifi、形式 auto・h264・h265、端末のシリアル）、`window`（ウィンドウのタイトルと実行ファイル名）、`crop`（auto・manual と手動の矩形）
  - `pose`：`source`（controller・hand）、`side`（left・right）
  - `display`：`enabled`、`size_inch`、`brightness`
  - `input`：`show_touches`、`keep_awake`
  - `vrchat`：`send_port`、`receive_port`、`param_visible`、`param_is_left`
  - `general`：`language`（auto・ja・en・zh-Hans）、`theme`（system・light・dark）、`start_with_steamvr`、`start_minimized`
  - `sensor`：接続トークン、待ち受けポート
  - `adb_devices`：ペア設定した端末（シリアル、名前、最後のアドレス）
- `calibration.json`：位置合わせの結果（基準、側、位置のずれ、回転のずれ、日時）
- `adbkey`・`adbkey.pub`：ADBの鍵
- `restore_state.json`：ADBで変える前の端末の設定値（変更中だけ存在する）

### 動きで見るデータの変化（架空の利用者Aさん）

1. **初めて起動する**：ファイルは何もない。既定値で起動し、`setup_completed`がfalseなのでS01を開く。まだ何も保存しない
2. **S01aでHDMIキャプチャを選び、デバイスと解像度を選ぶ**：メモリ上の設定だけが変わる。プレビューは流れるが、保存はしない
3. **S01cでワイヤレスデバッグのペア設定をする**：`adbkey`・`adbkey.pub`が作られ（初回だけ）、`settings.json`の`adb_devices`に端末が1件増える（ペア設定は端末側にも記録されるため、ここで保存する）
4. **S01cでセンサー送信アプリを入れる（ハンドトラッキングを使う場合）**：`sensor`の接続トークンが作られて保存される。APKのインストールは端末側だけが変わる
5. **S01dで「あとで設定する」を押す**：何も変わらない
6. **S01eで位置合わせを終える**：`calibration.json`が作られる。続いてセットアップが終わり、`settings.json`に、ここまでの設定と`setup_completed=true`がまとめて書かれる
7. **ホームで「VRに表示」をオンにする**：`display.enabled=true`が保存される。Androidでワイヤレスデバッグがあれば、端末の設定を変える前に`restore_state.json`が作られ、変えた後もそのまま残る
8. **VRChatとの接続が切れる、またはアプリを終了する**：端末の設定を`restore_state.json`の値に戻し、`restore_state.json`を消す
9. **設定で明るさを変えて保存する**：`settings.json`の`display.brightness`だけが変わる。保存を押さずに離れた場合は何も変わらない
10. **ホームの「位置を合わせ直す」で、もう一度位置合わせをする**：`calibration.json`が上書きされる。途中でやめた場合は、前の結果が残る
11. **プレビューを見るだけ、状態を見るだけ**：何も変わらない

### 画面の項目とデータの対応

- S01a・S01b：`video.source=hdmi`、`video.hdmi.*`、`video.crop`
- S01f：`video.source=mirror`、`video.mirror.*`
- S01g：`video.source=window`、`video.window`
- S01c：`adb_devices`、`sensor`、`phone_kind`（iPhoneを使うボタン）
- S01d：保存するものはない（OSCQueryの結果は`AppSnapshot`だけ）
- S01e：`pose.source`、`pose.side`、`calibration.json`
- S01h：保存するものはない（案内だけ）
- S02：`display.enabled`。状態の行とプレビューは`AppSnapshot`だけ
- S03a：`display.size_inch`、`display.brightness`、`video.source`、`video.crop`、`pose.source`、`pose.side`、`input.*`
- S03b：`vrchat.*`
- S03c：`general.*`

### 削除・修正・二重操作

- **取り込み方法を変える**：`video.source`だけが変わり、ほかの方法の設定（`hdmi`・`mirror`・`window`）は残る。戻したときに再利用する
- **同じ端末をもう一度ペア設定する**：`adb_devices`の同じシリアルの行を上書きする（2件にしない）
- **位置合わせのやり直し**：`calibration.json`を上書きする。履歴は持たない。基準や側を変えた場合は、前の結果は使えないことをホームの状態で示し、合わせ直しを促す
- **保存を2回押す**：同じ内容を書くだけで、結果は変わらない。書き込みは一時ファイルに書いてから置き換え、途中で落ちても壊れない
- **異常終了**：`restore_state.json`が残っていれば、次に端末とつながったときに戻してから消す
- **アプリのアンインストール**：アプリデータのフォルダは残す（インストーラーの既定に従う）。READMEに場所を書く
- **センサーのトークンを作り直す**：古いトークンのパケットは捨てられるため、端末側でQRコードを読み直すか、ADBで再送する

## 8. Androidのセンサー送信アプリ（Phase 7）

- パッケージ名`com.rinontech.phoneinhand`。UI（S05）、接続設定、センサー送信サービスの3つに分ける
- `TYPE_GAME_ROTATION_VECTOR`を60Hz目安で読み、UDPで送る。フォアグラウンドサービスの種類は`connectedDevice`を第一候補とし、前提条件を実装時に公式ドキュメントで確かめる
- 接続情報は、ADBで受け取る（`adb_controller.send_sensor_config`）か、QRコード・IPアドレスの入力で受け取る。端末の`SharedPreferences`に保存する
- テーマは生成した`Tokens.kt`だけを参照し、見た目をS05のモックに合わせる
- 署名の鍵はユーザーが作って保管し、CIではSecretsから使う

## 9. 配布（Phase 9）

- PCアプリ：tauri-actionでWindowsのインストーラーを作る。署名しない
- Androidアプリ：署名済みAPKを作り、PCアプリの`resources/`に同梱し、Releasesにも置く。Obtainiumは、ReleasesのAPKをそのまま読めるようにする
- NOTICEとアプリ内のライセンス表示に、scrcpy-server（Apache-2.0）、IBM Plex（OFL）、ほか同梱物を含める。GPLのソフトは同梱しない
- アバター用パッケージは初版のスコープ外

## 10. 検証の環境についての注意

- 実機の検証で使えるのは、Android、HDMIキャプチャ機器、iPhone（ユーザーの手元の機材）
- ハンドトラッキングの環境（Steam Link・Virtual Desktop）はない。ハンドトラッキングの検証は、SteamVRがコントローラーから出す手の骨格（Skeletal Input）を手の代わりに使い、Androidの実機とつないで行う。本物のハンドトラッキングでの確認は行わない（スペックで「試験的」として既知の制限に挙げている）
