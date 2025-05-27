/// パスがパターンにマッチするかチェックする
pub fn is_path_match(path: &str, parts: &[&str]) -> bool {
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