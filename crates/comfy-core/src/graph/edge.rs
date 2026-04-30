use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Link {
    pub from_node: String,
    pub from_socket: usize,
}

impl Link {
    pub fn new(from_node: impl Into<String>, from_socket: usize) -> Self {
        Self {
            from_node: from_node.into(),
            from_socket,
        }
    }
}

pub fn is_link(value: &serde_json::Value) -> bool {
    let arr = match value.as_array() {
        Some(a) if a.len() == 2 => a,
        _ => return false,
    };
    arr[0].is_string() && (arr[1].is_number())
}

pub fn parse_link(value: &serde_json::Value) -> Option<Link> {
    let arr = value.as_array()?;
    if arr.len() != 2 {
        return None;
    }
    let from_node = arr[0].as_str()?.to_string();
    let from_socket = arr[1].as_u64()? as usize;
    Some(Link {
        from_node,
        from_socket,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrongLink {
    pub from_node: String,
    pub from_socket: usize,
    pub to_node: String,
}

impl StrongLink {
    pub fn new(from_node: impl Into<String>, from_socket: usize, to_node: impl Into<String>) -> Self {
        Self {
            from_node: from_node.into(),
            from_socket,
            to_node: to_node.into(),
        }
    }
}
