use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WatchDirectory {
    /// ディレクトリパス
    pub dir: String,
    /// このディレクトリの画像に付けるタグ
    pub tag: String,
} 