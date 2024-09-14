use anyhow::{Context, Result};
use colored::*;
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr; // coloredクレートを使用して色付け

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("Error at line {line}, column {column}: {message}")]
    GenericError {
        line: usize,
        column: usize,
        message: String,
    },
    #[error("Warning at line {line}, column {column}: {message}")]
    Warning {
        line: usize,
        column: usize,
        message: String,
    },
    #[error("Info at line {line}, column {column}: {message}")]
    Info {
        line: usize,
        column: usize,
        message: String,
    },
    #[error("Note at line {line}, column {column}: {message}")]
    Note {
        line: usize,
        column: usize,
        message: String,
    },
    #[error("Help at line {line}, column {column}: {message}")]
    Help {
        line: usize,
        column: usize,
        message: String,
    },
}

impl CompilerError {
    pub fn format_error_string(&self, source_code: &str, file_name: &str) -> String {
        match self {
            CompilerError::GenericError {
                line,
                column,
                message,
            } => self.format_message("error", line, column, message, source_code, file_name),
            CompilerError::Warning {
                line,
                column,
                message,
            } => self.format_message("warning", line, column, message, source_code, file_name),
            CompilerError::Info {
                line,
                column,
                message,
            } => self.format_message("info", line, column, message, source_code, file_name),
            CompilerError::Note {
                line,
                column,
                message,
            } => self.format_message("note", line, column, message, source_code, file_name),
            CompilerError::Help {
                line,
                column,
                message,
            } => self.format_message("help", line, column, message, source_code, file_name),
        }
    }

    fn format_message(
        &self,
        level: &str,
        line: &usize,
        column: &usize,
        message: &str,
        source_code: &str,
        file_name: &str,
    ) -> String {
        let lines: Vec<&str> = source_code.lines().collect();

        let mut error_message = format!(
            "{}: {} at line {}, column {}\n   {} {}:{}:{}\n",
            level.color(self.get_color(level)),
            message,
            line,
            column,
            "-->".truecolor(100, 100, 200),
            file_name,
            line,
            column
        );
        log::info!("lines len: {:?}", lines.len());
        if *line > 0 && *line <= lines.len() {
            let error_line = lines[line - 1];
            error_message.push_str(&format!("{} | {}\n", line, error_line));
            if *column > 0 && *column <= error_line.graphemes(true).count() {
                let mut char_count = 0;
                let mut width_count = 0;
                error_message.push_str(&format!("{} | ", " ".repeat(line.to_string().len())));
                for g in error_line.graphemes(true) {
                    let width = UnicodeWidthStr::width(g);
                    if char_count == *column - 1 {
                        error_message.push_str(&" ".repeat(width_count));
                        error_message.push_str(&"^".red().to_string());
                        break;
                    }
                    char_count += 1;
                    width_count += width;
                }
            } else {
                error_message.push_str(&"Column value is invalid.\n".red().to_string());
            }
        } else {
            error_message.push_str(&"Line value is invalid.\n".red().to_string());
        }

        error_message
    }

    fn get_color(&self, level: &str) -> &str {
        match level {
            "error" => "red",
            "warning" => "yellow",
            "info" => "blue",
            "note" => "green",
            "help" => "cyan",
            _ => "white",
        }
    }
}

#[macro_export]
macro_rules! custom_compile_error {
  /*
    // デフォルトのエラーレベルとファイル名を設定
    ($line:expr, $column:expr, $src:expr, $($arg:tt)*) => {
        {
            let error = crate::error::CompilerError::GenericError {
                line: $line,
                column: $column,
                message: format!($($arg)*),
            };
            error.format_error_string($src, "default-script")
        }
    };
    // エラーレベルを指定する場合
    ($level:expr, $line:expr, $column:expr, $src:expr, $($arg:tt)*) => {
        {
            let error = match $level {
                "error" => crate::error::CompilerError::GenericError {
                    line: $line,
                    column: $column,
                    message: format!($($arg)*),
                },
                "warning" => crate::error::CompilerError::Warning {
                    line: $line,
                    column: $column,
                    message: format!($($arg)*),
                },
                "info" => crate::error::CompilerError::Info {
                    line: $line,
                    column: $column,
                    message: format!($($arg)*),
                },
                "note" => crate::error::CompilerError::Note {
                    line: $line,
                    column: $column,
                    message: format!($($arg)*),
                },
                "help" => crate::error::CompilerError::Help {
                    line: $line,
                    column: $column,
                    message: format!($($arg)*),
                },
                _ => panic!("Invalid error level"),
            };
            error.format_error_string($src, "src/main.rs")
        }
    };
    */
    // エラーレベルとファイル名を指定する場合
    ($level:expr, $line:expr, $column:expr,  $file_name:expr,$src:expr, $($arg:tt)*) => {
        {
            let error = match $level {
                "error" => crate::error::CompilerError::GenericError {
                    line: $line,
                    column: $column,
                    message: format!($($arg)*),
                },
                "warning" => crate::error::CompilerError::Warning {
                    line: $line,
                    column: $column,
                    message: format!($($arg)*),
                },
                "info" => crate::error::CompilerError::Info {
                    line: $line,
                    column: $column,
                    message: format!($($arg)*),
                },
                "note" => crate::error::CompilerError::Note {
                    line: $line,
                    column: $column,
                    message: format!($($arg)*),
                },
                "help" => crate::error::CompilerError::Help {
                    line: $line,
                    column: $column,
                    message: format!($($arg)*),
                },
                _ => panic!("Invalid error level"),
            };
            error.format_error_string($src, $file_name)
        }
    };
}
