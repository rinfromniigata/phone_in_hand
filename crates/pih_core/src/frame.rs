//! 切り抜きと黒い画面・信号断の検出（`frame_processor`。取り込み方法によらず共通）
//!
//! 判定は、取り込みが間引いて付けた縮小画像（[`Thumbnail`]）だけで行う。縮小画像のないフレームには、
//! 直前の判定の結果を返す。閾値はスペックの委任事項で、この下の定数に置く。
use std::time::{Duration, Instant};

use crate::geometry::{Rect, Size};
use crate::settings::{CropMode, CropSettings};
use crate::video::{Frame, Thumbnail, VideoStatus};

/// 1枚の映像の判定の結果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    pub crop: Rect,
    /// スマホの画面が消えている（黒い画面）
    pub screen_off: bool,
}

pub trait FrameProcessor: Send {
    /// 間引いた縮小画像で判定する
    fn analyze(&mut self, frame: &Frame, now: Instant) -> FrameInfo;
    /// 切り抜きの設定を変える
    fn set_crop(&mut self, crop: CropSettings);
    /// フレームが来ないまま時間が過ぎた（ウィンドウの取り込みは、画面が変わらないとフレームを送らない）。
    /// 最後の判定のまま時間だけを進める。まだ一度も判定していなければ `None`
    fn tick(&mut self, now: Instant) -> Option<FrameInfo>;
}

/// この明るさ（0〜255）以下の画素を黒とみなす
pub const BLACK_LUMA: u8 = 24;
/// 行・列のうち、黒でない画素がこの割合以上あれば、映像のある行・列とみなす
pub const CONTENT_LINE_SHARE: f32 = 0.02;
/// 黒でない画素が全体のこの割合より少なければ、黒い画面とみなす
pub const DARK_FRAME_SHARE: f32 = 0.002;
/// 黒い画面がこの時間続いたら、画面が消えたとみなす（暗い場面での誤判定を避ける）
pub const SCREEN_OFF_AFTER: Duration = Duration::from_millis(1500);
/// 自動の切り抜きは、同じ候補がこの回数続いたら変える（揺れを避ける）
pub const CROP_STABLE_COUNT: u32 = 3;
/// 切り抜きの候補の違いが、縦横それぞれの大きさのこの割合以内なら、同じ候補とみなす
pub const CROP_TOLERANCE: f32 = 0.01;
/// CPUのフレームから作る縮小画像の幅の上限
pub const THUMBNAIL_MAX_WIDTH: u32 = 360;

/// パネルを隠すべきか：取り込みが動いていない（信号なし・途絶・ウィンドウが閉じた）か、黒い画面
pub fn screen_off(status: &VideoStatus, info: Option<&FrameInfo>) -> bool {
    !matches!(status, VideoStatus::Running { .. }) || info.is_some_and(|i| i.screen_off)
}

/// 黒い余白の自動の切り抜きと手動の矩形、黒い画面の検出
#[derive(Debug)]
pub struct BlackBarProcessor {
    crop: CropSettings,
    size: Option<Size>,
    auto_crop: Option<Rect>,
    candidate: Option<(Rect, u32)>,
    dark_since: Option<Instant>,
    screen_off: bool,
}

impl BlackBarProcessor {
    pub fn new(crop: CropSettings) -> Self {
        BlackBarProcessor {
            crop,
            size: None,
            auto_crop: None,
            candidate: None,
            dark_since: None,
            screen_off: false,
        }
    }

    fn current_crop(&self, size: Size) -> Rect {
        match (self.crop.mode, self.crop.rect) {
            (CropMode::Manual, Some(r)) => r.clamped(size),
            (CropMode::Manual, None) => Rect::full(size),
            (CropMode::Auto, _) => self.auto_crop.unwrap_or(Rect::full(size)),
        }
    }

    fn update_dark(&mut self, dark: bool, now: Instant) {
        if dark {
            let since = *self.dark_since.get_or_insert(now);
            self.screen_off = now.duration_since(since) >= SCREEN_OFF_AFTER;
        } else {
            self.dark_since = None;
            self.screen_off = false;
        }
    }

