use aws_sdk_s3::Client;
use aws_types::region::Region;
use std::path::Path;
use aws_credential_types::Credentials;
use std::fs::metadata;
use image::ImageFormat;
use std::fs;
use uuid::{Uuid, Timestamp, NoContext};
use std::time::SystemTime;
use walkdir::WalkDir;

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

    /// ファイルが画像かどうかを判定する
    fn is_image(&self, path: &Path) -> bool {
        // ファイルの拡張子から画像フォーマットを判定
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return ImageFormat::from_extension(ext_str).is_some();
            }
        }
        false
    }

    /// ファイルサイズが制限内かどうかを判定する
    fn is_valid_size(&self, path: &Path) -> bool {
        if let Ok(metadata) = metadata(path) {
            let size = metadata.len();
            size >= self.min_image_size && size <= self.max_image_size
        } else {
            false
        }
    }

    /// ディレクトリ内の画像ファイルをアップロードする
    pub async fn upload_directory(&self, dir_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        println!("Uploading images from directory: {:?}", dir_path);
        
        // ディレクトリ内のファイルを再帰的に探索
        for entry in WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // ファイルで、画像で、サイズが制限内の場合のみアップロード
            if path.is_file() && self.is_image(path) && self.is_valid_size(path) {
                println!("Found image file: {:?}", path);
                if let Err(e) = self.upload_file(path).await {
                    eprintln!("Failed to upload file {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    pub async fn upload_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        // ファイルのメタデータを取得
        let metadata = fs::metadata(path)?;
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

        // ファイルを読み込む
        let body = fs::read(path)?;

        // S3にアップロード
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&new_key)
            .body(body.into())
            .send()
            .await?;

        println!("Uploaded file to S3: {}", new_key);

        Ok(())
    }
} 