use std::path::PathBuf;
use std::collections::HashMap;
use crate::config::watch_directory::WatchDirectory;
use crate::config::path_match::is_path_match;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// 監視対象のディレクトリパス（正規表現パターン）
    pub watch_dir: Vec<WatchDirectory>,
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
    /// 全ての画像に共通で付けるタグ
    #[serde(default = "default_common_tags")]
    pub common_tags: HashMap<String, String>,
}

fn default_use_regex() -> bool {
    false
}

fn default_upload_existing() -> bool {
    true
}

fn default_common_tags() -> HashMap<String, String> {
    HashMap::new()
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
        let config: Config = serde_json::from_str(&config_str)?;

        Ok(config)
    }
    
    /// 指定されたディレクトリパスに対応するタグを取得する
    pub fn get_tag_for_directory(&self, dir_path: &str) -> Option<&str> {
        for watch_dir in &self.watch_dir {
            // パターンマッチングでディレクトリパスがwatch_dirにマッチするかチェック
            if self.use_regex {
                // パターンを*で分割してマッチングを行う
                let parts: Vec<&str> = watch_dir.dir.split('*').collect();
                if is_path_match(dir_path, &parts) {
                    return Some(&watch_dir.tag);
                }
            } else {
                // 完全一致
                if dir_path == watch_dir.dir {
                    return Some(&watch_dir.tag);
                }
            }
        }
        None
    }
} 