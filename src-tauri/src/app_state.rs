//! `app_state`：各モジュールの状態をまとめて `AppSnapshot` を持ち、変化のたびに通知する。
//! UIからの操作は、Tauriのコマンド（`commands.rs`）がここへ渡す。
//!
//! 通知の先は関数で受け取り、Tauriに依存せずにテストできるようにする。
use std::sync::Mutex;

use pih_core::adb::{AdbController, AdbTarget, DeviceInfo};
use pih_core::fake::{FakeAdbController, FakeOverlayPresenter, FakeVideoSource, FakeVrchatBridge};
use pih_core::overlay::OverlayPresenter;
use pih_core::settings::{Calibration, Settings, Stored};
use pih_core::settings_store::SettingsStore;
use pih_core::snapshot::AppSnapshot;
use pih_core::video::{CaptureDevice, VideoSource, VideoSourceKind, WindowInfo};
use pih_core::vrchat::VrchatBridge;
use pih_core::{Error, Result};

type Notify = Box<dyn Fn(&AppSnapshot) + Send + Sync>;

/// 各モジュールの実装。未実装のものは偽の実装（「使用できません」を返す）
pub struct Modules {
    pub video: Box<dyn VideoSource>,
    pub overlay: Box<dyn OverlayPresenter>,
    pub adb: Box<dyn AdbController>,
    pub vrchat: Box<dyn VrchatBridge>,
}

impl Default for Modules {
    fn default() -> Self {
        Modules {
            video: Box::new(FakeVideoSource::default()),
            overlay: Box::new(FakeOverlayPresenter::default()),
            adb: Box::new(FakeAdbController),
            vrchat: Box::new(FakeVrchatBridge::default()),
        }
    }
}

struct Inner {
    settings: Settings,
    calibration: Option<Calibration>,
    snapshot: AppSnapshot,
    store: Box<dyn SettingsStore>,
    modules: Modules,
}

pub struct AppState {
    inner: Mutex<Inner>,
    notify: Notify,
}

impl AppState {
    /// 保存したものを読み込んで始める。読めないファイルは退避済みで、既定値で続ける
    pub fn new(store: Box<dyn SettingsStore>, modules: Modules, notify: Notify) -> Result<Self> {
        let Stored {
            settings,
            calibration,
            recovered,
            ..
        } = store.load()?;
        for name in recovered {
            log::warn!("{name} を読めなかったため、{name}.broken に退避して既定値で起動します");
        }
        let snapshot = AppSnapshot::initial(&settings, calibration.as_ref());
        let state = AppState {
            inner: Mutex::new(Inner {
                settings,
                calibration,
                snapshot,
                store,
                modules,
            }),
            notify,
        };
        state.with(|inner| inner.recompute());
        Ok(state)
    }

    fn with<T>(&self, f: impl FnOnce(&mut Inner) -> T) -> T {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let before = inner.snapshot.clone();
        let out = f(&mut inner);
        inner.recompute();
        if inner.snapshot != before {
            (self.notify)(&inner.snapshot);
        }
        out
    }

    pub fn snapshot(&self) -> AppSnapshot {
        self.with(|i| i.snapshot.clone())
    }

    pub fn settings(&self) -> Settings {
        self.with(|i| i.settings.clone())
    }

    /// 部分的な変更を当てて保存する（設定の「保存」）
    pub fn update_settings(&self, patch: &serde_json::Value) -> Result<Settings> {
        self.with(|i| {
            let next = i.settings.patched(patch)?;
            i.store.save_settings(&next)?;
            i.settings = next;
            Ok(i.settings.clone())
        })
    }

    /// ホームの「VRに表示」。保存し、オーバーレイとVRChat（表示中のパラメータ）へ伝える
    pub fn set_display_enabled(&self, enabled: bool) -> Result<AppSnapshot> {
        self.with(|i| {
            let mut next = i.settings.clone();
            next.display.enabled = enabled;
            i.store.save_settings(&next)?;
            i.settings = next;
            if let Err(e) = i.modules.overlay.set_enabled(enabled) {
                log::warn!("オーバーレイの表示を切り替えられません: {e}");
            }
            if let Err(e) = i.modules.vrchat.send_visible(enabled) {
                log::warn!("VRChatへ表示中を送れません: {e}");
            }
            Ok::<_, Error>(())
        })?;
        Ok(self.snapshot())
    }

