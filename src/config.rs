use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// 監視対象のディレクトリパス（正規表現パターン）
    pub watch_dir: Vec<String>,
    /// 正規表現パターンを使用するかどうか
    #[serde(default = "default_use_regex")]
    pub use_regex: bool,
    /// 既存の画像をアップロードするかどうか
    #[serde(default = "default_upload_existing")]
    pub upload_existing: bool,
    /// S3のバケット名
    pub s3_bucket: String,
    /// S3のリージョン
    pub s3_region: String,
    /// S3のアクセスキーID
    pub s3_access_key_id: String,
    /// S3のシークレットアクセスキー
    pub s3_secret_access_key: String,
    /// S3のエンドポイントURL
    pub s3_url: String,
    /// 画像の最大サイズ（バイト）
    pub max_image_size: u64,
    /// 画像の最小サイズ（バイト）
    pub min_image_size: u64,
}

fn default_use_regex() -> bool {
    false
}

fn default_upload_existing() -> bool {
    true
}

impl Config {
    /// コンフィグファイルを読み込む
    pub fn load(config_path: Option<PathBuf>) -> Result<Self, Box<dyn std::error::Error>> {
        // コンフィグファイルのパスを取得
        let config_path = match config_path {
            Some(path) => path,
            None => PathBuf::from("config.json"),
        };

        // コンフィグファイルを読み込む
        let config_str = std::fs::read_to_string(&config_path)?;
        let mut config: Config = serde_json::from_str(&config_str)?;

        // パターンを*で分割して処理
        let mut processed_patterns = Vec::new();
        for pattern in &config.watch_dir {
            // パターンを*で分割
            let parts: Vec<&str> = pattern.split('*').collect();
            // パターンを再構築
            let processed_pattern = parts.join("*");
            processed_patterns.push(processed_pattern);
        }
        config.watch_dir = processed_patterns;

        Ok(config)
    }
} 