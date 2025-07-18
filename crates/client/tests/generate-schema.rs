#[cfg(feature = "jsonschema")]
#[test]
fn generate_typescript_types() {
    use schemars::schema_for;
    let types = vec![
        schema_for!(speakez_client::commands::Response),
        schema_for!(speakez::common::events::Event),
    ];

    let new_text = gen::typescript::types_from_json_schema(&types);
    let types_path = std::path::Path::new("./schemas/types.d.ts");
    gen::file_matches(&types_path, &new_text);
}
