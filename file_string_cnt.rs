use std::fs;
use std::io;
use std::path::Path;

fn main() -> io::Result<()> {
    // カレントディレクトリの取得
    let current_dir = std::env::current_dir()?;
    println!("Scanning directory: {:?}", current_dir);

    // ディレクトリの中身を走査
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        // ファイルかどうかを確認
        if path.is_file() {
            // ファイル名の取得
            let file_name = path.file_name().unwrap().to_string_lossy();
            
            // ファイルの内容を読み込み
            match fs::read_to_string(&path) {
                Ok(contents) => {
                    // 文字数のカウント
                    let char_count = contents.chars().count();
                    println!("File: {}, Character count: {}", file_name, char_count);
                }
                Err(e) => {
                    println!("Failed to read file: {}, Error: {}", file_name, e);
                }
            }
        }
    }
    
    Ok(())
}
