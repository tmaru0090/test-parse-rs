use serde_json::Value;
pub trait Size {
    fn size(&self) -> usize;
}

impl Size for Value {
    fn size(&self) -> usize {
        match self {
            Value::Null => 0,
            Value::Bool(_) => std::mem::size_of::<bool>(),
            Value::Number(n) => {
                if n.is_i64() {
                    std::mem::size_of::<i64>()
                } else if n.is_u64() {
                    std::mem::size_of::<u64>()
                } else if n.is_f64() {
                    std::mem::size_of::<f64>()
                } else if let Some(i) = n.as_i64() {
                    if i <= i32::MAX as i64 && i >= i32::MIN as i64 {
                        std::mem::size_of::<i32>()
                    } else {
                        std::mem::size_of::<i64>()
                    }
                } else if let Some(u) = n.as_u64() {
                    if u <= u32::MAX as u64 {
                        std::mem::size_of::<u32>()
                    } else {
                        std::mem::size_of::<u64>()
                    }
                } else if let Some(f) = n.as_f64() {
                    if f <= f32::MAX as f64 && f >= f32::MIN as f64 {
                        std::mem::size_of::<f32>()
                    } else {
                        std::mem::size_of::<f64>()
                    }
                } else {
                    0 // ここは適宜調整
                }
            }
            Value::String(s) => std::mem::size_of::<String>() + s.len(),
            Value::Array(arr) => {
                std::mem::size_of::<Vec<Value>>() + arr.iter().map(|v| v.size()).sum::<usize>()
            }
            Value::Object(obj) => {
                std::mem::size_of::<serde_json::Map<String, Value>>()
                    + obj.iter().map(|(k, v)| k.len() + v.size()).sum::<usize>()
            }
        }
    }
}
