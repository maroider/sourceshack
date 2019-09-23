pub fn ensure_correct_path_separator(string: String) -> String {
    if std::path::MAIN_SEPARATOR != '/' {
        string.replace("/", "\\")
    } else {
        string
    }
}
