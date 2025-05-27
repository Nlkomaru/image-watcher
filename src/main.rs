use notify::{Watcher, RecursiveMode, Result, recommended_watcher};
use std::path::Path;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;
use std::env;
use tokio::runtime::Runtime;
use walkdir::WalkDir;

mod config;
mod s3;
mod event_handler;
use config::{Config, is_path_match};
use s3::S3Client;
use event_handler::handle_event;

fn process_existing_directories(config: &Config, s3_client: &S3Client, runtime: &Runtime) {
    for watch_dir in &config.watch_dir {
        println!("Processing pattern: {}", watch_dir.dir);
        let parts: Vec<&str> = watch_dir.dir.split('*').collect();
        println!("Split parts: {:?}", parts);
        if let Some(base_dir) = parts.first() {
            println!("Base directory: {}", base_dir);
            for entry in WalkDir::new(base_dir).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let path_str = path.to_str().unwrap_or("");
                    if is_path_match(path_str, &parts) {
                        println!("Match found! Pattern: {}, Path: {}", watch_dir.dir, path_str);
                        if config.upload_existing {
                            if let Err(e) = runtime.block_on(s3_client.upload_directory(path, &watch_dir.tag, &config.common_tags)) {
                                eprintln!("Failed to upload directory {:?}: {}", path, e);
                            }
                        } else {
                            println!("Skipping existing images in directory: {:?}", path);
                        }
                    }
                }
            }
        }
    }
}

fn register_watch_directories(config: &Config, watcher: &mut impl Watcher) -> Result<()> {
    if config.use_regex {
        for watch_dir in &config.watch_dir {
            println!("Processing pattern: {}", watch_dir.dir);
            let parts: Vec<&str> = watch_dir.dir.split('*').collect();
            println!("Split parts: {:?}", parts);
            if let Some(base_dir) = parts.first() {
                println!("Base directory: {}", base_dir);
                for entry in WalkDir::new(base_dir).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        let path_str = path.to_str().unwrap_or("");
                        if is_path_match(path_str, &parts) {
                            watcher.watch(path, RecursiveMode::Recursive)?;
                            println!("Monitoring directory: {:?}", path);
                        }
                    }
                }
            }
        }
    } else {
        for dir in &config.watch_dir {
            let path = Path::new(&dir.dir);
            watcher.watch(path, RecursiveMode::Recursive)?;
            println!("Monitoring directory: {:?}", path);
        }
    }
    Ok(())
}

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

    // 既存画像のアップロード
    process_existing_directories(&config, &s3_client, &runtime);

    // Create a channel to receive the events
    let (sender, receiver) = channel();
    // Create a watcher
    let mut watcher = recommended_watcher(sender)?;
    // 監視ディレクトリの登録
    register_watch_directories(&config, &mut watcher)?;

    println!("Press Ctrl-C to stop");

    // Loop to receive events
    loop {
        match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => handle_event(event?, &s3_client, &runtime, &config),
            Err(RecvTimeoutError::Timeout) => continue, // Just continue on timeout
            Err(e) => return Err(notify::Error::generic(format!("Watch error: {:?}", e).as_str())),
        }
    }
}
