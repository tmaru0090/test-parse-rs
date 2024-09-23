use anyhow::{Context, Result};
use colored::*;
use std::fmt;
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub struct CompilerError {
    messages: Vec<ErrorMessage>,
}

struct ErrorMessage {
    level: String,
    lines: Vec<(usize, usize)>,
    message: String,
    children: Vec<ChildMessage>,
}

struct ChildMessage {
    level: String,
    message: String,
}

impl CompilerError {
    pub fn new() -> Self {
        CompilerError {
            messages: Vec::new(),
        }
    }

    pub fn add_group_message(&mut self, level: &str, lines: Vec<(usize, usize)>, message: &str) {
        self.messages.push(ErrorMessage {
            level: level.to_string(),
            lines,
            message: message.to_string(),
            children: Vec::new(),
        });
    }

    pub fn add_message(&mut self, level: &str, line: usize, column: usize, message: &str) {
        self.messages.push(ErrorMessage {
            level: level.to_string(),
            lines: vec![(line, column)],
            message: message.to_string(),
            children: Vec::new(),
        });
    }

    pub fn add_child_message(&mut self, index: usize, level: &str, message: &str) {
        if let Some(msg) = self.messages.get_mut(index) {
            msg.children.push(ChildMessage {
                level: level.to_string(),
                message: message.to_string(),
            });
        }
    }

    pub fn format_error_string(&self, file: &str, source_code: &str) -> String {
        let mut result = String::new();
        for msg in &self.messages {
            let color = match msg.level.as_str() {
                "warning" => "warning".yellow().bold(),
                "error" => "error".red().bold(),
                "note" => "note".blue().bold(),
                _ => "info".normal(),
            };
            result.push_str(&format!(
                "{}: {}\n  {} {}:{}:{}\n",
                color,
                msg.message,
                "-->".blue().bold(),
                file,
                msg.lines[0].0,
                msg.lines[0].1
            ));
            for &(line, column) in &msg.lines {
                if let Some(source_line) = source_code.lines().nth(line - 1) {
                    result.push_str(&format!(
                        "{} {}   {}  \n",
                        line.to_string().blue().bold(),
                        "|".blue().bold(),
                        source_line
                    ));
                    result.push_str(&format!(
                        "   {} {}  {}  \n",
                        "|".blue().bold(),
                        " ".repeat(column - 1),
                        "^".repeat(1).red().bold()
                    ));
                }
            }
            for child in &msg.children {
                let child_color = match child.level.as_str() {
                    "note" => "note".blue().bold(),
                    _ => "info".normal(),
                };
                result.push_str(&format!("  = {}: {}\n", child_color, child.message));
            }
        }
        result
    }
}

#[macro_export]
macro_rules! compile_error_with_children {
    ($level:expr, $file_name:expr, $src:expr, $line:expr, $column:expr, $message:expr, $($child_level:expr, $child_message:expr),*) => {
        {
            let mut compiler_error = crate::error::CompilerError::new();
            compiler_error.add_message($level, $line, $column, $message);
            $(
                compiler_error.add_child_message(0, $child_level, $child_message);
            )*
            compiler_error.format_error_string($file_name, $src)
        }
    };
}

#[macro_export]
macro_rules! compile_error {
    ($level:expr, $line:expr, $column:expr, $file_name:expr, $src:expr, $($arg:tt)*) => {
        {
            let mut error = crate::error::CompilerError::new();
            error.add_message($level, $line, $column, &format!($($arg)*));
            error.format_error_string($file_name, $src)
        }
    };
}

#[macro_export]
macro_rules! compile_group_error {
    ($level:expr, $file_name:expr, $src:expr, $message:expr, $($line:expr, $column:expr),*) => {
        {
            let mut compiler_error = crate::error::CompilerError::new();
            compiler_error.add_group_message($level, vec![$(($line, $column)),*], $message);
            compiler_error.format_error_string($file_name, $src)
        }
    };
}
