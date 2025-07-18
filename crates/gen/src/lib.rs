pub mod enums;
pub mod typescript;

pub fn file_matches(path: &std::path::Path, new_text: &str) {
    let original_text = {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => "".to_string(),
            Err(e) => panic!("reading `{path:?}` failed: {e}"),
        }
    };

    if original_text != new_text {
        std::fs::write(path, new_text).unwrap();
        panic!("file was not up-to-date: {path:?}")
    }
}

pub fn generate_json_schema(schemas: &[schemars::Schema]) -> serde_json::Result<String> {
    let schemas = schemas
        .iter()
        .map(serde_json::to_string_pretty)
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    Ok(format!("[{schemas}]"))
}
