//! 設定と位置合わせの結果の保存（`settings_store`）
//!
//! - アプリデータのフォルダに、`settings.json`・`calibration.json`・`restore_state.json` を置く
//! - ファイルがなければ既定値（位置合わせと復元の記録はなし）
//! - 書き込みは一時ファイルに書いてから置き換え、途中で落ちても壊れない
//! - 読めないファイルは `*.broken` に名前を変えて残し、既定値で続ける（[`Stored::recovered`]）
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::settings::{Calibration, RestoreState, SETTINGS_SCHEMA_VERSION, Settings, Stored};
use crate::{Error, Result};

pub const SETTINGS_FILE: &str = "settings.json";
pub const CALIBRATION_FILE: &str = "calibration.json";
pub const RESTORE_STATE_FILE: &str = "restore_state.json";

pub trait SettingsStore: Send {
    /// ないときは既定値
    fn load(&self) -> Result<Stored>;
    fn save_settings(&self, s: &Settings) -> Result<()>;
    fn save_calibration(&self, c: &Calibration) -> Result<()>;
    /// `None` なら記録を消す
    fn save_restore_state(&self, r: Option<&RestoreState>) -> Result<()>;
}

/// JSONのファイルに保存する実装
#[derive(Debug, Clone)]
pub struct JsonSettingsStore {
    dir: PathBuf,
}

impl JsonSettingsStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        JsonSettingsStore { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// 読めなければ `*.broken` に退避して `None` を返し、`recovered` に名前を加える
    fn read_or_recover<T: DeserializeOwned>(
        &self,
        name: &'static str,
        recovered: &mut Vec<&'static str>,
        parse: impl FnOnce(&str) -> Result<T>,
    ) -> Result<Option<T>> {
        let path = self.path(name);
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        match parse(&text) {
            Ok(v) => Ok(Some(v)),
            Err(_) => {
                fs::rename(&path, self.path(&format!("{name}.broken")))?;
                recovered.push(name);
                Ok(None)
            }
        }
    }

    fn write_atomic<T: Serialize>(&self, name: &str, value: &T) -> Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.path(name);
        let tmp = self.path(&format!("{name}.tmp"));
        {
            let mut f = fs::File::create(&tmp)?;
            serde_json::to_writer_pretty(&mut f, value)?;
            f.write_all(b"\n")?;
            f.sync_all()?;
        }
        // Windowsでも既存のファイルを置き換える（MoveFileExW の MOVEFILE_REPLACE_EXISTING）
        fs::rename(&tmp, &path)?;
        Ok(())
    }
}

/// `settings.json` を読む。スキーマの版が新しすぎるものは読まない
fn parse_settings(text: &str) -> Result<Settings> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if version > u64::from(SETTINGS_SCHEMA_VERSION) {
        return Err(Error::Corrupt(format!("schema_version {version}")));
    }
    // 版1より前はない。版を上げたら、ここで古い形から移行する
    let mut settings: Settings = serde_json::from_value(value)?;
    settings.schema_version = SETTINGS_SCHEMA_VERSION;
    settings.validate()?;
    Ok(settings)
}

impl SettingsStore for JsonSettingsStore {
    fn load(&self) -> Result<Stored> {
        let mut recovered = Vec::new();
        let settings = self
            .read_or_recover(SETTINGS_FILE, &mut recovered, parse_settings)?
            .unwrap_or_default();
        let calibration = self.read_or_recover(CALIBRATION_FILE, &mut recovered, |t| {
            Ok(serde_json::from_str::<Calibration>(t)?)
        })?;
        let restore_state = self.read_or_recover(RESTORE_STATE_FILE, &mut recovered, |t| {
            Ok(serde_json::from_str::<RestoreState>(t)?)
        })?;
        Ok(Stored {
            settings,
            calibration,
            restore_state,
            recovered,
        })
    }

    fn save_settings(&self, s: &Settings) -> Result<()> {
        s.validate()?;
        self.write_atomic(SETTINGS_FILE, s)
    }

    fn save_calibration(&self, c: &Calibration) -> Result<()> {
        self.write_atomic(CALIBRATION_FILE, c)
    }

