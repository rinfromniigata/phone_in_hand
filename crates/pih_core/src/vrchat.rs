//! VRChat（`vrchat_bridge`）
use serde::Serialize;

use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct VrchatStatus {
    pub osc_found: bool,
    pub params_found: bool,
}

pub trait VrchatBridge: Send {
    fn send_visible(&mut self, v: bool) -> Result<()>;
    fn send_is_left(&mut self, v: bool) -> Result<()>;
    /// OSCQueryとmDNS
    fn query(&mut self) -> VrchatStatus;
}
