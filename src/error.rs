use anyhow::{Context, Result};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("Error at line {line}, column {column}: {message}")]
    GenericError {
        line: usize,
        column: usize,
        message: String,
    },
}

impl CompilerError {
    pub fn format_error(&self, source_code: &str) -> String {
        match self {
            CompilerError::GenericError {
                line,
                column,
                message,
            } => {
                let lines: Vec<&str> = source_code.lines().collect();
                let mut error_message =
                    format!("Error at line {}, column {}: {}\n", line, column, message);

                if *line > 0 && *line <= lines.len() {
                    let error_line = lines[line - 1];
                    error_message.push_str(error_line);
                    error_message.push('\n');
                    if *column > 0 && *column <= error_line.graphemes(true).count() {
                        let mut char_count = 0;
                        let mut width_count = 0;
                        for g in error_line.graphemes(true) {
                            let width = UnicodeWidthStr::width(g);
                            if char_count == *column - 1 {
                                error_message.push('^');
                                break;
                            }
                            char_count += 1;
                            width_count += width;
                            error_message.push_str(&" ".repeat(width));
                        }
                    } else {
                        error_message.push_str("Column value is invalid.\n");
                    }
                } else {
                    error_message.push_str("Line value is invalid.\n");
                }

                error_message
            }
        }
    }
}

#[macro_export]
macro_rules! custom_compile_error {
    ($line:expr, $column:expr, $src:expr, $($arg:tt)*) => {
        {
            let error = crate::error::CompilerError::GenericError {
                line: $line,
                column: $column,
                message: format!($($arg)*),
            };
            error.format_error($src)
        }
    };
}
