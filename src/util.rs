use std::env;

pub fn ensure_correct_path_separator(string: String) -> String {
    if std::path::MAIN_SEPARATOR != '/' {
        string.replace("/", "\\")
    } else {
        string
    }
}

pub fn read_expected_env_var(name: &str) -> String {
    env::var(name).expect(&format!("{} should be set", name))
}
