use serde::{Deserialize, Serialize};

pub use glam::{Mat3, Quat, Vec3};

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

    /// 大きさの内側に収めた矩形。幅や高さが0になる場合は全体を返す
    pub fn clamped(self, size: Size) -> Rect {
        let x = self.x.min(size.width.saturating_sub(1));
        let y = self.y.min(size.height.saturating_sub(1));
        let width = self.width.min(size.width.saturating_sub(x));
        let height = self.height.min(size.height.saturating_sub(y));
        if width == 0 || height == 0 {
            return Rect::full(size);
        }
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// 0〜1に正規化した範囲 `(u_min, v_min, u_max, v_max)`（オーバーレイのテクスチャの範囲）
    pub fn uv_bounds(&self, size: Size) -> (f32, f32, f32, f32) {
        if size.width == 0 || size.height == 0 {
            return (0.0, 0.0, 1.0, 1.0);
        }
        let w = size.width as f32;
        let h = size.height as f32;
        (
            self.x as f32 / w,
            self.y as f32 / h,
            (self.x + self.width) as f32 / w,
            (self.y + self.height) as f32 / h,
        )
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

impl Pose {
    pub fn new(position: Vec3, rotation: Quat) -> Self {
        Pose { position, rotation }
    }

    /// `self` の座標系で表した `local` を、外の座標系へ移す（姿勢の合成）
    pub fn compose(&self, local: &Pose) -> Pose {
        Pose {
            position: self.position + self.rotation * local.position,
            rotation: (self.rotation * local.rotation).normalize(),
        }
    }

    /// 逆の姿勢
    pub fn inverse(&self) -> Pose {
        let r = self.rotation.inverse();
        Pose {
            position: r * -self.position,
            rotation: r,
        }
    }

    /// `self` から見た `other` の相対姿勢（`self.compose(&self.relative(other))` は `other` になる）
    pub fn relative(&self, other: &Pose) -> Pose {
        self.inverse().compose(other)
    }

    /// OpenVRの `HmdMatrix34_t`（3行4列、行優先）にする
    pub fn to_matrix34(&self) -> [f32; 12] {
        let m = Mat3::from_quat(self.rotation);
        let p = self.position;
        #[rustfmt::skip]
        let out = [
            m.x_axis.x, m.y_axis.x, m.z_axis.x, p.x,
            m.x_axis.y, m.y_axis.y, m.z_axis.y, p.y,
            m.x_axis.z, m.y_axis.z, m.z_axis.z, p.z,
        ];
        out
    }

    /// OpenVRの `HmdMatrix34_t`（3行4列、行優先）から作る。回転の部分は正規化する
    pub fn from_matrix34(m: &[f32; 12]) -> Pose {
        let rot = Mat3::from_cols(
            Vec3::new(m[0], m[4], m[8]),
            Vec3::new(m[1], m[5], m[9]),
            Vec3::new(m[2], m[6], m[10]),
        );
        Pose {
            position: Vec3::new(m[3], m[7], m[11]),
            rotation: Quat::from_mat3(&rot).normalize(),
        }
    }

    /// 2つの姿勢が、位置（メートル）と回転（四元数の成分。符号が逆でも同じ回転）の許容の範囲で同じか
    pub fn approx_eq(&self, other: &Pose, eps: f32) -> bool {
        self.position.abs_diff_eq(other.position, eps)
            && (self.rotation.abs_diff_eq(other.rotation, eps)
                || self.rotation.abs_diff_eq(-other.rotation, eps))
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

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Pose {
        Pose::new(
            Vec3::new(0.1, 1.2, -0.3),
            Quat::from_euler(glam::EulerRot::YXZ, 0.4, -0.7, 0.2),
        )
    }

    #[test]
    fn matrix_round_trip() {
        let p = sample();
        assert!(Pose::from_matrix34(&p.to_matrix34()).approx_eq(&p, 1e-5));
    }

    #[test]
    fn matrix_is_row_major_with_translation_last() {
        let p = Pose::new(Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY);
        let m = p.to_matrix34();
        assert_eq!([m[3], m[7], m[11]], [1.0, 2.0, 3.0]);
        assert_eq!([m[0], m[5], m[10]], [1.0, 1.0, 1.0]);
        // Y軸まわりに90度：X軸がZの負の向きへ（3行目の1列目）
        let r = Pose::new(
            Vec3::ZERO,
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        );
        let m = r.to_matrix34();
        assert!((m[8] + 1.0).abs() < 1e-6, "{m:?}");
    }

    #[test]
    fn relative_then_compose_gives_back_the_pose() {
        let base = sample();
        let target = Pose::new(Vec3::new(-0.2, 0.9, 0.4), Quat::from_rotation_x(0.3));
        let offset = base.relative(&target);
        assert!(base.compose(&offset).approx_eq(&target, 1e-5));
        assert!(
            base.compose(&base.inverse())
                .approx_eq(&Pose::default(), 1e-5)
        );
    }

    #[test]
    fn rect_clamps_and_maps_to_uv() {
        let size = Size {
            width: 100,
            height: 200,
        };
        let r = Rect {
            x: 90,
            y: 10,
            width: 50,
            height: 50,
        }
        .clamped(size);
        assert_eq!(
            r,
            Rect {
                x: 90,
                y: 10,
                width: 10,
                height: 50
            }
        );
        assert_eq!(r.uv_bounds(size), (0.9, 0.05, 1.0, 0.3));
        let empty = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 10,
        };
        assert_eq!(empty.clamped(size), Rect::full(size));
    }
}
