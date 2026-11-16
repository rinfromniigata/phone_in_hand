//! テスト用と、未実装のモジュールの代わりに使う偽の実装。
//!
//! 機器を使うモジュールは、実装するまで「使用できません」（[`Error::Unavailable`]）を返す。
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::adb::{
    AdbController, AdbTarget, DeviceInfo, MirrorOptions, MirrorStream, SensorConfig,
    SessionSettings,
};
use crate::frame::{FrameInfo, FrameProcessor};
use crate::geometry::{Rect, Side};
use crate::overlay::OverlayPresenter;
use crate::pose::{
    OrientationSample, PanelState, PhoneOrientationSource, PoseSample, PoseSource, TrackingState,
};
use crate::settings::{Calibration, RestoreState, Settings, Stored};
use crate::settings_store::SettingsStore;
use crate::video::{Frame, VideoConfig, VideoSource, VideoSourceKind, VideoStatus};
use crate::vrchat::{VrchatBridge, VrchatStatus};
use crate::{Error, Result};

/// 映像を返さない取り込み
#[derive(Debug, Default)]
pub struct FakeVideoSource {
    pub kind: VideoSourceKind,
}

impl VideoSource for FakeVideoSource {
    fn kind(&self) -> VideoSourceKind {
        self.kind
    }
    fn start(&mut self, _config: &VideoConfig) -> Result<()> {
        Err(Error::Unavailable)
    }
    fn next_frame(&mut self, _timeout: Duration) -> Result<Option<Frame>> {
        Err(Error::Unavailable)
    }
    fn stop(&mut self) {}
    fn status(&self) -> VideoStatus {
        VideoStatus::Unavailable
    }
}

/// 切り抜かず、画面は点いているとみなす
#[derive(Debug, Default)]
pub struct FakeFrameProcessor;

impl FrameProcessor for FakeFrameProcessor {
    fn analyze(&mut self, frame: &Frame, _now: Instant) -> FrameInfo {
        FrameInfo {
            crop: Rect::full(frame.size()),
            screen_off: false,
        }
    }
}

/// 決まった姿勢を返す（`None` なら追跡が切れている）
#[derive(Debug, Default)]
pub struct FakePoseSource {
    pub sample: Option<PoseSample>,
}

impl PoseSource for FakePoseSource {
    fn sample(&mut self, _side: Side) -> PoseSample {
        self.sample.unwrap_or(PoseSample {
            pose: None,
            state: TrackingState::Lost,
        })
    }
}

/// 決まったセンサーの値を返す
#[derive(Debug, Default)]
pub struct FakePhoneOrientationSource {
    pub latest: Option<OrientationSample>,
}

impl PhoneOrientationSource for FakePhoneOrientationSource {
    fn latest(&self) -> Option<OrientationSample> {
        self.latest
    }
}

/// 受け取ったものを記録するだけの表示
#[derive(Debug, Default)]
pub struct FakeOverlayPresenter {
    pub enabled: bool,
    pub last_state: Option<PanelState>,
    pub appearance: Option<(f32, f32)>,
    pub frames: usize,
}

impl OverlayPresenter for FakeOverlayPresenter {
    fn set_frame(&mut self, _frame: &Frame, _crop: Rect) -> Result<()> {
        self.frames += 1;
        Ok(())
    }
    fn set_state(&mut self, state: &PanelState) -> Result<()> {
        self.last_state = Some(*state);
        Ok(())
    }
    fn set_appearance(&mut self, size_inch: f32, brightness: f32) -> Result<()> {
        self.appearance = Some((size_inch, brightness));
        Ok(())
    }
    fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        self.enabled = enabled;
        Ok(())
    }
}

/// どの操作も「使用できません」を返すADB
#[derive(Debug, Default)]
pub struct FakeAdbController;

impl AdbController for FakeAdbController {
    fn pair(&mut self, _host: &str, _port: u16, _code: &str) -> Result<()> {
        Err(Error::Unavailable)
    }
    fn connect(&mut self, _target: &AdbTarget) -> Result<DeviceInfo> {
        Err(Error::Unavailable)
    }
    fn install(&mut self, _apk: &Path) -> Result<()> {
        Err(Error::Unavailable)
    }
    fn apply_session_settings(&mut self, _want: SessionSettings) -> Result<()> {
        Err(Error::Unavailable)
    }
    fn restore(&mut self) -> Result<()> {
        Err(Error::Unavailable)
    }
    fn start_mirror(&mut self, _opts: &MirrorOptions) -> Result<MirrorStream> {
        Err(Error::Unavailable)
    }
    fn send_sensor_config(&mut self, _cfg: &SensorConfig) -> Result<()> {
        Err(Error::Unavailable)
    }
}

/// VRChatが見つからないとするOSC
#[derive(Debug, Default)]
pub struct FakeVrchatBridge {
    pub status: VrchatStatus,
    pub sent: Vec<(&'static str, bool)>,
}

impl VrchatBridge for FakeVrchatBridge {
    fn send_visible(&mut self, v: bool) -> Result<()> {
        self.sent.push(("visible", v));
        Ok(())
    }
    fn send_is_left(&mut self, v: bool) -> Result<()> {
        self.sent.push(("is_left", v));
        Ok(())
    }
    fn query(&mut self) -> VrchatStatus {
        self.status
    }
}

/// メモリの中だけに保存する
#[derive(Debug, Default)]
pub struct MemorySettingsStore {
    pub stored: Mutex<Stored>,
}

impl MemorySettingsStore {
    fn lock(&self) -> std::sync::MutexGuard<'_, Stored> {
        self.stored
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl SettingsStore for MemorySettingsStore {
    fn load(&self) -> Result<Stored> {
        Ok(self.lock().clone())
    }
    fn save_settings(&self, s: &Settings) -> Result<()> {
        s.validate()?;
        self.lock().settings = s.clone();
        Ok(())
    }
    fn save_calibration(&self, c: &Calibration) -> Result<()> {
        self.lock().calibration = Some(*c);
        Ok(())
    }
    fn save_restore_state(&self, r: Option<&RestoreState>) -> Result<()> {
        self.lock().restore_state = r.cloned();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unimplemented_devices_report_unavailable() {
        let mut v = FakeVideoSource::default();
        assert_eq!(v.status(), VideoStatus::Unavailable);
        assert!(matches!(
            v.next_frame(Duration::ZERO),
            Err(Error::Unavailable)
        ));
        let mut adb = FakeAdbController;
        assert!(matches!(
            adb.pair("192.168.0.2", 37000, "123456"),
            Err(Error::Unavailable)
        ));
    }

    #[test]
    fn memory_store_round_trip() {
        let store = MemorySettingsStore::default();
        let s = Settings {
            setup_completed: true,
            ..Settings::default()
        };
        store.save_settings(&s).unwrap();
        assert_eq!(store.load().unwrap().settings, s);
    }
}
