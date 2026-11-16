//! `overlay_presenter`（T3-6）：映像と姿勢をSteamVRのオーバーレイへ反映する。
//!
//! - パネル（映像）とチップ（状態の文言）の2つのオーバーレイを持つ
//! - 映像はGPUの中で自分のテクスチャへ写してから渡す（取り込みのフレームはすぐにプールへ戻すため）。
//!   切り抜きはテクスチャの範囲（`SetOverlayTextureBounds`）で指定し、画素を写し直さない
//! - チップと位置合わせの見本の画像は、UIが辞書の文言とトークンの値で描いて渡す（[`OverlayImages`]）
//! - SteamVRとつなぎ直したら、オーバーレイを作り直して状態を当て直す
use std::collections::HashMap;
use std::sync::Arc;

use pih_core::geometry::{Pose, Rect, Size};
use pih_core::overlay::{
    DIMMED_ALPHA, OverlayPresenter, aspect, chip_pose, chip_width_m, panel_size_m,
};
use pih_core::pose::{Chip, PanelState, PanelVisibility};
use pih_core::video::Frame;
use pih_core::{Error, Result};
use serde::Deserialize;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
use windows::core::Interface;

use crate::gpu::Gpu;
use crate::openvr::{OpenVr, OverlayHandle, VrError};

const PANEL_KEY: &str = "com.rinontech.phone-in-hand.panel";
const CHIP_KEY: &str = "com.rinontech.phone-in-hand.chip";
const OVERLAY_NAME: &str = "phone_in_hand";

/// RGBA（1画素4バイト）の画像
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    #[serde(with = "base64_bytes")]
    pub rgba: Vec<u8>,
}

impl RgbaImage {
    pub fn is_valid(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.rgba.len() == (self.width as usize) * (self.height as usize) * 4
    }
}

mod base64_bytes {
    use base64::Engine;
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .map_err(serde::de::Error::custom)
    }
}

/// UIが描いて渡す画像：状態のチップ（S04）と、位置合わせの見本（映像がないとき）
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OverlayImages {
    pub chips: HashMap<Chip, RgbaImage>,
    pub sample: Option<RgbaImage>,
}

struct Overlays {
    panel: OverlayHandle,
    chip: OverlayHandle,
}

/// パネルに出しているもの
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelContent {
    None,
    Video,
    Sample,
}

pub struct OpenVrOverlayPresenter {
    vr: Arc<OpenVr>,
    gpu: Arc<Gpu>,
    overlays: Option<Overlays>,
    /// 映像を写す自分のテクスチャ
    texture: Option<(ID3D11Texture2D, Size)>,
    crop: Option<Rect>,
    content: PanelContent,
    images: OverlayImages,
    shown_chip: Option<Chip>,
    state: Option<PanelState>,
    enabled: bool,
    size_inch: f32,
    brightness: f32,
    /// 位置合わせ中は、スイッチや画面の状態によらずパネルを出す
    calibrating: bool,
}

// SAFETY: テクスチャはフリースレッドのD3D11のオブジェクトで、使うのはロックを持ったスレッドだけ
unsafe impl Send for OpenVrOverlayPresenter {}

impl OpenVrOverlayPresenter {
    pub fn new(vr: Arc<OpenVr>, gpu: Arc<Gpu>) -> Self {
        OpenVrOverlayPresenter {
            vr,
            gpu,
            overlays: None,
            texture: None,
            crop: None,
            content: PanelContent::None,
            images: OverlayImages::default(),
            shown_chip: None,
            state: None,
            enabled: false,
            size_inch: 6.7,
            brightness: 1.0,
            calibrating: false,
        }
    }

    /// 接続の世代が変わっていれば、オーバーレイを作り直す。つながっていなければ `None`
    fn ensure(&mut self) -> Option<&Overlays> {
        let generation = self.vr.generation()?;
        if self
            .overlays
            .as_ref()
            .is_some_and(|o| o.panel.generation == generation)
        {
            return self.overlays.as_ref();
        }
        self.overlays = None;
        self.content = PanelContent::None;
        self.shown_chip = None;
        let created = (|| -> std::result::Result<Overlays, VrError> {
            let panel = self.vr.create_overlay(PANEL_KEY, OVERLAY_NAME)?;
            let chip = self.vr.create_overlay(CHIP_KEY, OVERLAY_NAME)?;
            // 取り込んだ映像のアルファは意味を持たないため無視する。チップはパネルより手前に描く
            self.vr.set_ignore_alpha(panel, true)?;
            self.vr.set_sort_order(panel, 0)?;
            self.vr.set_sort_order(chip, 1)?;
            self.vr.set_visible(panel, false)?;
            self.vr.set_visible(chip, false)?;
            Ok(Overlays { panel, chip })
        })();
        match created {
            Ok(o) => {
                self.overlays = Some(o);
                self.apply_all();
                self.overlays.as_ref()
            }
            Err(e) => {
                log::warn!("オーバーレイを作れません: {e}");
                None
            }
        }
    }