    fn save_restore_state(&self, r: Option<&RestoreState>) -> Result<()> {
        match r {
            Some(r) => self.write_atomic(RESTORE_STATE_FILE, r),
            None => match fs::remove_file(self.path(RESTORE_STATE_FILE)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
                _ => Ok(()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Pose, Side};
    use crate::settings::PoseSourceKind;

    fn store() -> (tempfile::TempDir, JsonSettingsStore) {
        let dir = tempfile::tempdir().unwrap();
        let s = JsonSettingsStore::new(dir.path().join("app"));
        (dir, s)
    }

    fn calibration() -> Calibration {
        Calibration {
            source: PoseSourceKind::Controller,
            side: Side::Left,
            offset: Pose::default(),
            calibrated_at_ms: 1_800_000_000_000,
        }
    }

    #[test]
    fn empty_folder_gives_defaults_without_writing() {
        let (_d, s) = store();
        let stored = s.load().unwrap();
        assert_eq!(stored, Stored::default());
        assert!(!s.dir().exists());
    }

    #[test]
    fn settings_round_trip() {
        let (_d, s) = store();
        let mut settings = Settings {
            setup_completed: true,
            ..Settings::default()
        };
        settings.display.brightness = 0.3;
        s.save_settings(&settings).unwrap();
        assert_eq!(s.load().unwrap().settings, settings);
    }

    #[test]
    fn saving_twice_gives_same_result_and_leaves_no_temp_file() {
        let (_d, s) = store();
        let settings = Settings::default();
        s.save_settings(&settings).unwrap();
        let first = fs::read(s.dir().join(SETTINGS_FILE)).unwrap();
        s.save_settings(&settings).unwrap();
        let second = fs::read(s.dir().join(SETTINGS_FILE)).unwrap();
        assert_eq!(first, second);
        assert!(!s.dir().join("settings.json.tmp").exists());
    }

    #[test]
    fn invalid_settings_are_not_saved() {
        let (_d, s) = store();
        let mut settings = Settings::default();
        settings.display.brightness = 5.0;
        assert!(s.save_settings(&settings).is_err());
        assert!(!s.dir().join(SETTINGS_FILE).exists());
    }

    #[test]
    fn calibration_is_overwritten() {
        let (_d, s) = store();
        s.save_calibration(&calibration()).unwrap();
        let mut next = calibration();
        next.side = Side::Right;
        s.save_calibration(&next).unwrap();
        assert_eq!(s.load().unwrap().calibration, Some(next));
    }

    #[test]
    fn restore_state_exists_only_while_changed() {
        let (_d, s) = store();
        let r = RestoreState {
            serial: "abc".into(),
            show_touches: Some("0".into()),
            stay_on_while_plugged_in: None,
        };
        s.save_restore_state(Some(&r)).unwrap();
        assert_eq!(s.load().unwrap().restore_state, Some(r));
        s.save_restore_state(None).unwrap();
        assert_eq!(s.load().unwrap().restore_state, None);
        // ないものを消しても失敗しない
        s.save_restore_state(None).unwrap();
    }

    #[test]
    fn broken_file_is_moved_aside_and_defaults_used() {
        let (_d, s) = store();
        fs::create_dir_all(s.dir()).unwrap();
        fs::write(s.dir().join(SETTINGS_FILE), "{ not json").unwrap();
        s.save_calibration(&calibration()).unwrap();
        let stored = s.load().unwrap();
        assert_eq!(stored.settings, Settings::default());
        assert_eq!(stored.calibration, Some(calibration()));
        assert_eq!(stored.recovered, vec![SETTINGS_FILE]);
        assert!(s.dir().join("settings.json.broken").exists());
        assert!(!s.dir().join(SETTINGS_FILE).exists());
    }

    #[test]
    fn newer_schema_is_not_read() {
        let (_d, s) = store();
        fs::create_dir_all(s.dir()).unwrap();
        let newer = format!(r#"{{ "schema_version": {} }}"#, SETTINGS_SCHEMA_VERSION + 1);
        fs::write(s.dir().join(SETTINGS_FILE), newer).unwrap();
        let stored = s.load().unwrap();
        assert_eq!(stored.settings, Settings::default());
        assert_eq!(stored.recovered, vec![SETTINGS_FILE]);
    }

    #[test]
    fn unknown_and_missing_fields_are_tolerated() {
        let (_d, s) = store();
        fs::create_dir_all(s.dir()).unwrap();
        let text = r#"{ "schema_version": 1, "setup_completed": true, "unknown": 1 }"#;
        fs::write(s.dir().join(SETTINGS_FILE), text).unwrap();
        let stored = s.load().unwrap();
        assert!(stored.settings.setup_completed);
        assert!(stored.recovered.is_empty());
    }
}
