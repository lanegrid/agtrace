use crate::error::{Error, Result};
use serde_json::Value;
use std::io::{BufRead, Write};
use std::path::PathBuf;

pub fn cmd_scrub(input: PathBuf, output: PathBuf) -> Result<()> {
    if !input.exists() {
        return Err(Error::AgentDataNotFound(input));
    }

    let infile = std::fs::File::open(&input)?;
    let reader = std::io::BufReader::new(infile);

    if let Some(parent) = output.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let outfile = std::fs::File::create(&output)?;
    let mut writer = std::io::BufWriter::new(outfile);

    for (idx, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| {
            Error::Parse(format!(
                "Failed to read line {} in {}: {}",
                idx + 1,
                input.display(),
                e
            ))
        })?;

        if line.trim().is_empty() {
            continue;
        }

        let mut value: Value = serde_json::from_str(&line).map_err(|e| {
            Error::Parse(format!(
                "Failed to parse JSON in {} line {}: {}",
                input.display(),
                idx + 1,
                e
            ))
        })?;

        scrub_value(&mut value);

        serde_json::to_writer(&mut writer, &value).map_err(|e| {
            Error::Parse(format!(
                "Failed to write scrubbed JSON for {} line {}: {}",
                input.display(),
                idx + 1,
                e
            ))
        })?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;

    println!(
        "Scrubbed fixture written from {} to {}",
        input.display(),
        output.display()
    );

    Ok(())
}

fn scrub_value(value: &mut Value) {
    match value {
        Value::Null => {}
        Value::Bool(b) => {
            // Normalize all booleans to false to avoid leaking intent
            *b = false;
        }
        Value::Number(_) => {
            // Normalize all numbers to 0
            *value = Value::from(0);
        }
        Value::String(s) => {
            // Replace all strings with a generic dummy marker
            if s.is_empty() {
                *s = "DUMMY".to_string();
            } else if s.len() <= 16 {
                *s = "DUMMY".to_string();
            } else {
                *s = "DUMMY_TEXT".to_string();
            }
        }
        Value::Array(arr) => {
            // Keep at most the first element to preserve shape, but scrub it
            if !arr.is_empty() {
                let mut first = arr[0].clone();
                scrub_value(&mut first);
                arr.clear();
                arr.push(first);
            }
        }
        Value::Object(map) => {
            // Recurse into all fields; we don't remove keys so schema stays intact
            for (_k, v) in map.iter_mut() {
                scrub_value(v);
            }
        }
    }
}
