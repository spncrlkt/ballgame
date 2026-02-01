//! Load replay data from SQLite.

use std::path::Path;

use crate::replay::ReplayData;
use crate::simulation::SimDatabase;

pub fn load_replay_from_db(db_path: &Path, match_id: i64) -> Result<ReplayData, String> {
    let db = SimDatabase::open(db_path).map_err(|e| e.to_string())?;
    db.load_replay_data(match_id)
}
