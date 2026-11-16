use serde::{Deserialize, Serialize};

pub use glam::{Quat, Vec3};

/// 画素の大きさ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// 画素の矩形（左上が原点）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// 大きさ全体を覆う矩形
    pub fn full(size: Size) -> Self {
        Rect {
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
        }
    }
}

/// 位置と向き（SteamVRのstanding空間。単位はメートル）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub position: Vec3,
    pub rotation: Quat,
}

impl Default for Pose {
    fn default() -> Self {
        Pose {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}

/// 左右
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    #[default]
    Right,
}
