use serde::Serialize;

/// モジュールの間とUIとの境界で使う失敗の種類。
///
/// UIへは [`Error::code`] の文字列だけを渡し、文言はUIの辞書で引く（UIで文を組み立てない）。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// まだ実装していない、またはこの環境では使えない機能
    #[error("使用できません")]
    Unavailable,
    /// 機器が見つからない
    #[error("機器が見つかりません: {0}")]
    NotFound(String),
    /// 入力の値が正しくない
    #[error("値が正しくありません: {0}")]
    Invalid(String),
    /// 保存したファイルが読めない
    #[error("保存したファイルが壊れています: {0}")]
    Corrupt(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Error {
    /// UIへ渡す識別子（辞書のキーに対応させる）
    pub fn code(&self) -> ErrorCode {
        match self {
            Error::Unavailable => ErrorCode::Unavailable,
            Error::NotFound(_) => ErrorCode::NotFound,
            Error::Invalid(_) | Error::Json(_) => ErrorCode::Invalid,
            Error::Corrupt(_) => ErrorCode::Corrupt,
            Error::Io(_) => ErrorCode::Io,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unavailable,
    NotFound,
    Invalid,
    Corrupt,
    Io,
}

pub type Result<T> = std::result::Result<T, Error>;
