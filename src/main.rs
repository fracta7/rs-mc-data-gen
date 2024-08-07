use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;

fn parse_recipe(file_path: &str) -> Option<(String, u32, HashMap<String, u32>, String)> {
    let data = fs::read_to_string(file_path).ok()?;
    let json: Value = serde_json::from_str(&data).ok()?;
    let r#type = json["type"].as_str()?.replace("minecraft:", "");
    let result_id = json["result"]["id"].as_str()?.replace("minecraft:", "");
    let result_quantity = json["result"]["count"].as_u64().unwrap_or(1) as u32;

    let mut requirements = HashMap::new();

    match r#type.as_str() {
        "crafting_shaped" => {
            let key = json["key"].as_object()?;
            let pattern = json["pattern"].as_array()?;
            for row in pattern {
                for c in row.as_str().unwrap_or("").chars() {
                    if let Some(ingredient) = key.get(&c.to_string()) {
                        if let Some(item_id) = ingredient.get("item").and_then(Value::as_str) {
                            let item_id = item_id.replace("minecraft:", "");
                            *requirements.entry(item_id).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
        "crafting_shapeless" => {
            let ingredients = json["ingredients"].as_array()?;
            for ingredient in ingredients {
                if let Some(item_id) = ingredient.get("item").and_then(Value::as_str) {
                    let item_id = item_id.replace("minecraft:", "");
                    *requirements.entry(item_id).or_insert(0) += 1;
                }
            }
        }
        "smoking" | "smelting" | "blasting" => {
            if let Some(ingredient) = json.get("ingredient").and_then(Value::as_object) {
                if let Some(item_id) = ingredient.get("item").and_then(Value::as_str) {
                    let item_id = item_id.replace("minecraft:", "");
                    *requirements.entry(item_id).or_insert(0) += 1;
                }
            }
            *requirements.entry("fuel".to_string()).or_insert(0) += 1;
        }
        "campfire_cooking" => {
            if let Some(ingredient) = json.get("ingredient").and_then(Value::as_object) {
                if let Some(item_id) = ingredient.get("item").and_then(Value::as_str) {
                    let item_id = item_id.replace("minecraft:", "");
                    *requirements.entry(item_id).or_insert(0) += 1;
                }
            }
        }
        "smithing_transform" => {
            let addition = json["addition"].as_object()?;
            let base = json["base"].as_object()?;
            let template = json["template"].as_object()?;
            if let Some(item_id) = addition.get("item").and_then(Value::as_str) {
                *requirements.entry(item_id.replace("minecraft:", "")).or_insert(0) += 1;
            }
            if let Some(item_id) = base.get("item").and_then(Value::as_str) {
                *requirements.entry(item_id.replace("minecraft:", "")).or_insert(0) += 1;
            }
            if let Some(item_id) = template.get("item").and_then(Value::as_str) {
                *requirements.entry(item_id.replace("minecraft:", "")).or_insert(0) += 1;
            }
        }
        "stonecutting" => {
            if let Some(ingredient) = json.get("ingredient").and_then(Value::as_object) {
                if let Some(item_id) = ingredient.get("item").and_then(Value::as_str) {
                    let item_id = item_id.replace("minecraft:", "");
                    *requirements.entry(item_id).or_insert(0) += 1;
                }
            }
        }
        _ => return None,
    }

    Some((result_id, result_quantity, requirements, r#type))
}

fn generate_kotlin_code(recipes: Vec<(String, u32, HashMap<String, u32>, String)>) -> String {
    let mut code = String::new();
    code.push_str("package com.fracta7.crafter.data.repository\n\n");
    code.push_str("import com.fracta7.crafter.domain.model.Recipe\n\n");
    code.push_str("/**\n * Initiates all recipes\n * @return List of Recipes.\n */\n");
    code.push_str("fun recipesInit(): List<Recipe> {\n");
    code.push_str("    return listOf(\n");

    for (result_id, result_quantity, requirements, recipe_type) in recipes {
        let requirements_str = requirements.iter()
            .map(|(id, qty)| format!("\"{}\" to {}", id, qty))
            .collect::<Vec<_>>()
            .join(", ");
        code.push_str(&format!(
            "        Recipe(result = \"{}\", resultQuantity = {}, requirements = mapOf({}), recipeType = \"{}\"),\n",
            result_id,
            result_quantity,
            requirements_str,
            recipe_type
        ));
    }

    code.push_str("    )\n");
    code.push_str("}\n");

    code
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let recipe_files = fs::read_dir("recipe")?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let recipes: Vec<_> = recipe_files.into_iter()
        .filter_map(|file_path| parse_recipe(&file_path))
        .collect();

    let kotlin_code = generate_kotlin_code(recipes);

    let mut file = File::create("Recipes.kt")?;
    file.write_all(kotlin_code.as_bytes())?;

    println!("Kotlin file generated successfully.");

    Ok(())
}
