use aws_sdk_s3::Client;
use aws_types::region::Region;
use std::path::Path;
use aws_credential_types::Credentials;
use std::fs::metadata;
use std::fs;
use uuid::{Uuid, Timestamp, NoContext};
use std::time::SystemTime;
use walkdir::WalkDir;
use crate::s3::util::{is_image, is_valid_size};

pub struct S3Client {
    client: Client,
    bucket: String,
    max_image_size: u64,
    min_image_size: u64,
}

impl S3Client {
    pub async fn new(
        region: &str,
        bucket: &str,
        access_key_id: &str,
        secret_access_key: &str,
        endpoint_url: &str,
        max_image_size: u64,
        min_image_size: u64,
    ) -> Self {
        let region = Region::new(region.to_string());
        let config = aws_config::from_env()
            .region(region)
            .endpoint_url(endpoint_url)
            .credentials_provider(Credentials::new(
                access_key_id,
                secret_access_key,
                None,
                None,
                "static",
            ))
            .load()
            .await;
        let client = Client::new(&config);

        Self {
            client,
            bucket: bucket.to_string(),
            max_image_size,
            min_image_size,
        }
    }

    /// ディレクトリ内の画像ファイルをアップロードする
    pub async fn upload_directory(&self, dir_path: &Path, dir_tag: &str, common_tags: &std::collections::HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Uploading images from directory: {:?}", dir_path);
        
        // ディレクトリ内のファイルを再帰的に探索
        for entry in WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // ファイルで、画像で、サイズが制限内の場合のみアップロード
            if path.is_file() && is_image(path) && is_valid_size(path, self.min_image_size, self.max_image_size) {
                println!("Found image file: {:?}", path);
                if let Err(e) = self.upload_file(path, dir_tag, common_tags).await {
                    eprintln!("Failed to upload file {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    pub async fn upload_file(&self, path: &Path, dir_tag: &str, common_tags: &std::collections::HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        // ファイルのメタデータを取得
        let metadata = metadata(path)?;
        let file_size = metadata.len();

        // ファイルサイズをチェック
        if file_size < self.min_image_size || file_size > self.max_image_size {
            println!("File size {} is not within the allowed range ({} - {})", file_size, self.min_image_size, self.max_image_size);
            return Ok(());
        }

        // ファイルの生成時間を取得
        let created = metadata.created()?.duration_since(SystemTime::UNIX_EPOCH)?.as_secs() as u64;
        let ts = Timestamp::from_unix(NoContext, created, 0);
        // UUIDv7を生成（タイムスタンプベース）
        let uuid = Uuid::new_v7(ts);

        // 元のファイル名から拡張子を取得
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        // 新しいファイル名を生成
        let new_key = format!("{}.{}", uuid, extension);
        
        let content_type = match extension {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            _ => "application/octet-stream", // デフォルトのコンテンツタイプ
        };

        // ファイルを読み込む
        let body = fs::read(path)?;

        // S3のメタデータを作成
        let mut metadata_map = std::collections::HashMap::new();
        metadata_map.insert("dir-tag".to_string(), dir_tag.to_string());
        for (k, v) in common_tags.iter() {
            metadata_map.insert(k.clone(), v.clone());
        }

        // S3にアップロード
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&new_key)
            .body(body.into())
            .content_type(content_type)
            .set_metadata(Some(metadata_map))
            .send()
            .await?;

        println!("Uploaded file to S3: {}", new_key);

        Ok(())
    }
} 