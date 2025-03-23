use notify::{Watcher, RecursiveMode, Result, recommended_watcher, Event};
use std::path::Path;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;
use std::env;
use tokio::runtime::Runtime;
use walkdir::WalkDir;

mod config;
mod s3;
use config::Config;
use s3::S3Client;

fn main() -> Result<()> {
    // コマンドライン引数からコンフィグファイルのパスを取得
    let config_path = env::args().nth(1).map(|s| s.into());
    
    // コンフィグファイルを読み込む
    let config = Config::load(config_path).expect("Failed to load config");
    
    // Tokioランタイムを作成
    let runtime = Runtime::new().expect("Failed to create Tokio runtime");

    // S3クライアントを初期化
    let s3_client = runtime.block_on(S3Client::new(
        &config.s3_region,
        &config.s3_bucket,
        &config.s3_access_key_id,
        &config.s3_secret_access_key,
        &config.s3_url,
        config.max_image_size,
        config.min_image_size,
    ));
    
    // 各パターンのベースディレクトリを取得
    for pattern in &config.watch_dir {
        println!("Processing pattern: {}", pattern);
        // パターンを*で分割
        let parts: Vec<&str> = pattern.split('*').collect();
        println!("Split parts: {:?}", parts);
        if let Some(base_dir) = parts.first() {
            println!("Base directory: {}", base_dir);
            // ベースディレクトリから再帰的にディレクトリを探索
            for entry in WalkDir::new(base_dir).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let path_str = path.to_str().unwrap_or("");
                    println!("Checking directory: {}", path_str);
                    // パスがパターンにマッチするかチェック
                    if is_path_match(path_str, &parts) {
                        println!("Match found! Pattern: {}, Path: {}", pattern, path_str);
                        // 既存の画像をアップロードするかどうかをチェック
                        if config.upload_existing {
                            // ディレクトリ内の画像ファイルをアップロード
                            if let Err(e) = runtime.block_on(s3_client.upload_directory(path)) {
                                eprintln!("Failed to upload directory {:?}: {}", path, e);
                            }
                        } else {
                            println!("Skipping existing images in directory: {:?}", path);
                        }
                    } else {
                        println!("No match. Pattern: {}, Path: {}", pattern, path_str);
                    }
                }
            }
        }
    }

    // Create a channel to receive the events
    let (sender, receiver) = channel();

    // Create a watcher
    let mut watcher = recommended_watcher(sender)?;

    // 監視対象のディレクトリを設定
    if config.use_regex {
        // 各パターンのベースディレクトリを取得
        for pattern in &config.watch_dir {
            println!("Processing pattern: {}", pattern);
            // パターンを*で分割
            let parts: Vec<&str> = pattern.split('*').collect();
            println!("Split parts: {:?}", parts);
            if let Some(base_dir) = parts.first() {
                println!("Base directory: {}", base_dir);
                // ベースディレクトリから再帰的にディレクトリを探索
                for entry in WalkDir::new(base_dir).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        let path_str = path.to_str().unwrap_or("");
                        println!("Checking directory: {}", path_str);
                        // パスがパターンにマッチするかチェック
                        if is_path_match(path_str, &parts) {
                            println!("Match found! Pattern: {}, Path: {}", pattern, path_str);
                            watcher.watch(path, RecursiveMode::Recursive)?;
                            println!("Monitoring directory: {:?}", path);
                        } else {
                            println!("No match. Pattern: {}, Path: {}", pattern, path_str);
                        }
                    }
                }
            }
        }
    } else {
        // 通常のパスを使用する場合
        for dir in &config.watch_dir {
            let path = Path::new(dir);
            watcher.watch(path, RecursiveMode::Recursive)?;
            println!("Monitoring directory: {:?}", path);
        }
    }

    println!("Press Ctrl-C to stop");

    // Loop to receive events
    loop {
        match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => handle_event(event?, &s3_client, &runtime),
            Err(RecvTimeoutError::Timeout) => continue, // Just continue on timeout
            Err(e) => return Err(notify::Error::generic(format!("Watch error: {:?}", e).as_str())),
        }
    }
}

/// パスがパターンにマッチするかチェックする
fn is_path_match(path: &str, parts: &[&str]) -> bool {
    let mut current_pos = 0;
    for (i, part) in parts.iter().enumerate() {
        // 最初の部分は完全一致
        if i == 0 {
            if !path.starts_with(part) {
                return false;
            }
            current_pos = part.len();
        }
        // 最後の部分は完全一致
        else if i == parts.len() - 1 {
            if !path[current_pos..].ends_with(part) {
                return false;
            }
        }
        // 中間の部分は部分一致
        else {
            if let Some(pos) = path[current_pos..].find(part) {
                current_pos += pos + part.len();
            } else {
                return false;
            }
        }
    }
    true
}

fn handle_event(event: Event, s3_client: &S3Client, runtime: &Runtime) {
    match event.kind {
        notify::EventKind::Create(_) => println!("File created: {:?}", event.paths),
        notify::EventKind::Modify(_) => {
            for path in event.paths {
                println!("File modified: {:?}", path);
                // ファイルをS3にアップロード
                if let Err(e) = runtime.block_on(s3_client.upload_file(&path)) {
                    eprintln!("Failed to upload file to S3: {}", e);
                }
            }
        }
        notify::EventKind::Remove(_) => println!("File removed: {:?}", event.paths),
        _ => println!("Other event: {:?}", event.kind),
    }
}
