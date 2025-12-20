use agtrace_index::LogFileRecord;
use std::path::Path;

pub fn should_skip_indexed_file(indexed: &LogFileRecord) -> bool {
    let path = Path::new(&indexed.path);

    // File doesn't exist anymore - don't skip (will be removed from index)
    if !path.exists() {
        return false;
    }

    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false, // Error reading metadata - rescan
    };

    // Compare file size
    if let Some(db_size) = indexed.file_size {
        if db_size != metadata.len() as i64 {
            return false; // Size changed - rescan
        }
    } else {
        return false; // No size in DB - rescan
    }

    // Compare mod time
    if let Some(db_mod_time) = &indexed.mod_time {
        if let Ok(fs_mod_time) = metadata.modified() {
            let fs_mod_time_str = format!("{:?}", fs_mod_time);
            if db_mod_time != &fs_mod_time_str {
                return false; // Mod time changed - rescan
            }
        } else {
            return false; // Can't read mod time - rescan
        }
    } else {
        return false; // No mod time in DB - rescan
    }

    // File unchanged - skip
    true
}
