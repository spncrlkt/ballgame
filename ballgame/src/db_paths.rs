//! Central database path configuration
//!
//! All database paths should be generated through this module to ensure
//! consistent naming and directory structure.

use chrono::Local;

/// Default database directory
pub const DB_DIR: &str = "db";

/// Database type for path generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbType {
    /// Training session database
    Training,
    /// Tournament simulation database
    Tournament,
    /// Bracket tournament database
    Bracket,
}

impl DbType {
    /// Get the filename prefix for this database type
    pub fn prefix(&self) -> &'static str {
        match self {
            DbType::Training => "training",
            DbType::Tournament => "tournament",
            DbType::Bracket => "bracket",
        }
    }
}

/// Ensure the database directory exists
pub fn ensure_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(DB_DIR)
}

/// Generate a timestamped database path: db/{prefix}_YYYYMMDD_HHMMSS.db
///
/// Each session gets a unique timestamped file to avoid overwriting previous data.
pub fn timestamped(db_type: DbType) -> String {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    format!("{}/{}_{}.db", DB_DIR, db_type.prefix(), timestamp)
}

/// Get the default path for a database type (for reading/replay)
///
/// Note: This returns a pattern path. For most uses, prefer timestamped()
/// for writing and let users specify exact paths for reading.
pub fn default_path(db_type: DbType) -> String {
    format!("{}/{}.db", DB_DIR, db_type.prefix())
}

/// Find the most recent database file of a given type
///
/// Searches the db directory for files matching the pattern and returns
/// the most recently modified one.
pub fn find_latest(db_type: DbType) -> Option<String> {
    let prefix = db_type.prefix();
    let pattern = format!("{}_", prefix);

    let entries: Vec<_> = std::fs::read_dir(DB_DIR)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with(&pattern) && n.ends_with(".db"))
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        return None;
    }

    // Sort by modification time, most recent first
    let mut entries_with_time: Vec<_> = entries
        .into_iter()
        .filter_map(|e| {
            let metadata = e.metadata().ok()?;
            let modified = metadata.modified().ok()?;
            Some((e.path(), modified))
        })
        .collect();

    entries_with_time.sort_by(|a, b| b.1.cmp(&a.1));

    entries_with_time
        .first()
        .map(|(path, _)| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_type_prefix() {
        assert_eq!(DbType::Training.prefix(), "training");
        assert_eq!(DbType::Tournament.prefix(), "tournament");
        assert_eq!(DbType::Bracket.prefix(), "bracket");
    }

    #[test]
    fn test_timestamped_format() {
        let path = timestamped(DbType::Training);
        assert!(path.starts_with("db/training_"));
        assert!(path.ends_with(".db"));
        // Should have format: db/training_YYYYMMDD_HHMMSS.db
        assert!(path.len() > "db/training_.db".len());
    }

    #[test]
    fn test_default_path() {
        assert_eq!(default_path(DbType::Training), "db/training.db");
        assert_eq!(default_path(DbType::Tournament), "db/tournament.db");
        assert_eq!(default_path(DbType::Bracket), "db/bracket.db");
    }
}
