use serde_json::Value;

pub fn get_str(val: &Value, field: &str) -> Result<String, String> {
    val.as_str()
        .ok_or_else(|| format!("Invalid {}", field))
        .map(String::from)
}

pub fn get_str_or_default(val: &Value) -> String {
    val.as_str().unwrap_or("").to_string()
}

pub fn get_f32(val: &Value, field: &str) -> Result<f32, String> {
    val.as_f64()
        .ok_or_else(|| format!("Invalid {}", field))
        .map(|v| v as f32)
}

pub fn get_i32(val: &Value, field: &str) -> Result<i32, String> {
    val.as_i64()
        .ok_or_else(|| format!("Invalid {}", field))
        .map(|v| v as i32)
}

pub fn get_i64(val: &Value, field: &str) -> Result<i64, String> {
    val.as_i64()
        .ok_or_else(|| format!("Invalid {}", field))
}

pub fn get_u64(val: &Value, field: &str) -> Result<u64, String> {
    val.as_u64()
        .ok_or_else(|| format!("Invalid {}", field))
}

pub fn get_array<'a>(val: &'a Value, field: &str) -> Result<&'a Vec<Value>, String> {
    val.as_array()
        .ok_or_else(|| format!("{} must be an array", field))
}

pub fn get_object<'a>(val: &'a Value, field: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    val.as_object()
        .ok_or_else(|| format!("{} must be an object", field))
}

pub fn get_optional_str(obj: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    obj.get(key).and_then(|v| v.as_str()).map(String::from)
}

pub fn parse_string_array(val: &Value) -> Vec<String> {
    val.as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

pub fn parse_u32_array(val: &Value) -> Vec<u32> {
    val.as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect())
        .unwrap_or_default()
}

pub fn parse_single_or_multiple<T, F>(value: &Value, single: F, multiple: fn(Vec<u32>) -> T) -> Result<T, String>
where
    F: FnOnce(u32) -> T,
{
    if let Some(num) = value.as_u64() {
        return Ok(single(num as u32));
    }
    
    if let Some(arr) = value.as_array() {
        let positions = arr.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect();
        return Ok(multiple(positions));
    }
    
    Err("Invalid position value".to_string())
}
