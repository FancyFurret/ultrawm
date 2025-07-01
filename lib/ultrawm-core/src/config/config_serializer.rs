use crate::config::config::{Config, ModTransformBindings, ResizeHandleBindings};
use schemars::{schema_for, Schema};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};

pub fn serialize_config(config: &Config, path: &str) -> io::Result<()> {
    let yaml_string = serde_yaml::to_string(config).unwrap();
    let mut field_docs = HashMap::new();
    let config_schema = schema_for!(Config);
    extract_field_documentation(&config_schema, "", &mut field_docs);

    let resize_handle_schema = schema_for!(ResizeHandleBindings);
    extract_field_documentation(
        &resize_handle_schema,
        "resize_handle_bindings",
        &mut field_docs,
    );

    let mod_transform_schema = schema_for!(ModTransformBindings);
    extract_field_documentation(
        &mod_transform_schema,
        "mod_transform_bindings",
        &mut field_docs,
    );

    let mut output = String::new();
    output.push_str("# UltraWM Configuration File\n");
    output.push_str("# Changes will take effect immediately\n\n\n");
    output.push_str(&add_comments_to_yaml(&yaml_string, &field_docs));

    let mut file = File::create(path)?;
    file.write_all(output.as_bytes())?;
    Ok(())
}

fn extract_field_documentation(
    schema: &Schema,
    prefix: &str,
    field_docs: &mut HashMap<String, String>,
) {
    if let Some(schema_obj) = schema.as_object() {
        if let Some(properties) = schema_obj.get("properties") {
            if let Some(props_obj) = properties.as_object() {
                for (key, prop_value) in props_obj {
                    let field_path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };

                    // Check if this property has a description
                    if let Some(prop_obj) = prop_value.as_object() {
                        if let Some(description) = prop_obj.get("description") {
                            if let Some(desc_str) = description.as_str() {
                                field_docs.insert(field_path.clone(), desc_str.to_string());
                            }
                        }

                        // Recursively process nested objects
                        if prop_obj.contains_key("properties") {
                            // Create a Schema from the property value for recursion
                            if let Ok(nested_schema) = Schema::try_from(prop_value.clone()) {
                                extract_field_documentation(
                                    &nested_schema,
                                    &field_path,
                                    field_docs,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

fn add_comments_to_yaml(yaml: &str, field_docs: &HashMap<String, String>) -> String {
    let lines: Vec<&str> = yaml.lines().collect();
    let mut result = Vec::new();
    let mut field_path_stack = Vec::new();

    for line in lines {
        if let Some(colon_pos) = line.find(':') {
            let before_colon = &line[..colon_pos];
            let field_name = before_colon.trim();

            let indent_level = (before_colon.len() - before_colon.trim_start().len()) / 2;
            let indent = "  ".repeat(indent_level);
            field_path_stack.truncate(indent_level);
            field_path_stack.push(field_name.to_string());
            let full_field_path = field_path_stack.join(".");

            let mut found_doc = None;

            if let Some(doc) = field_docs.get(&full_field_path) {
                found_doc = Some(doc);
            } else if let Some(doc) = field_docs.get(field_name) {
                found_doc = Some(doc);
            }

            if let Some(doc) = found_doc {
                if !result.is_empty() {
                    result.push(String::new());
                }
                result.push(format!("{}# {}", indent, doc));
            }
        }

        result.push(line.to_string());
    }

    result.join("\n")
}