    /// つなぎ直した後に、持っている状態を全部当て直す
    fn apply_all(&mut self) {
        if self.texture.is_some()
            && let Err(e) = self.present_texture()
        {
            log::debug!("{e}");
        }
        self.apply_appearance();
        if let Some(s) = self.state {
            self.apply_state(&s);
        }
    }

    fn panel_aspect(&self) -> f32 {
        match self.content {
            PanelContent::Sample => self
                .images
                .sample
                .as_ref()
                .map(|i| i.width as f32 / i.height as f32)
                .unwrap_or_else(|| aspect(Rect::default())),
            _ => aspect(self.crop.unwrap_or_default()),
        }
    }

    fn panel_size(&self) -> (f32, f32) {
        panel_size_m(self.size_inch, self.panel_aspect())
    }

    fn apply_appearance(&mut self) {
        let Some(o) = self.overlays.as_ref() else {
            return;
        };
        let (width, _) = self.panel_size();
        let b = self.brightness.clamp(0.0, 1.0);
        let r = self
            .vr
            .set_width(o.panel, width)
            .and_then(|_| self.vr.set_color(o.panel, [b, b, b]));
        if let Err(e) = r {
            log::debug!("{e}");
        }
    }

    /// 自分のテクスチャをパネルに出す
    fn present_texture(&mut self) -> std::result::Result<(), VrError> {
        let (Some(o), Some((tex, size))) = (self.overlays.as_ref(), self.texture.as_ref()) else {
            return Ok(());
        };
        self.vr.set_texture_d3d11(o.panel, tex.as_raw())?;
        let crop = self.crop.unwrap_or(Rect::full(*size)).clamped(*size);
        self.vr.set_bounds(o.panel, crop.uv_bounds(*size))?;
        self.content = PanelContent::Video;
        Ok(())
    }

    /// 映像がないときに、位置合わせの見本の画像を出す
    fn present_sample(&mut self) {
        let Some(o) = self.overlays.as_ref() else {
            return;
        };
        let (panel, mut sample) = match self.images.sample.clone() {
            Some(s) if s.is_valid() => (o.panel, s),
            _ => return,
        };
        let r = self
            .vr
            .set_bounds(panel, (0.0, 0.0, 1.0, 1.0))
            .and_then(|_| {
                self.vr
                    .set_raw(panel, &mut sample.rgba, sample.width, sample.height)
            });
        match r {
            Ok(()) => {
                self.content = PanelContent::Sample;
                self.apply_appearance();
            }
            Err(e) => log::debug!("見本を出せません: {e}"),
        }
    }

    fn apply_state(&mut self, s: &PanelState) {
        let calibrating = self.calibrating;
        let screen_off = s.screen_off;
        if calibrating && (screen_off || self.content == PanelContent::None) {
            if self.content != PanelContent::Sample {
                self.present_sample();
            }
        } else if self.content == PanelContent::Sample && self.texture.is_some() && !screen_off {
            if let Err(e) = self.present_texture() {
                log::debug!("{e}");
            }
            self.apply_appearance();
        }
        let Some(o) = self.overlays.as_ref() else {
            return;
        };
        let (panel, chip) = (o.panel, o.chip);
        let (_, height) = self.panel_size();

        let panel_visible = s.has_pose
            && if calibrating {
                true
            } else {
                self.enabled && s.visibility != PanelVisibility::Hidden
            };
        let alpha = if s.visibility == PanelVisibility::Dimmed {
            DIMMED_ALPHA
        } else {
            1.0
        };
        let r = self
            .vr
            .set_transform(panel, &s.pose)
            .and_then(|_| self.vr.set_alpha(panel, alpha))
            .and_then(|_| self.vr.set_visible(panel, panel_visible));
        if let Err(e) = r {
            log::debug!("{e}");
        }

        // チップ：位置合わせ中は出さない（見本だけを見せる）
        let want = if self.enabled && !calibrating {
            s.chip()
                .filter(|c| self.images.chips.get(c).is_some_and(RgbaImage::is_valid))
        } else {
            None
        };
        if want != self.shown_chip {
            if let Some(c) = want {
                let mut img = self.images.chips[&c].clone();
                let r = self
                    .vr
                    .set_raw(chip, &mut img.rgba, img.width, img.height)
                    .and_then(|_| self.vr.set_width(chip, chip_width_m(img.width, img.height)));
                if let Err(e) = r {
                    log::debug!("チップを出せません: {e}");
                }
            }
            self.shown_chip = want;
        }
        let chip_visible = want.is_some();
        if chip_visible {
            let hidden = !panel_visible;
            if let Err(e) = self
                .vr
                .set_transform(chip, &chip_pose(&s.pose, height, hidden))
            {
                log::debug!("{e}");
            }
        }
        if let Err(e) = self.vr.set_visible(chip, chip_visible) {
            log::debug!("{e}");
        }
    }

    /// UIが描いた画像を受け取る（言語やテーマを変えたときにも届く）
    pub fn set_images(&mut self, images: OverlayImages) {
        self.images = images;
        self.shown_chip = None;
        if self.content == PanelContent::Sample {
            self.content = PanelContent::None;
        }
        if let Some(s) = self.state {
            self.ensure();
            self.apply_state(&s);
        }
    }

