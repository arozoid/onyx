/// Returns a list of user extension commands: (name, description)
pub fn list_help() -> Vec<(String, String)> {
    vec![
        ("server".to_string(), "User extension plugin 1".to_string()),
        ("fs".to_string(), "User extension plugin 2".to_string()),
    ]
}