    fn update_auto_crop(&mut self, found: Rect, size: Size) {
        let same = |a: &Rect, b: &Rect| {
            let tw = (size.width as f32 * CROP_TOLERANCE).ceil() as i64;
            let th = (size.height as f32 * CROP_TOLERANCE).ceil() as i64;
            let d = |p: u32, q: u32| (p as i64 - q as i64).abs();
            d(a.x, b.x) <= tw
                && d(a.width, b.width) <= tw
                && d(a.y, b.y) <= th
                && d(a.height, b.height) <= th
        };
        if self.auto_crop.as_ref().is_some_and(|c| same(c, &found)) {
            self.candidate = None;
            return;
        }
        let count = match self.candidate {
            Some((c, n)) if same(&c, &found) => n + 1,
            _ => 1,
        };
        if count >= CROP_STABLE_COUNT {
            self.auto_crop = Some(found);
            self.candidate = None;
        } else {
            self.candidate = Some((found, count));
        }
    }
}

/// 縮小画像の判定：黒い画面か、黒くない範囲（縮小画像の座標）
fn inspect(t: &Thumbnail) -> (bool, Option<Rect>) {
    let (w, h) = (t.size.width, t.size.height);
    if w == 0 || h == 0 {
        return (true, None);
    }
    let mut col = vec![0u32; w as usize];
    let mut row = vec![0u32; h as usize];
    let mut bright = 0u32;
    for y in 0..h {
        for x in 0..w {
            if t.luma(x, y) > BLACK_LUMA {
                col[x as usize] += 1;
                row[y as usize] += 1;
                bright += 1;
            }
        }
    }
    let dark = (bright as f32) < (w * h) as f32 * DARK_FRAME_SHARE;
    if dark {
        return (true, None);
    }
    let col_min = (h as f32 * CONTENT_LINE_SHARE).max(1.0) as u32;
    let row_min = (w as f32 * CONTENT_LINE_SHARE).max(1.0) as u32;
    let x0 = col.iter().position(|&c| c >= col_min);
    let x1 = col.iter().rposition(|&c| c >= col_min);
    let y0 = row.iter().position(|&c| c >= row_min);
    let y1 = row.iter().rposition(|&c| c >= row_min);
    let rect = match (x0, x1, y0, y1) {
        (Some(x0), Some(x1), Some(y0), Some(y1)) if x1 >= x0 && y1 >= y0 => Some(Rect {
            x: x0 as u32,
            y: y0 as u32,
            width: (x1 - x0 + 1) as u32,
            height: (y1 - y0 + 1) as u32,
        }),
        _ => None,
    };
    (false, rect)
}

/// 縮小画像の座標の矩形を、元の大きさの座標へ広げる（外側へ丸める）
fn scale_up(r: Rect, thumb: Size, full: Size) -> Rect {
    let sx = full.width as f64 / thumb.width as f64;
    let sy = full.height as f64 / thumb.height as f64;
    let x0 = (r.x as f64 * sx).floor() as u32;
    let y0 = (r.y as f64 * sy).floor() as u32;
    let x1 = (((r.x + r.width) as f64 * sx).ceil() as u32).min(full.width);
    let y1 = (((r.y + r.height) as f64 * sy).ceil() as u32).min(full.height);
    Rect {
        x: x0,
        y: y0,
        width: x1.saturating_sub(x0),
        height: y1.saturating_sub(y0),
    }
    .clamped(full)
}

