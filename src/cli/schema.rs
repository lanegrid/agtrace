use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct JsonSchemaNode {
    /// Primitive/aggregate types observed at this node: null, bool, number, string, object, array
    pub types: BTreeSet<String>,
    /// For object types: union of observed fields
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub fields: BTreeMap<String, JsonSchemaNode>,
    /// For array types: merged schema of all items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchemaNode>>,
}

impl JsonSchemaNode {
    fn record_type(&mut self, t: &str) {
        self.types.insert(t.to_string());
    }

    fn update_with_value(&mut self, value: &Value) {
        match value {
            Value::Null => {
                self.record_type("null");
            }
            Value::Bool(_) => {
                self.record_type("bool");
            }
            Value::Number(_) => {
                self.record_type("number");
            }
            Value::String(_) => {
                self.record_type("string");
            }
            Value::Array(arr) => {
                self.record_type("array");
                if arr.is_empty() {
                    return;
                }
                let items_schema = self.items.get_or_insert_with(|| Box::new(JsonSchemaNode::default()));
                for item in arr {
                    items_schema.update_with_value(item);
                }
            }
            Value::Object(map) => {
                self.record_type("object");
                for (k, v) in map {
                    let field_schema = self.fields.entry(k.clone()).or_insert_with(JsonSchemaNode::default);
                    field_schema.update_with_value(v);
                }
            }
        }
    }
}

pub fn cmd_schema(agent: String, path: Option<PathBuf>) -> Result<()> {
    let mut schema = JsonSchemaNode::default();

    match agent.as_str() {
        "claude-code" => build_claude_schema(&mut schema, path)?,
        "codex" => build_codex_schema(&mut schema, path)?,
        other => return Err(Error::UnknownAgent(other.to_string())),
    }

    serde_json::to_writer_pretty(std::io::stdout(), &schema).map_err(|e| {
        Error::Parse(format!("Failed to serialize schema as JSON: {}", e))
    })?;
    println!();

    Ok(())
}

fn build_claude_schema(root: &mut JsonSchemaNode, path: Option<PathBuf>) -> Result<()> {
    let dir = if let Some(path) = path {
        path
    } else {
        let home = home::home_dir()
            .ok_or_else(|| Error::Parse("Could not find home directory".to_string()))?;
        home.join(".claude").join("projects")
    };

    if !dir.exists() {
        return Err(Error::AgentDataNotFound(dir));
    }

    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().and_then(|s| s.to_str()) == Some("jsonl"))
    {
        let path = entry.path();
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&line).map_err(|e| {
                Error::Parse(format!(
                    "Failed to parse JSON in {}: {}",
                    path.display(),
                    e
                ))
            })?;
            root.update_with_value(&value);
        }
    }

    Ok(())
}

fn build_codex_schema(root: &mut JsonSchemaNode, path: Option<PathBuf>) -> Result<()> {
    let dir = if let Some(path) = path {
        path
    } else {
        let home = home::home_dir()
            .ok_or_else(|| Error::Parse("Could not find home directory".to_string()))?;
        home.join(".codex").join("sessions")
    };

    if !dir.exists() {
        return Err(Error::AgentDataNotFound(dir));
    }

    for year_entry in std::fs::read_dir(&dir)? {
        let year_entry = year_entry?;
        let year_path = year_entry.path();
        if !year_path.is_dir() {
            continue;
        }

        for month_entry in std::fs::read_dir(&year_path)? {
            let month_entry = month_entry?;
            let month_path = month_entry.path();
            if !month_path.is_dir() {
                continue;
            }

            for day_entry in std::fs::read_dir(&month_path)? {
                let day_entry = day_entry?;
                let day_path = day_entry.path();
                if !day_path.is_dir() {
                    continue;
                }

                for file_entry in std::fs::read_dir(&day_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();
                    if !file_path.is_file()
                        || file_path
                            .extension()
                            .and_then(|s| s.to_str())
                            != Some("jsonl")
                    {
                        continue;
                    }

                    let file = std::fs::File::open(&file_path)?;
                    let reader = std::io::BufReader::new(file);

                    for line in reader.lines() {
                        let line = line?;
                        if line.trim().is_empty() {
                            continue;
                        }

                        let value: Value = serde_json::from_str(&line).map_err(|e| {
                            Error::Parse(format!(
                                "Failed to parse JSON in {}: {}",
                                file_path.display(),
                                e
                            ))
                        })?;
                        root.update_with_value(&value);
                    }
                }
            }
        }
    }

    Ok(())
}

