use notify::Event;
use crate::s3::S3Client;
use crate::config::Config;
use tokio::runtime::Runtime;

pub fn handle_event(event: Event, s3_client: &S3Client, runtime: &Runtime, config: &Config) {
    match event.kind {
        notify::EventKind::Create(_) => println!("File created: {:?}", event.paths),
        notify::EventKind::Modify(_) => {
            for path in event.paths {
                println!("File modified: {:?}", path);
                // ファイルの親ディレクトリからタグを取得
                let dir_path = path.parent().and_then(|p| p.to_str()).unwrap_or("");
                let dir_tag = config.get_tag_for_directory(dir_path).unwrap_or("");
                // ファイルをS3にアップロード
                if let Err(e) = runtime.block_on(s3_client.upload_file(&path, dir_tag, &config.common_tags)) {
                    eprintln!("Failed to upload file to S3: {}", e);
                }
            }
        }
        notify::EventKind::Remove(_) => println!("File removed: {:?}", event.paths),
        _ => println!("Other event: {:?}", event.kind),
    }
} 