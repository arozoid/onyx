/// Returns a list of user extension commands: (name, description)
pub fn list_help() -> Vec<(String, String)> {
    vec![
        ("coming-soon".to_string(), "wait for a bit longer...".to_string()),
    ]
}