    /// 位置合わせの間、パネルを出し続ける
    pub fn set_calibrating(&mut self, on: bool) {
        self.calibrating = on;
        if let Some(s) = self.state {
            self.ensure();
            self.apply_state(&s);
        }
    }

    /// 新しいフレームなしに切り抜きだけが変わった（変化のないウィンドウで範囲を動かしたとき）
    pub fn set_crop(&mut self, crop: Rect) {
        if self.crop == Some(crop) {
            return;
        }
        self.crop = Some(crop);
        if self.content == PanelContent::Video && self.ensure().is_some() {
            if let Err(e) = self.present_texture() {
                log::debug!("{e}");
            }
            self.apply_appearance();
        }
    }

    /// 取り込みを止めたとき：映像を捨てる
    pub fn clear_frame(&mut self) {
        self.texture = None;
        self.crop = None;
        if self.content == PanelContent::Video {
            self.content = PanelContent::None;
        }
    }

    /// SteamVRとの接続が切れた
    pub fn on_disconnected(&mut self) {
        self.overlays = None;
        self.shown_chip = None;
        self.content = PanelContent::None;
    }
}

impl OverlayPresenter for OpenVrOverlayPresenter {
    fn set_frame(&mut self, frame: &Frame, crop: Rect) -> Result<()> {
        let Frame::Gpu { texture, size, .. } = frame else {
            // CPUのフレーム（HDMIキャプチャ）はPhase 4でつなぐ
            return Err(Error::Unavailable);
        };
        let size = *size;
        let raw = texture.ptr as *mut std::ffi::c_void;
        // SAFETY: owner が生きている間、ptr は有効な ID3D11Texture2D（同じデバイスのもの）
        let Some(src) = (unsafe { ID3D11Texture2D::from_raw_borrowed(&raw) }) else {
            return Err(Error::Invalid("texture".into()));
        };
        if self.texture.as_ref().map(|(_, s)| *s) != Some(size) {
            let tex = self.gpu.create_overlay_texture(size).map_err(|e| {
                log::warn!("オーバーレイのテクスチャを作れません: {e}");
                Error::Unavailable
            })?;
            self.texture = Some((tex, size));
        }
        let aspect_changed = self.crop.map(aspect) != Some(aspect(crop));
        self.crop = Some(crop);
        let (dst, _) = self.texture.as_ref().expect("直前に作った");
        self.gpu.copy(dst, src, size);
        self.gpu.flush();

        if self.ensure().is_none() {
            return Ok(());
        }
        // 位置合わせで見本を出している間は、画面がつくまで差し替えない
        let showing_sample =
            self.content == PanelContent::Sample && self.state.is_some_and(|s| s.screen_off);
        if !showing_sample {
            self.present_texture()?;
            if aspect_changed {
                self.apply_appearance();
            }
        }
        Ok(())
    }

    fn set_state(&mut self, state: &PanelState) -> Result<()> {
        self.state = Some(*state);
        if self.ensure().is_some() {
            self.apply_state(state);
        }
        Ok(())
    }

    fn set_appearance(&mut self, size_inch: f32, brightness: f32) -> Result<()> {
        self.size_inch = size_inch;
        self.brightness = brightness;
        if self.ensure().is_some() {
            self.apply_appearance();
        }
        Ok(())
    }

    fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        self.enabled = enabled;
        if let Some(s) = self.state
            && self.ensure().is_some()
        {
            self.apply_state(&s);
        }
        Ok(())
    }
}

/// 位置合わせなどで、パネルの姿勢だけを直接決めるときに使う
pub fn shown_at(pose: Pose) -> PanelState {
    PanelState {
        pose,
        visibility: PanelVisibility::Shown,
        mode: pih_core::pose::PanelMode::Controller,
        screen_off: false,
        has_pose: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn images_deserialize_from_base64() {
        let rgba = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let b64 = base64::engine::general_purpose::STANDARD.encode(&rgba);
        let json = serde_json::json!({
            "chips": { "tracking": { "width": 2, "height": 1, "rgba": b64 } },
            "sample": null
        });
        let images: OverlayImages = serde_json::from_value(json).unwrap();
        let chip = &images.chips[&Chip::Tracking];
        assert_eq!(chip.rgba, rgba);
        assert!(chip.is_valid());
        assert!(images.sample.is_none());
    }

    #[test]
    fn presenter_without_steamvr_keeps_state_quietly() {
        let Ok(gpu) = Gpu::new() else {
            return;
        };
        let vr = Arc::new(OpenVr::new("Z:/no/openvr_api.dll".into()));
        let mut p = OpenVrOverlayPresenter::new(vr, gpu);
        p.set_enabled(true).unwrap();
        p.set_appearance(6.1, 0.5).unwrap();
        p.set_state(&shown_at(Pose::default())).unwrap();
        assert!(p.overlays.is_none());
        assert_eq!(p.size_inch, 6.1);
    }
}
