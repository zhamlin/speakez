use std::collections::HashMap;

pub fn types_from_json_schema(schemas: &[schemars::Schema]) -> String {
    let mut all_types = HashMap::new();
    for types in schemas.iter().map(get_typescript_types) {
        for (name, val) in types {
            all_types.entry(name).or_insert(val);
        }
    }

    let mut to_print: Vec<String> = all_types.into_values().collect();
    to_print.sort();
    to_print.join("\n")
}

fn get_typescript_types(schema: &schemars::Schema) -> HashMap<String, String> {
    let mut result = HashMap::new();

    let root_name = schema
        .get("title")
        .and_then(|v| v.as_str())
        .expect("root type should have title");

    let typ = json_schema_to_typescript(schema, root_name);
    result.insert(root_name.to_string(), typ);

    let mut defs = serde_json::Map::new();
    if let Some(defs_map) = schema.get("$defs").and_then(serde_json::Value::as_object) {
        defs = defs_map.clone();
    }

    for (typ_name, value) in &defs {
        let typ = json_schema_to_typescript(value.try_into().unwrap(), typ_name);
        result.insert(typ_name.to_string(), typ);
    }

    result
}

fn json_schema_to_typescript(schema: &schemars::Schema, root_name: &str) -> String {
    let mut typescript = String::new();

    if let Some(description) = schema.get("description").and_then(|v| v.as_str()) {
        typescript += "/**\n";
        typescript += &format!(" * {description}\n");
        typescript += " */\n";
    }

    typescript += &format!("export type {root_name} = ");

    process_schema(schema, &mut typescript, 0);
    typescript + ";"
}

fn process_schema(schema: &schemars::Schema, output: &mut String, indent_level: usize) {
    let indent = "  ".repeat(indent_level);

    if let Some(any_of) = schema
        .get("anyOf")
        .map(|v| v.as_array().expect("any_of should be an array"))
    {
        for obj in any_of
            .iter()
            .map(|v| v.as_object().expect("anyOf items should be objects"))
            .map(|v| -> schemars::Schema { v.clone().into() })
        {
            if let Some(typ) = obj
                .get("type")
                .map(|v| v.as_str().expect("type should be string in anyOf"))
            {
                if typ == "null" {
                    continue;
                }
                process_schema(&obj, output, indent_level);
            } else {
                process_schema(&obj, output, indent_level);
            }
        }
        return;
    }

    if let Some(one_of) = schema
        .get("oneOf")
        .map(|v| v.as_array().expect("oneOf should be an array"))
    {
        for (i, sub_schema) in one_of.iter().enumerate() {
            if i > 0 {
                *output += " | ";
            }
            process_schema(sub_schema.try_into().unwrap(), output, indent_level);
        }
        return;
    }

    if let Some(schema_type) = schema.get("type") {
        let mut nullable = false;
        let typ = if let Some(arr) = schema_type.as_array() {
            let mut t = "";
            for item in arr
                .iter()
                .map(|v| v.as_str().expect("type items should be a string"))
            {
                if item == "null" {
                    nullable = true
                } else {
                    t = item
                }
            }
            t
        } else {
            schema_type
                .as_str()
                .expect("type should be a string or an array")
        };

        match typ {
            "object" => {
                *output += &format!("{indent}{{\n");
                if let Some(properties) = schema
                    .get("properties")
                    .map(|o| o.as_object().expect("properties should be an object"))
                {
                    for (key, value) in properties {
                        let required = schema
                            .get("required")
                            .and_then(|r| r.as_array())
                            .map(|r| r.iter().any(|v| v.as_str() == Some(key)))
                            .unwrap_or(false);

                        if let Some(description) = value.get("description").and_then(|d| d.as_str())
                        {
                            *output += &format!("{indent}  /**\n");
                            *output += &format!("{indent}   * {description}\n");
                            *output += &format!("{indent}   */\n");
                        }

                        let key_optional = if required { "" } else { "?" };
                        *output += &format!("{indent}  {key}{key_optional}: ");
                        process_schema(value.try_into().unwrap(), output, indent_level + 1);
                        *output += ";\n";
                    }
                }
                *output += &format!("{indent}}}\n");
            }
            "array" => {
                if let Some(items) = schema.get("items") {
                    process_schema(items.try_into().unwrap(), output, indent_level);
                    *output += "[]";
                } else {
                    *output += "any[]";
                }
            }
            "string" => {
                if let Some(const_val) = schema
                    .get("const")
                    .map(|v| v.as_str().expect("string const should be a string"))
                {
                    *output += &format!("\"{const_val}\"");
                } else {
                    *output += "string";
                }
            }
            "number" => *output += "number",
            "integer" => *output += "number",
            "boolean" => *output += "boolean",
            "null" => *output += "null",
            _ => *output += "any",
        }
        return;
    }

    if let Some(reference) = schema.get("$ref").and_then(|r| r.as_str()) {
        if let Some(def_name) = reference.strip_prefix("#/$defs/") {
            *output += def_name;
        } else {
            *output += "any";
        }
        return;
    }

    if schema.as_value().is_array() {
        *output += "any[]";
    } else {
        *output += "any";
    }
}