    /// セットアップの終わり：ここまでの設定と `setup_completed=true` をまとめて保存する
    pub fn complete_setup(&self, patch: &serde_json::Value) -> Result<Settings> {
        self.with(|i| {
            let mut next = i.settings.patched(patch)?;
            next.setup_completed = true;
            i.store.save_settings(&next)?;
            i.settings = next;
            Ok(i.settings.clone())
        })
    }

    pub fn list_capture_devices(&self) -> Result<Vec<CaptureDevice>> {
        Err(Error::Unavailable)
    }

    pub fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        Err(Error::Unavailable)
    }

    /// 取り込みを始める。取り込み方法の実装（T3-2・T4-1・T7-2）がつながるまでは使用できない
    pub fn start_video(&self, _source: VideoSourceKind) -> Result<()> {
        Err(Error::Unavailable)
    }

    pub fn adb_pair(&self, host: &str, port: u16, code: &str) -> Result<()> {
        self.with(|i| i.modules.adb.pair(host, port, code))
    }

    pub fn adb_connect(&self, target: &AdbTarget) -> Result<DeviceInfo> {
        self.with(|i| i.modules.adb.connect(target))
    }
}

impl Inner {
    /// 設定と各モジュールの状態から `AppSnapshot` を作り直す
    fn recompute(&mut self) {
        let mut snap = AppSnapshot::initial(&self.settings, self.calibration.as_ref());
        snap.video = self.modules.video.status();
        self.snapshot = snap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pih_core::fake::MemorySettingsStore;
    use pih_core::snapshot::{AdbStatus, CalibrationStatus};
    use pih_core::video::VideoStatus;
    use serde_json::json;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn state() -> (AppState, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let notify: Notify = Box::new(move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });
        let s = AppState::new(
            Box::new(MemorySettingsStore::default()),
            Modules::default(),
            notify,
        )
        .unwrap();
        (s, count)
    }

    #[test]
    fn starts_with_unavailable_devices() {
        let (s, _) = state();
        let snap = s.snapshot();
        assert_eq!(snap.video, VideoStatus::Unavailable);
        assert_eq!(snap.adb, AdbStatus::Unavailable);
        assert_eq!(snap.calibration, CalibrationStatus::Missing);
        assert!(!s.settings().setup_completed);
    }

    #[test]
    fn display_toggle_saves_and_notifies() {
        let (s, count) = state();
        let snap = s.set_display_enabled(true).unwrap();
        assert!(snap.display_enabled);
        assert!(s.settings().display.enabled);
        assert_eq!(count.load(Ordering::SeqCst), 1);
        // 同じ値をもう一度：状態は変わらず、通知もしない
        s.set_display_enabled(true).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn settings_change_updates_derived_state() {
        let (s, count) = state();
        s.update_settings(&json!({ "phone_kind": "iphone" }))
            .unwrap();
        assert_eq!(s.snapshot().adb, AdbStatus::NotUsed);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn invalid_patch_changes_nothing() {
        let (s, count) = state();
        let before = s.settings();
        assert!(
            s.update_settings(&json!({ "display": { "brightness": -1 } }))
                .is_err()
        );
        assert_eq!(s.settings(), before);
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn complete_setup_saves_everything_at_once() {
        let (s, _) = state();
        let saved = s
            .complete_setup(&json!({ "video": { "source": "window" } }))
            .unwrap();
        assert!(saved.setup_completed);
        assert_eq!(saved.video.source, VideoSourceKind::Window);
    }

    #[test]
    fn unimplemented_commands_report_unavailable() {
        let (s, _) = state();
        assert!(matches!(s.list_windows(), Err(Error::Unavailable)));
        assert!(matches!(
            s.adb_pair("192.168.0.2", 37000, "123456"),
            Err(Error::Unavailable)
        ));
        assert!(matches!(
            s.start_video(VideoSourceKind::Hdmi),
            Err(Error::Unavailable)
        ));
    }
}
