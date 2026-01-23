/// Returns a list of user extension commands: (name, description)
pub fn list_help() -> Vec<(String, String)> {
    vec![
        ("ext_plugin1324".to_string(), "User extension plugin 1".to_string()),
        ("ext_plugin2".to_string(), "User extension plugin 2".to_string()),
    ]
}