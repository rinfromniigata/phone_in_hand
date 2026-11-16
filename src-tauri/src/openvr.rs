//! OpenVR（SteamVR）との接続（T3-1）。`native/pih_openvr.cpp` の薄い層を安全に呼ぶ。
//!
//! - `openvr_api.dll` は実行時に読み込む。SteamVRがない環境では [`VrError::NotLoaded`] か
//!   [`VrError::Init`] を返すだけで、アプリはそのまま動く
//! - OpenVRの呼び出しは、内部のロックで直列化する。接続と切断、`wait_frame_sync` は、
//!   姿勢のスレッド（`runtime.rs`）だけが呼ぶ
//! - 接続し直すと、それまでのオーバーレイのハンドルは使えない。[`OpenVr::generation`] で見分ける
use std::ffi::{CString, c_int, c_void};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pih_core::geometry::{Pose, Side};

mod ffi {
    use std::ffi::{c_char, c_int, c_void};

    unsafe extern "C" {
        pub fn pih_vr_load(path: *const u16) -> c_int;
        pub fn pih_vr_runtime_installed() -> c_int;
        pub fn pih_vr_init() -> c_int;
        pub fn pih_vr_shutdown();
        pub fn pih_vr_poll_quit() -> c_int;
        pub fn pih_vr_wait_frame_sync(timeout_ms: u32) -> c_int;
        pub fn pih_vr_controller_pose(side: c_int, m: *mut f32) -> c_int;

        pub fn pih_overlay_create(
            key: *const c_char,
            name: *const c_char,
            handle: *mut u64,
        ) -> c_int;
        pub fn pih_overlay_set_visible(h: u64, visible: c_int) -> c_int;
        pub fn pih_overlay_set_width(h: u64, meters: f32) -> c_int;
        pub fn pih_overlay_set_alpha(h: u64, alpha: f32) -> c_int;
        pub fn pih_overlay_set_color(h: u64, r: f32, g: f32, b: f32) -> c_int;
        pub fn pih_overlay_set_sort_order(h: u64, order: u32) -> c_int;
        pub fn pih_overlay_set_ignore_alpha(h: u64, on: c_int) -> c_int;
        pub fn pih_overlay_set_transform(h: u64, m: *const f32) -> c_int;
        pub fn pih_overlay_set_texture_d3d11(h: u64, texture: *mut c_void) -> c_int;
        pub fn pih_overlay_set_bounds(h: u64, u0: f32, v0: f32, u1: f32, v1: f32) -> c_int;
        pub fn pih_overlay_set_raw(h: u64, rgba: *mut c_void, width: u32, height: u32) -> c_int;

        pub fn pih_input_init(manifest_path: *const c_char, handles: *mut u64) -> c_int;
        pub fn pih_input_poll(
            handles: *const u64,
            side: c_int,
            grab: *mut c_int,
            confirm: *mut c_int,
        ) -> c_int;

        pub fn pih_apps_is_installed(key: *const c_char) -> c_int;
        pub fn pih_apps_add_manifest(path: *const c_char) -> c_int;
        pub fn pih_apps_remove_manifest(path: *const c_char) -> c_int;
        pub fn pih_apps_set_auto_launch(key: *const c_char, on: c_int) -> c_int;
        pub fn pih_apps_get_auto_launch(key: *const c_char) -> c_int;
    }
}

/// OpenVRの失敗。数値はOpenVRの各エラーの列挙の値（ログにだけ使う）
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VrError {
    #[error("openvr_api.dll を読み込めません")]
    NotLoaded,
    #[error("SteamVRとつながっていません")]
    NotConnected,
    #[error("SteamVRに接続できません（EVRInitError {0}）")]
    Init(i32),
    #[error("オーバーレイの操作に失敗しました（EVROverlayError {0}）")]
    Overlay(i32),
    #[error("入力の操作に失敗しました（EVRInputError {0}）")]
    Input(i32),
    #[error("アプリの登録に失敗しました（EVRApplicationError {0}）")]
    Applications(i32),
    #[error("文字列にNULを含みます")]
    Nul,
}

impl From<VrError> for pih_core::Error {
    fn from(e: VrError) -> Self {
        log::debug!("{e}");
        pih_core::Error::Unavailable
    }
}

/// この層で負の値は、接続していない・読み込めていない
fn check(code: c_int, wrap: fn(i32) -> VrError) -> Result<(), VrError> {
    match code {
        0 => Ok(()),
        -1 => Err(VrError::NotLoaded),
        c if c < 0 => Err(VrError::NotConnected),
        c => Err(wrap(c)),
    }
}

