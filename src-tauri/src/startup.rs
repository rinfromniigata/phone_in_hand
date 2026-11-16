//! 起動の設定（T3-9。S03c）：SteamVRと一緒に起動する。
//!
//! SteamVRのアプリのマニフェスト（`.vrmanifest`）をアプリデータのフォルダに書き、SteamVRに登録して
//! 自動起動を有効にする。設定を切ったら、自動起動を切って登録を解除する。SteamVRとつながっていない間に
//! 設定を変えた場合は、次につながったときに合わせる。
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::openvr::{OpenVr, VrError};

/// SteamVRに登録するアプリのキー（アプリの識別子）
pub const APP_KEY: &str = "com.rinontech.phone-in-hand";
const MANIFEST_FILE: &str = "phone_in_hand.vrmanifest";

/// 辞書のアプリ名（SteamVRの一覧に出る名前）。辞書が正本で、ビルドのときに読み込む
fn app_name(messages: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(messages).ok()?;
    v.get("app.name")?.as_str().map(str::to_owned)
}

/// マニフェストの中身
pub fn manifest_json(exe: &Path) -> serde_json::Value {
    let names = [
        ("en_us", include_str!("../../messages/en.json")),
        ("ja_jp", include_str!("../../messages/ja.json")),
        ("zh_cn", include_str!("../../messages/zh-Hans.json")),
    ];
    let mut strings = serde_json::Map::new();
    for (lang, messages) in names {
        if let Some(name) = app_name(messages) {
            strings.insert(lang.to_owned(), json!({ "name": name }));
        }
    }
    json!({
        "source": "builtin",
        "applications": [{
            "app_key": APP_KEY,
            "launch_type": "binary",
            "binary_path_windows": exe.to_string_lossy(),
            "is_dashboard_overlay": true,
            "strings": strings,
        }]
    })
}

/// マニフェストをアプリデータのフォルダに書く（起動のたびに、今の実行ファイルの場所で書き直す）
pub fn write_manifest(dir: &Path, exe: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(MANIFEST_FILE);
    let body = serde_json::to_string_pretty(&manifest_json(exe)).map_err(std::io::Error::other)?;
    let tmp = dir.join(format!("{MANIFEST_FILE}.tmp"));
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

/// SteamVRへの登録を設定に合わせる。つながっていなければ何もしない（次につながったときに呼ぶ）
pub fn sync(vr: &OpenVr, manifest: &Path, start_with_steamvr: bool) -> Result<(), VrError> {
    if vr.generation().is_none() {
        return Err(VrError::NotConnected);
    }
    if start_with_steamvr {
        // 実行ファイルの場所が変わっていても、登録し直せば新しい場所になる
        vr.add_app_manifest(manifest)?;
        if !vr.auto_launch(APP_KEY)? {
            vr.set_auto_launch(APP_KEY, true)?;
        }
    } else if vr.app_installed(APP_KEY)? {
        vr.set_auto_launch(APP_KEY, false)?;
        vr.remove_app_manifest(manifest)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_points_to_the_executable_and_uses_dictionary_names() {
        let exe = Path::new("C:/Program Files/phone_in_hand/phone_in_hand.exe");
        let v = manifest_json(exe);
        let app = &v["applications"][0];
        assert_eq!(app["app_key"], APP_KEY);
        assert_eq!(app["launch_type"], "binary");
        assert_eq!(app["is_dashboard_overlay"], true);
        assert!(
            app["binary_path_windows"]
                .as_str()
                .unwrap()
                .ends_with("phone_in_hand.exe")
        );
        let ja: serde_json::Value =
            serde_json::from_str(include_str!("../../messages/ja.json")).unwrap();
        assert_eq!(app["strings"]["ja_jp"]["name"], ja["app.name"]);
        assert!(app["strings"]["en_us"]["name"].is_string());
    }

    #[test]
    fn manifest_is_written_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_manifest(dir.path(), Path::new("C:/x/phone_in_hand.exe")).unwrap();
        let read: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(read["applications"][0]["app_key"], APP_KEY);
        assert!(!dir.path().join(format!("{MANIFEST_FILE}.tmp")).exists());
    }

    #[test]
    fn sync_without_steamvr_does_nothing() {
        let vr = OpenVr::new("Z:/no/openvr_api.dll".into());
        assert_eq!(
            sync(&vr, Path::new("x.vrmanifest"), true),
            Err(VrError::NotConnected)
        );
    }
}
