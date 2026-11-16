//! D3D11のデバイスと、GPUのフレームの縮小画像の読み出し。
//!
//! ウィンドウの取り込み（WGC）とオーバーレイの表示は、同じデバイスを使う。デバイスのコンテキストは
//! 映像のスレッドだけが使うが、念のためマルチスレッドの保護を有効にする。
use std::sync::Arc;

use pih_core::geometry::Size;
use pih_core::video::Thumbnail;
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_RESOURCE_MISC_GENERATE_MIPS, D3D11_RESOURCE_MISC_SHARED, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, D3D11CreateDevice,
    ID3D11Device, ID3D11DeviceContext, ID3D11Multithread, ID3D11ShaderResourceView,
    ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::core::Interface;

/// 映像の経路で共有するD3D11のデバイス
pub struct Gpu {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    /// WGCに渡すWinRTのデバイス
    pub winrt_device: IDirect3DDevice,
}

// SAFETY: D3D11のデバイスはフリースレッドで、コンテキストはマルチスレッドの保護を有効にしている
unsafe impl Send for Gpu {}
unsafe impl Sync for Gpu {}

impl Gpu {
    pub fn new() -> windows::core::Result<Arc<Gpu>> {
        let mut device = None;
        let mut context = None;
        // SAFETY: 出力先のポインタを渡す
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
        }
        let device: ID3D11Device = device.ok_or_else(windows::core::Error::empty)?;
        let context: ID3D11DeviceContext = context.ok_or_else(windows::core::Error::empty)?;
        if let Ok(mt) = context.cast::<ID3D11Multithread>() {
            // SAFETY: 戻り値は前の設定で、使わない
            unsafe {
                let _ = mt.SetMultithreadProtected(true);
            }
        }
        let dxgi: IDXGIDevice = device.cast()?;
        // SAFETY: 有効なDXGIのデバイスを渡す
        let winrt_device: IDirect3DDevice =
            unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? }.cast()?;
        Ok(Arc::new(Gpu {
            device,
            context,
            winrt_device,
        }))
    }

    /// BGRAのテクスチャを作る
    pub fn create_texture(
        &self,
        size: Size,
        bind: u32,
        misc: u32,
        mip_levels: u32,
    ) -> windows::core::Result<ID3D11Texture2D> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: size.width,
            Height: size.height,
            MipLevels: mip_levels,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: bind,
            CPUAccessFlags: 0,
            MiscFlags: misc,
        };
        let mut tex = None;
        // SAFETY: 記述子と出力先を渡す
        unsafe { self.device.CreateTexture2D(&desc, None, Some(&mut tex))? };
        tex.ok_or_else(windows::core::Error::empty)
    }

    /// オーバーレイに渡すテクスチャ（SteamVRが開けるように共有にする）
    pub fn create_overlay_texture(&self, size: Size) -> windows::core::Result<ID3D11Texture2D> {
        self.create_texture(
            size,
            D3D11_BIND_SHADER_RESOURCE.0 as u32,
            D3D11_RESOURCE_MISC_SHARED.0 as u32,
            1,
        )
    }

    /// `src` の左上から `size` の範囲を、`dst` の左上へ写す（GPUの中だけで写す）
    pub fn copy(&self, dst: &ID3D11Texture2D, src: &ID3D11Texture2D, size: Size) {
        let b = D3D11_BOX {
            left: 0,
            top: 0,
            front: 0,
            right: size.width,
            bottom: size.height,
            back: 1,
        };
        // SAFETY: 同じデバイスの、同じ形式のテクスチャ
        unsafe {
            self.context
                .CopySubresourceRegion(dst, 0, 0, 0, 0, src, 0, Some(&b));
        }
    }

    pub fn flush(&self) {
        // SAFETY: 引数なし
        unsafe { self.context.Flush() };
    }
}

/// テクスチャの大きさ
pub fn texture_size(tex: &ID3D11Texture2D) -> Size {
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    // SAFETY: 出力先を渡す
    unsafe { tex.GetDesc(&mut desc) };
    Size {
        width: desc.Width,
        height: desc.Height,
    }
}

/// GPUのフレームを、ミップマップで縮小してCPUへ読み出す
pub struct ThumbnailReader {
    gpu: Arc<Gpu>,
    max_width: u32,
    state: Option<ReaderState>,
}

struct ReaderState {
    source_size: Size,
    mips: ID3D11Texture2D,
    srv: ID3D11ShaderResourceView,
    level: u32,
    level_size: Size,
    staging: ID3D11Texture2D,
}

impl ThumbnailReader {
    pub fn new(gpu: Arc<Gpu>, max_width: u32) -> Self {
        ThumbnailReader {
            gpu,
            max_width,
            state: None,
        }
    }

    fn prepare(&mut self, size: Size) -> windows::core::Result<&ReaderState> {
        if self.state.as_ref().map(|s| s.source_size) != Some(size) {
            self.state = None;
            // 幅が max_width 以下になる段まで縮める
            let mut level = 0;
            let mut w = size.width;
            let mut h = size.height;
            while w > self.max_width && w > 1 && h > 1 {
                w = (w / 2).max(1);
                h = (h / 2).max(1);
                level += 1;
            }
            let mips = self.gpu.create_texture(
                size,
                (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
                D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
                level + 1,
            )?;
            let mut srv = None;
            // SAFETY: 作ったテクスチャと出力先を渡す
            unsafe {
                self.gpu
                    .device
                    .CreateShaderResourceView(&mips, None, Some(&mut srv))?
            };
            let level_size = Size {
                width: w,
                height: h,
            };
            let desc = D3D11_TEXTURE2D_DESC {
                Width: w,
                Height: h,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            let mut staging = None;
            // SAFETY: 記述子と出力先を渡す
            unsafe {
                self.gpu
                    .device
                    .CreateTexture2D(&desc, None, Some(&mut staging))?
            };
            self.state = Some(ReaderState {
                source_size: size,
                mips,
                srv: srv.ok_or_else(windows::core::Error::empty)?,
                level,
                level_size,
                staging: staging.ok_or_else(windows::core::Error::empty)?,
            });
        }
        Ok(self.state.as_ref().expect("直前に作った"))
    }

    /// `src`（大きさ `size`）の縮小画像を読み出す。GPUの処理の完了を待つため、間引いて呼ぶ
    pub fn read(&mut self, src: &ID3D11Texture2D, size: Size) -> windows::core::Result<Thumbnail> {
        let gpu = self.gpu.clone();
        let st = self.prepare(size)?;
        gpu.copy(&st.mips, src, size);
        let ctx = &gpu.context;
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: 同じデバイスのテクスチャとビュー。読み出しの後に Unmap する
        unsafe {
            ctx.GenerateMips(&st.srv);
            ctx.CopySubresourceRegion(&st.staging, 0, 0, 0, 0, &st.mips, st.level, None);
            ctx.Map(&st.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
        }
        let (w, h) = (st.level_size.width as usize, st.level_size.height as usize);
        let mut bgra = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            // SAFETY: Map の結果の、行の幅（RowPitch）の範囲だけを読む
            let row = unsafe {
                std::slice::from_raw_parts(
                    (mapped.pData as *const u8).add(y * mapped.RowPitch as usize),
                    w * 4,
                )
            };
            bgra.extend_from_slice(row);
        }
        // SAFETY: Map したものを戻す
        unsafe { ctx.Unmap(&st.staging, 0) };
        Ok(Thumbnail {
            size: st.level_size,
            bgra,
        })
    }
}