fn cstr(s: &str) -> Result<CString, VrError> {
    CString::new(s).map_err(|_| VrError::Nul)
}

fn path_cstr(p: &Path) -> Result<CString, VrError> {
    cstr(&p.to_string_lossy())
}

/// オーバーレイのハンドル。作ったときの接続の世代を持つ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayHandle {
    pub raw: u64,
    pub generation: u64,
}

#[derive(Debug, Default)]
struct Conn {
    loaded: bool,
    connected: bool,
    generation: u64,
    /// [アクションセット, つかむ, 決定, 左手, 右手]
    input: Option<[u64; 5]>,
}

pub struct OpenVr {
    dll_path: PathBuf,
    conn: Mutex<Conn>,
}

/// 位置合わせで使う、手ごとのボタンの状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HandButtons {
    pub grab: bool,
    pub confirm: bool,
}

impl OpenVr {
    pub fn new(dll_path: PathBuf) -> Self {
        OpenVr {
            dll_path,
            conn: Mutex::new(Conn::default()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Conn> {
        self.conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn load(&self, conn: &mut Conn) -> Result<(), VrError> {
        if conn.loaded {
            return Ok(());
        }
        let wide: Vec<u16> = self
            .dll_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: NUL終端のUTF-16の文字列を渡す
        check(unsafe { ffi::pih_vr_load(wide.as_ptr()) }, VrError::Init)?;
        conn.loaded = true;
        Ok(())
    }

    /// SteamVRが入っているか（読み込めなければ false）
    pub fn runtime_installed(&self) -> bool {
        let mut conn = self.lock();
        // SAFETY: 引数なし。読み込む前は0を返す
        self.load(&mut conn).is_ok() && unsafe { ffi::pih_vr_runtime_installed() } == 1
    }

    /// オーバーレイのアプリとして接続する。接続の世代を返す
    pub fn connect(&self) -> Result<u64, VrError> {
        let mut conn = self.lock();
        if conn.connected {
            return Ok(conn.generation);
        }
        self.load(&mut conn)?;
        // SAFETY: 引数なし。失敗しても状態は変わらない
        check(unsafe { ffi::pih_vr_init() }, VrError::Init)?;
        conn.connected = true;
        conn.generation += 1;
        conn.input = None;
        Ok(conn.generation)
    }

    pub fn disconnect(&self) {
        let mut conn = self.lock();
        if conn.connected {
            // SAFETY: 接続しているときだけ呼ぶ
            unsafe { ffi::pih_vr_shutdown() };
            conn.connected = false;
            conn.input = None;
        }
    }

    /// 接続していればその世代
    pub fn generation(&self) -> Option<u64> {
        let conn = self.lock();
        conn.connected.then_some(conn.generation)
    }

    /// SteamVRが終了を求めたか（求めたら切断する）
    pub fn poll_quit(&self) -> bool {
        let quit = {
            let conn = self.lock();
            // SAFETY: 接続していなければ0を返す
            conn.connected && unsafe { ffi::pih_vr_poll_quit() } == 1
        };
        if quit {
            self.disconnect();
        }
        quit
    }

    /// 次のSteamVRのフレームまで待つ。ロックを持たずに待つため、姿勢のスレッドだけが呼ぶ
    pub fn wait_frame_sync(&self, timeout_ms: u32) -> Result<(), VrError> {
        if self.generation().is_none() {
            return Err(VrError::NotConnected);
        }
        // SAFETY: 接続と切断は同じスレッドからしか行わないため、待つ間に切断されない
        check(
            unsafe { ffi::pih_vr_wait_frame_sync(timeout_ms) },
            VrError::Overlay,
        )
    }

    /// コントローラーの予測の姿勢。追跡が切れている・つながっていなければ `None`
    pub fn controller_pose(&self, side: Side) -> Option<Pose> {
        let conn = self.lock();
        if !conn.connected {
            return None;
        }
        let mut m = [0f32; 12];
        let s = match side {
            Side::Left => 0,
            Side::Right => 1,
        };
        // SAFETY: 12要素の配列を渡す
        let ok = unsafe { ffi::pih_vr_controller_pose(s, m.as_mut_ptr()) } == 1;
        ok.then(|| Pose::from_matrix34(&m))
    }

    // ---- オーバーレイ ----

    pub fn create_overlay(&self, key: &str, name: &str) -> Result<OverlayHandle, VrError> {
        let conn = self.lock();
        if !conn.connected {
            return Err(VrError::NotConnected);
        }
        let (k, n) = (cstr(key)?, cstr(name)?);
        let mut raw = 0u64;
        // SAFETY: NUL終端の文字列と、書き込み先を渡す
        check(
            unsafe { ffi::pih_overlay_create(k.as_ptr(), n.as_ptr(), &mut raw) },
            VrError::Overlay,
        )?;
        Ok(OverlayHandle {
            raw,
            generation: conn.generation,
        })
    }

    /// 接続が同じ世代のときだけ、オーバーレイを操作する
    fn with_overlay(&self, h: OverlayHandle, f: impl FnOnce(u64) -> c_int) -> Result<(), VrError> {
        let conn = self.lock();
        if !conn.connected || conn.generation != h.generation {
            return Err(VrError::NotConnected);
        }
        check(f(h.raw), VrError::Overlay)
    }

    pub fn set_visible(&self, h: OverlayHandle, visible: bool) -> Result<(), VrError> {
        // SAFETY: 同じ世代のハンドル
        self.with_overlay(h, |raw| unsafe {
            ffi::pih_overlay_set_visible(raw, visible as c_int)
        })
    }

    pub fn set_width(&self, h: OverlayHandle, meters: f32) -> Result<(), VrError> {
        // SAFETY: 同じ世代のハンドル
        self.with_overlay(h, |raw| unsafe { ffi::pih_overlay_set_width(raw, meters) })
    }

    pub fn set_alpha(&self, h: OverlayHandle, alpha: f32) -> Result<(), VrError> {
        // SAFETY: 同じ世代のハンドル
        self.with_overlay(h, |raw| unsafe { ffi::pih_overlay_set_alpha(raw, alpha) })
    }

    pub fn set_color(&self, h: OverlayHandle, rgb: [f32; 3]) -> Result<(), VrError> {
        // SAFETY: 同じ世代のハンドル
        self.with_overlay(h, |raw| unsafe {
            ffi::pih_overlay_set_color(raw, rgb[0], rgb[1], rgb[2])
        })
    }

    pub fn set_sort_order(&self, h: OverlayHandle, order: u32) -> Result<(), VrError> {
        // SAFETY: 同じ世代のハンドル
        self.with_overlay(h, |raw| unsafe {
            ffi::pih_overlay_set_sort_order(raw, order)
        })
    }

    pub fn set_ignore_alpha(&self, h: OverlayHandle, on: bool) -> Result<(), VrError> {
        // SAFETY: 同じ世代のハンドル
        self.with_overlay(h, |raw| unsafe {
            ffi::pih_overlay_set_ignore_alpha(raw, on as c_int)
        })
    }

    pub fn set_transform(&self, h: OverlayHandle, pose: &Pose) -> Result<(), VrError> {
        let m = pose.to_matrix34();
        // SAFETY: 同じ世代のハンドルと、12要素の配列
        self.with_overlay(h, |raw| unsafe {
            ffi::pih_overlay_set_transform(raw, m.as_ptr())
        })
    }

    /// `texture` は `ID3D11Texture2D` のポインタ。呼んでいる間、有効であること
    pub fn set_texture_d3d11(&self, h: OverlayHandle, texture: *mut c_void) -> Result<(), VrError> {
        // SAFETY: 呼び出し側が有効なテクスチャを渡す
        self.with_overlay(h, |raw| unsafe {
            ffi::pih_overlay_set_texture_d3d11(raw, texture)
        })
    }

    pub fn set_bounds(&self, h: OverlayHandle, uv: (f32, f32, f32, f32)) -> Result<(), VrError> {
        // SAFETY: 同じ世代のハンドル
        self.with_overlay(h, |raw| unsafe {
            ffi::pih_overlay_set_bounds(raw, uv.0, uv.1, uv.2, uv.3)
        })
    }

    /// RGBA（1画素4バイト）の小さな画像を設定する
    pub fn set_raw(
        &self,
        h: OverlayHandle,
        rgba: &mut [u8],
        width: u32,
        height: u32,
    ) -> Result<(), VrError> {
        if rgba.len() != (width as usize) * (height as usize) * 4 {
            return Err(VrError::Overlay(0));
        }
        // SAFETY: 大きさを確かめたバッファを渡す。OpenVRは呼び出しの間に読み終える
        self.with_overlay(h, |raw| unsafe {
            ffi::pih_overlay_set_raw(raw, rgba.as_mut_ptr().cast(), width, height)
        })
    }

    // ---- 入力（位置合わせ） ----

    /// アクションの定義を読む（接続ごとに1回）
    pub fn init_input(&self, manifest: &Path) -> Result<(), VrError> {
        let mut conn = self.lock();
        if !conn.connected {
            return Err(VrError::NotConnected);
        }
        if conn.input.is_some() {
            return Ok(());
        }
        let p = path_cstr(manifest)?;
        let mut handles = [0u64; 5];
        // SAFETY: NUL終端の文字列と、5要素の配列
        check(
            unsafe { ffi::pih_input_init(p.as_ptr(), handles.as_mut_ptr()) },
            VrError::Input,
        )?;
        conn.input = Some(handles);
        Ok(())
    }

    /// 指定した手のボタンの状態
    pub fn poll_buttons(&self, side: Side) -> Result<HandButtons, VrError> {
        let conn = self.lock();
        let Some(handles) = conn.input.filter(|_| conn.connected) else {
            return Err(VrError::NotConnected);
        };
        let (mut grab, mut confirm) = (0, 0);
        let s = match side {
            Side::Left => 0,
            Side::Right => 1,
        };
        // SAFETY: 5要素の配列と、書き込み先を渡す
        check(
            unsafe { ffi::pih_input_poll(handles.as_ptr(), s, &mut grab, &mut confirm) },
            VrError::Input,
        )?;
        Ok(HandButtons {
            grab: grab == 1,
            confirm: confirm == 1,
        })
    }

    // ---- アプリの登録（SteamVRと一緒に起動する） ----

    fn with_apps<T>(&self, f: impl FnOnce() -> Result<T, VrError>) -> Result<T, VrError> {
        let conn = self.lock();
        if !conn.connected {
            return Err(VrError::NotConnected);
        }
        f()
    }

    pub fn app_installed(&self, key: &str) -> Result<bool, VrError> {
        let k = cstr(key)?;
        // SAFETY: NUL終端の文字列
        self.with_apps(|| Ok(unsafe { ffi::pih_apps_is_installed(k.as_ptr()) } == 1))
    }

    pub fn add_app_manifest(&self, path: &Path) -> Result<(), VrError> {
        let p = path_cstr(path)?;
        // SAFETY: NUL終端の文字列
        self.with_apps(|| {
            check(
                unsafe { ffi::pih_apps_add_manifest(p.as_ptr()) },
                VrError::Applications,
            )
        })
    }

    pub fn remove_app_manifest(&self, path: &Path) -> Result<(), VrError> {
        let p = path_cstr(path)?;
        // SAFETY: NUL終端の文字列
        self.with_apps(|| {
            check(
                unsafe { ffi::pih_apps_remove_manifest(p.as_ptr()) },
                VrError::Applications,
            )
        })
    }

    pub fn set_auto_launch(&self, key: &str, on: bool) -> Result<(), VrError> {
        let k = cstr(key)?;
        // SAFETY: NUL終端の文字列
        self.with_apps(|| {
            check(
                unsafe { ffi::pih_apps_set_auto_launch(k.as_ptr(), on as c_int) },
                VrError::Applications,
            )
        })
    }

    pub fn auto_launch(&self, key: &str) -> Result<bool, VrError> {
        let k = cstr(key)?;
        // SAFETY: NUL終端の文字列
        self.with_apps(|| Ok(unsafe { ffi::pih_apps_get_auto_launch(k.as_ptr()) } == 1))
    }
}

impl Drop for OpenVr {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DLLがなければ読み込めず、同梱の openvr_api.dll なら読み込んで関数を解決できる。
    /// SteamVRがない環境では、接続だけが失敗する。
    /// 読み込んだDLLはプロセスで1つのため（C++の層が持つ）、DLLを読み込むテストはこの1つだけにする
    #[test]
    fn loads_bundled_dll_and_reports_missing_one() {
        let missing = OpenVr::new(PathBuf::from("Z:/no/such/openvr_api.dll"));
        assert_eq!(missing.connect(), Err(VrError::NotLoaded));
        assert!(!missing.runtime_installed());
        assert_eq!(missing.generation(), None);
        assert!(missing.controller_pose(Side::Left).is_none());
        assert_eq!(
            missing.create_overlay("k", "n").unwrap_err(),
            VrError::NotConnected
        );

        let dll =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/openvr/bin/win64/openvr_api.dll");
        let vr = OpenVr::new(dll);
        if !vr.runtime_installed() {
            assert!(matches!(vr.connect(), Err(VrError::Init(_))));
        }
        vr.disconnect();
    }
}
