use crate::graph::node::IoType;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Type mismatch: received {received} but expected {expected}")]
    TypeMismatch {
        received: String,
        expected: String,
    },
    #[error("Missing required input '{input_name}' on node '{node_id}'")]
    MissingRequiredInput {
        node_id: String,
        input_name: String,
    },
    #[error("Unknown input type: {type_name}")]
    UnknownType {
        type_name: String,
    },
}

pub fn validate_node_input(received_type: &str, input_type: &str, strict: bool) -> bool {
    if !received_type_ne(input_type, received_type) {
        return true;
    }

    if received_type == IoType::Any.io_type_str() || input_type == IoType::Any.io_type_str() {
        return true;
    }

    if !received_type.is_char_boundary(0) || !input_type.is_char_boundary(0) {
        return false;
    }

    let received_types: std::collections::HashSet<&str> =
        received_type.split(',').map(|t| t.trim()).collect();
    let input_types: std::collections::HashSet<&str> =
        input_type.split(',').map(|t| t.trim()).collect();

    if input_types.contains("*") || received_types.contains("*") {
        return true;
    }

    if strict {
        received_types.is_subset(&input_types)
    } else {
        !received_types.is_disjoint(&input_types)
    }
}

fn received_type_ne(a: &str, b: &str) -> bool {
    a != b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(validate_node_input("STRING", "STRING", false));
        assert!(validate_node_input("STRING", "STRING", true));
    }

    #[test]
    fn test_any_type() {
        assert!(validate_node_input("*", "STRING", false));
        assert!(validate_node_input("STRING", "*", false));
        assert!(validate_node_input("*", "STRING", true));
        assert!(validate_node_input("STRING", "*", true));
    }

    #[test]
    fn test_union_type_non_strict() {
        assert!(validate_node_input("STRING,BOOLEAN", "STRING,INT", false));
        assert!(validate_node_input("STRING", "STRING,INT", false));
    }

    #[test]
    fn test_union_type_strict() {
        assert!(validate_node_input("STRING", "STRING,INT", true));
        assert!(!validate_node_input("STRING,BOOLEAN", "STRING,INT", true));
    }

    #[test]
    fn test_no_overlap() {
        assert!(!validate_node_input("STRING", "INT", false));
        assert!(!validate_node_input("STRING", "INT", true));
    }

    #[test]
    fn test_combo_type() {
        assert!(validate_node_input("COMBO", "COMBO", false));
    }
}