impl FrameProcessor for BlackBarProcessor {
    fn analyze(&mut self, frame: &Frame, now: Instant) -> FrameInfo {
        let size = frame.size();
        if self.size != Some(size) {
            // 大きさが変わったら、前の切り抜きは使えない
            self.size = Some(size);
            self.auto_crop = None;
            self.candidate = None;
        }
        let owned;
        let thumbnail = match frame {
            Frame::Gpu { thumbnail, .. } => thumbnail.as_deref(),
            Frame::Cpu { format, data, .. } => {
                owned = Thumbnail::from_cpu(*format, size, data, THUMBNAIL_MAX_WIDTH);
                owned.as_ref()
            }
        };
        if let Some(t) = thumbnail {
            let (dark, found) = inspect(t);
            self.update_dark(dark, now);
            if let (CropMode::Auto, Some(r)) = (self.crop.mode, found) {
                self.update_auto_crop(scale_up(r, t.size, size), size);
            }
        }
        FrameInfo {
            crop: self.current_crop(size),
            screen_off: self.screen_off,
        }
    }

    fn set_crop(&mut self, crop: CropSettings) {
        if crop.mode != self.crop.mode {
            self.auto_crop = None;
            self.candidate = None;
        }
        self.crop = crop;
    }

    fn tick(&mut self, now: Instant) -> Option<FrameInfo> {
        let size = self.size?;
        if self.dark_since.is_some() {
            self.update_dark(true, now);
        }
        Some(FrameInfo {
            crop: self.current_crop(size),
            screen_off: self.screen_off,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::{PixelFormat, SharedTexture};
    use std::sync::Arc;

    const FULL: Size = Size {
        width: 1920,
        height: 1080,
    };

    /// 縮小画像（192×108）で、`content`（縮小画像の座標）の内側だけが明るい映像
    fn frame(content: Option<Rect>) -> Frame {
        let ts = Size {
            width: 192,
            height: 108,
        };
        let mut bgra = vec![0u8; (ts.width * ts.height * 4) as usize];
        if let Some(c) = content {
            for y in c.y..c.y + c.height {
                for x in c.x..c.x + c.width {
                    let i = ((y * ts.width + x) * 4) as usize;
                    // 暗めの灰色と白の縞（暗い画面のアプリでも映像とみなす）
                    let v = if x % 4 == 0 { 230 } else { 40 };
                    bgra[i..i + 4].copy_from_slice(&[v, v, v, 255]);
                }
            }
        }
        Frame::Gpu {
            texture: SharedTexture {
                ptr: 0,
                owner: Arc::new(()),
            },
            size: FULL,
            thumbnail: Some(Arc::new(Thumbnail { size: ts, bgra })),
        }
    }

    fn without_thumbnail() -> Frame {
        Frame::Gpu {
            texture: SharedTexture {
                ptr: 0,
                owner: Arc::new(()),
            },
            size: FULL,
            thumbnail: None,
        }
    }

    /// 縦長のスマホの画面が中央にある（左右に黒い余白）
    const PHONE: Rect = Rect {
        x: 71,
        y: 0,
        width: 50,
        height: 108,
    };

    #[test]
    fn auto_crop_finds_content_after_it_is_stable() {
        let mut p = BlackBarProcessor::new(CropSettings::default());
        let now = Instant::now();
        // 1回目・2回目はまだ全体
        assert_eq!(p.analyze(&frame(Some(PHONE)), now).crop, Rect::full(FULL));
        assert_eq!(p.analyze(&frame(Some(PHONE)), now).crop, Rect::full(FULL));
        let info = p.analyze(&frame(Some(PHONE)), now);
        // 縮小の10倍で、外側へ丸める
        assert_eq!(
            info.crop,
            Rect {
                x: 710,
                y: 0,
                width: 500,
                height: 1080
            }
        );
        assert!(!info.screen_off);
        // 縮小画像のないフレームでも、同じ切り抜きを返す
        assert_eq!(p.analyze(&without_thumbnail(), now).crop, info.crop);
    }

    #[test]
    fn small_jitter_does_not_change_crop() {
        let mut p = BlackBarProcessor::new(CropSettings::default());
        let now = Instant::now();
        for _ in 0..3 {
            p.analyze(&frame(Some(PHONE)), now);
        }
        let stable = p.analyze(&frame(Some(PHONE)), now).crop;
        let jitter = Rect { x: 72, ..PHONE };
        for _ in 0..5 {
            assert_eq!(p.analyze(&frame(Some(jitter)), now).crop, stable);
        }
    }

    #[test]
    fn black_screen_turns_off_after_a_while_and_back_on_at_once() {
        let mut p = BlackBarProcessor::new(CropSettings::default());
        let t0 = Instant::now();
        assert!(!p.analyze(&frame(None), t0).screen_off);
        assert!(
            !p.analyze(&frame(None), t0 + SCREEN_OFF_AFTER / 2)
                .screen_off
        );
        let off = p.analyze(&frame(None), t0 + SCREEN_OFF_AFTER);
        assert!(off.screen_off);
        // 黒い間は切り抜きを変えない
        assert_eq!(off.crop, Rect::full(FULL));
        // 縮小画像のないフレームでは、前の判定のまま
        assert!(
            p.analyze(&without_thumbnail(), t0 + SCREEN_OFF_AFTER * 2)
                .screen_off
        );
        // 画面がつけばすぐに戻る
        assert!(
            !p.analyze(&frame(Some(PHONE)), t0 + SCREEN_OFF_AFTER * 2)
                .screen_off
        );
    }

    #[test]
    fn black_screen_without_further_frames_turns_off_by_time() {
        let mut p = BlackBarProcessor::new(CropSettings::default());
        let t0 = Instant::now();
        assert_eq!(p.tick(t0), None);
        // 黒い1枚のあと、フレームが来ない
        assert!(!p.analyze(&frame(None), t0).screen_off);
        assert!(!p.tick(t0 + SCREEN_OFF_AFTER / 2).unwrap().screen_off);
        assert!(p.tick(t0 + SCREEN_OFF_AFTER).unwrap().screen_off);
        // 明るい画面のままなら、時間が過ぎても消えたことにしない
        let mut lit = BlackBarProcessor::new(CropSettings::default());
        lit.analyze(&frame(Some(PHONE)), t0);
        assert!(!lit.tick(t0 + SCREEN_OFF_AFTER * 3).unwrap().screen_off);
    }

    #[test]
    fn manual_crop_uses_the_rect_clamped() {
        let rect = Rect {
            x: 100,
            y: 50,
            width: 5000,
            height: 500,
        };
        let mut p = BlackBarProcessor::new(CropSettings {
            mode: CropMode::Manual,
            rect: Some(rect),
        });
        let info = p.analyze(&frame(Some(PHONE)), Instant::now());
        assert_eq!(
            info.crop,
            Rect {
                x: 100,
                y: 50,
                width: 1820,
                height: 500
            }
        );
        // 自動に戻すと、自動の判定からやり直す
        p.set_crop(CropSettings::default());
        assert_eq!(
            p.analyze(&frame(Some(PHONE)), Instant::now()).crop,
            Rect::full(FULL)
        );
    }

    #[test]
    fn cpu_frames_are_judged_from_their_pixels() {
        let size = Size {
            width: 64,
            height: 32,
        };
        let mut p = BlackBarProcessor::new(CropSettings::default());
        let t0 = Instant::now();
        let black = Frame::Cpu {
            format: PixelFormat::Nv12,
            size,
            data: vec![16u8; 64 * 32 * 3 / 2].into(),
        };
        p.analyze(&black, t0);
        assert!(p.analyze(&black, t0 + SCREEN_OFF_AFTER).screen_off);
    }

    #[test]
    fn panel_is_hidden_unless_running_and_lit() {
        let running = VideoStatus::Running {
            device: "w".into(),
            size: FULL,
            fps: 60.0,
        };
        let lit = FrameInfo {
            crop: Rect::full(FULL),
            screen_off: false,
        };
        let dark = FrameInfo {
            screen_off: true,
            ..lit
        };
        assert!(!screen_off(&running, Some(&lit)));
        assert!(!screen_off(&running, None));
        assert!(screen_off(&running, Some(&dark)));
        assert!(screen_off(&VideoStatus::NoSignal, Some(&lit)));
        assert!(screen_off(
            &VideoStatus::Error {
                error: crate::video::VideoError::SourceLost
            },
            None
        ));
    }
}
