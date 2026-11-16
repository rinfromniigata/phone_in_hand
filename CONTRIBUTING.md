# phone_in_hand の開発

不具合の報告はIssueで受け付けます。プルリクエストは、事前にIssueで相談してください。

利用者向けの説明は`README.md`にあります。この文書は、開発・ビルドのための情報です。

## 開発環境

PCアプリはWindows x64だけを対象にしています（SteamVRとPC版VRChatの利用環境のため）。

- Windows 11（x64）
- [Bun](https://bun.sh/) 1.4以上
- Node.js 22.17以上（SvelteKitとViteの実行に使う）
- [Rust](https://www.rust-lang.org/tools/install)（stable。版と部品は`rust-toolchain.toml`で指定）
- Visual Studio Build Tools（「C++によるデスクトップ開発」。MSVCのリンカーとWindows SDK）
- Microsoft Edge WebView2 Runtime（Windows 11には入っている）

Tauriの前提条件の詳細は、[Tauriの公式ドキュメント](https://v2.tauri.app/start/prerequisites/)を確かめてください。

## セットアップ

```powershell
git clone https://github.com/rinfromniigata/phone_in_hand.git
cd phone_in_hand
bun install
```

初回の`bun run dev`・`bun run build`・`bun run check`で、次のものが自動で用意されます。

- 書体（IBM Plex）：`node_modules/@ibm/plex-*`から`static/fonts/`へコピーされる（`scripts/copy_fonts.ts`）。アプリは実行時に外部から書体を読まない
- 辞書のコンパイル：Paraglideが`messages/`から`src/lib/paraglide/`を作る。初回だけ、inlangの辞書形式のプラグインをネットワークから取得する（ビルド時のみ。アプリの実行時ではない）

## 起動

```powershell
# アプリとして起動する（Viteの開発サーバーとTauriのウィンドウ）
bun run tauri dev

# UIだけをブラウザで開く（http://localhost:5173。機器を使う機能は「使用できません」になる）
bun run dev
```

開発サーバーで起動したときだけ、画面の右下に「DEV」の切り替えが出ます。

- **State sample**：ホームの状態の見本（S02a・S02b・S02c）を流し、機器なしで状態の表示を確かめる
- **Locale**：表示の言語を、設定によらず切り替える。疑似ロケール`en-XA`（英語を約4割伸ばし、［］で囲んだもの）で、直書きの取りこぼしと長さによる崩れを確かめる

どちらも公開版のビルドには含まれません。

## 確認とテスト

```powershell
bun run check
```

次をまとめて実行し、通った項目は1行だけ、失敗した項目はその出力を表示します。CIも同じコマンドを呼びます。

- 型（svelte-check）、ESLint、UIとスクリプトのテスト（`bun test`）
- i18nの自動検査（`scripts/check_i18n.ts`）：ロケール間のキーの過不足とプレースホルダの一致、未使用キー、`.svelte`の文字列の直書き
- トークンの検査（`scripts/check_tokens.ts`）：`roadmap/specs/`の最新版と、`src/lib/theme/tokens.css`・`docs/brand/`の一致。`src/`のスタイルの色・サイズの直書き
- `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`

### 守ること

- **値**：色・サイズ・形の正本は`src/lib/theme/tokens.css`（`roadmap/specs/`のトークンのコピー）。部品には`var(--cds-*)`・`var(--pih-*)`で参照し、値を直書きしない。Carbonの部品のCSSは`@layer carbon`に入れて読み込んでいるため、`tokens.css`の値が優先される（`scripts/vite_carbon_css.ts`）
- **文言**：UIの文字列は`messages/{ja,en,zh-Hans}.json`に置き、`m['nav.home']()`のように文字列の定数のキーで引く。原文はja。文字列を連結して文を組み立てず、可変部分はプレースホルダ（`{name}`）にする
- **未実装の画面のキー**：`scripts/i18n_pending.json`に画面IDごとに載せ、未使用キーの検査から外している。画面を実装してキーを使ったら、そこから消す（残っていると検査が指摘する）。`mobile.*`はAndroidのセンサー送信アプリ用
- **設定の既定値**：正本はRustの`crates/pih_core/src/settings.rs`。UIがTauriの外で使う`src/lib/state/default_settings.json`は、Rustのテストで一致を検査する。既定値を変えたら次で書き直す

  ```powershell
  $env:UPDATE_FIXTURES = '1'; cargo test -p pih_core; Remove-Item Env:UPDATE_FIXTURES
  ```

## データの保存場所

アプリのデータは`%APPDATA%\com.rinontech.phone-in-hand\`に置かれます（`settings.json`、`calibration.json`など）。読めないファイルは`*.broken`に名前を変えて残し、既定値で起動します。

## リポジトリ構成

```
phone_in_hand/
├─ src/                    SvelteKit（静的出力）のUI
│  ├─ lib/theme/           tokens.css（specsのコピー）、Carbonのテーマの切り替え、書体
│  ├─ lib/i18n/            ロケールの判定・保持・フォールバック
│  ├─ lib/state/           Rust側の状態と設定を受け持つストア、Tauriのコマンドの呼び出し
│  ├─ lib/components/      画面をまたいで使う部品
│  ├─ lib/dev/             開発時だけの切り替え（公開版には含まれない）
│  └─ routes/              画面（home、settings など）
├─ messages/               辞書（ja・en・zh-Hans。jaが原文）
├─ project.inlang/         Paraglideのプロジェクトの設定
├─ crates/pih_core/        Tauriに依存しないロジック（型、トレイト、設定のモデルと保存）。単体テストの中心
├─ src-tauri/              Tauriのアプリと、OS・機器に依存する実装
├─ scripts/                checkコマンドの部品、書体のコピー、疑似ロケールの生成
├─ docs/brand/             brand.md、icon_master.svg（roadmap/specs/ のコピー。版番号なし）
└─ roadmap/                計画と記録（下記）
```

### roadmap/

企画の成果物と、実装の計画・記録を置きます。アプリのビルドには使いません。

- `specs/`：プランアセットの最新版（スペックシート、トークン、ブランド、アイコンのマスター、画面ごとのモック）。振る舞い・値・画面構成の正本で、モックは参照専用
- `planning_dialog/`：実装設計（imple）、実装タスク（tasks）、検証（walk）の最新版
- `development/`：修正記録と仕様変更のサマリー
- `timeline.md`：プランアセットの版とリリースの時系列
- `archived/`：旧版（Gitの管理外）

### docs/brand/

ブランドの方針（`brand.md`）とアイコンのマスター（`icon_master.svg`）です。`roadmap/specs/`の最新版を版番号なしでコピーしたもので、checkコマンドが一致を確かめます。直すときは`roadmap/specs/`の側を新しい版にしてからコピーします。
