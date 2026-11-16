//! phone_in_hand のTauriに依存しないロジック。
//!
//! 型、モジュールの間をつなぐトレイト（スペック5章、imple 2章）、判定のロジック、設定のモデルを置く。
//! OS・機器に依存する実装は `src-tauri` に置く。各トレイトのテスト用の偽の実装は [`fake`] にある。

pub mod adb;
pub mod error;
pub mod fake;
pub mod frame;
pub mod geometry;
pub mod overlay;
pub mod pose;
pub mod settings;
pub mod settings_store;
pub mod snapshot;
pub mod video;
pub mod vrchat;

pub use error::{Error, Result};
