//! Bracket tournament analysis and reporting
//!
//! Provides tools for analyzing bracket tournament results and generating reports.

use rusqlite::{Connection, Result, params};
use std::path::Path;

/// Bracket tournament summary
#[derive(Debug, Clone)]
pub struct BracketTournament {
    pub id: i64,
    pub session_id: String,
    pub format_best_of: u32,
    pub format_score_limit: u32,
    pub format_duration_limit: f32,
    pub seeding_method: String,
    pub entrant_count: u32,
    pub champion_profile: Option<String>,
    pub is_complete: bool,
}

/// Bracket entry with standings
#[derive(Debug, Clone)]
pub struct BracketStanding {
    pub entry_index: usize,
    pub profile_name: String,
    pub seed: u32,
    pub final_placement: Option<u32>,
    pub match_wins: u32,
    pub match_losses: u32,
    pub game_wins: u32,
    pub game_losses: u32,
}

impl BracketStanding {
    /// Calculate match win rate
    pub fn match_win_rate(&self) -> f32 {
        let total = self.match_wins + self.match_losses;
        if total == 0 {
            0.0
        } else {
            self.match_wins as f32 / total as f32
        }
    }

    /// Calculate game win rate
    pub fn game_win_rate(&self) -> f32 {
        let total = self.game_wins + self.game_losses;
        if total == 0 {
            0.0
        } else {
            self.game_wins as f32 / total as f32
        }
    }

    /// Game differential (wins - losses)
    pub fn game_differential(&self) -> i32 {
        self.game_wins as i32 - self.game_losses as i32
    }
}

/// Bracket match summary
#[derive(Debug, Clone)]
pub struct BracketMatchSummary {
    pub bracket_match_id: u32,
    pub side: String,
    pub round: u32,
    pub match_in_round: u32,
    pub player1_name: String,
    pub player2_name: String,
    pub player1_wins: u32,
    pub player2_wins: u32,
    pub winner_name: String,
}

/// Full bracket report
#[derive(Debug, Clone)]
pub struct BracketReport {
    pub tournament: BracketTournament,
    pub standings: Vec<BracketStanding>,
    pub matches: Vec<BracketMatchSummary>,
    pub total_games: u32,
    pub total_events: u64,
}

impl BracketReport {
    /// Generate markdown report
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Header
        md.push_str(&format!("# Bracket Tournament Report\n\n"));
        md.push_str(&format!("**Tournament ID:** {}\n\n", self.tournament.id));
        md.push_str(&format!(
            "**Format:** Best of {} (First to {})\n\n",
            self.tournament.format_best_of, self.tournament.format_score_limit
        ));
        md.push_str(&format!(
            "**Seeding:** {}\n\n",
            self.tournament.seeding_method
        ));
        md.push_str(&format!(
            "**Entrants:** {}\n\n",
            self.tournament.entrant_count
        ));

        if let Some(ref champion) = self.tournament.champion_profile {
            md.push_str(&format!("**Champion:** {}\n\n", champion));
        }

        md.push_str(&format!(
            "**Total Games:** {} ({} events logged)\n\n",
            self.total_games, self.total_events
        ));

        // Standings table
        md.push_str("## Final Standings\n\n");
        md.push_str("| Rank | Profile | Seed | Match W-L | Game W-L | Match% | Game% |\n");
        md.push_str("|------|---------|------|-----------|----------|--------|-------|\n");

        for standing in &self.standings {
            let rank = standing.final_placement.unwrap_or(0);
            md.push_str(&format!(
                "| {} | {} | {} | {}-{} | {}-{} | {:.1}% | {:.1}% |\n",
                rank,
                standing.profile_name,
                standing.seed,
                standing.match_wins,
                standing.match_losses,
                standing.game_wins,
                standing.game_losses,
                standing.match_win_rate() * 100.0,
                standing.game_win_rate() * 100.0,
            ));
        }

        // Match results by bracket side
        md.push_str("\n## Match Results\n\n");

        // Group matches by side
        let sides = ["Winners", "Losers", "GrandFinals", "GrandFinalsReset"];
        for side in &sides {
            let side_matches: Vec<_> = self.matches.iter().filter(|m| m.side == *side).collect();

            if side_matches.is_empty() {
                continue;
            }

            let display_side = match *side {
                "GrandFinals" => "Grand Finals",
                "GrandFinalsReset" => "Grand Finals Reset",
                _ => side,
            };
            md.push_str(&format!("### {} Bracket\n\n", display_side));
            md.push_str("| Round | Match | Player 1 | Score | Player 2 | Winner |\n");
            md.push_str("|-------|-------|----------|-------|----------|--------|\n");

            for m in side_matches {
                md.push_str(&format!(
                    "| {} | {} | {} | {}-{} | {} | {} |\n",
                    m.round,
                    m.match_in_round,
                    m.player1_name,
                    m.player1_wins,
                    m.player2_wins,
                    m.player2_name,
                    m.winner_name,
                ));
            }
            md.push_str("\n");
        }

        // Top performers summary
        md.push_str("## Top Performers\n\n");
        if !self.standings.is_empty() {
            let top3: Vec<_> = self.standings.iter().take(3).collect();
            for (i, s) in top3.iter().enumerate() {
                let medal = match i {
                    0 => "🥇",
                    1 => "🥈",
                    2 => "🥉",
                    _ => "",
                };
                md.push_str(&format!(
                    "{} **{}** - {}-{} matches ({:.1}%), {}-{} games ({:.1}%)\n\n",
                    medal,
                    s.profile_name,
                    s.match_wins,
                    s.match_losses,
                    s.match_win_rate() * 100.0,
                    s.game_wins,
                    s.game_losses,
                    s.game_win_rate() * 100.0,
                ));
            }
        }

        md
    }

    /// Export standings to a simple rankings file (one profile per line)
    pub fn export_rankings(&self, path: &Path) -> std::io::Result<()> {
        let mut content = String::new();
        content.push_str(&format!(
            "# Bracket Rankings (Tournament {})\n",
            self.tournament.id
        ));
        content.push_str(&format!(
            "# Generated: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
        content.push_str(&format!(
            "# Format: BO{} FT{}\n",
            self.tournament.format_best_of, self.tournament.format_score_limit
        ));
        content.push_str("#\n");
        content.push_str("# Rank, Profile, MatchW, MatchL, GameW, GameL\n");

        for standing in &self.standings {
            let rank = standing.final_placement.unwrap_or(0);
            content.push_str(&format!(
                "{},{},{},{},{},{}\n",
                rank,
                standing.profile_name,
                standing.match_wins,
                standing.match_losses,
                standing.game_wins,
                standing.game_losses,
            ));
        }

        std::fs::write(path, content)
    }
}

/// Load the most recent bracket tournament from the database
pub fn load_latest_bracket(db_path: &Path) -> Result<Option<BracketReport>> {
    let conn = Connection::open(db_path)?;

    // Get most recent tournament
    let tournament_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM bracket_tournaments ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();

    match tournament_id {
        Some(id) => load_bracket_tournament(&conn, id).map(Some),
        None => Ok(None),
    }
}

/// Load a specific bracket tournament by ID
pub fn load_bracket_tournament(conn: &Connection, tournament_id: i64) -> Result<BracketReport> {
    // Load tournament metadata
    let tournament = conn.query_row(
        r#"SELECT id, session_id, format_best_of, format_score_limit, format_duration_limit,
                  seeding_method, entrant_count, champion_profile, is_complete
           FROM bracket_tournaments WHERE id = ?1"#,
        params![tournament_id],
        |row| {
            Ok(BracketTournament {
                id: row.get(0)?,
                session_id: row.get(1)?,
                format_best_of: row.get(2)?,
                format_score_limit: row.get(3)?,
                format_duration_limit: row.get(4)?,
                seeding_method: row.get(5)?,
                entrant_count: row.get(6)?,
                champion_profile: row.get(7)?,
                is_complete: row.get::<_, i32>(8)? != 0,
            })
        },
    )?;

    // Load standings - sorted by match wins desc, then game wins desc
    let mut stmt = conn.prepare(
        r#"SELECT entry_index, profile_name, seed, final_placement,
                  match_wins, match_losses, game_wins, game_losses
           FROM bracket_entries
           WHERE tournament_id = ?1
           ORDER BY match_wins DESC, game_wins DESC, game_losses ASC"#,
    )?;

    let standings: Vec<BracketStanding> = stmt
        .query_map(params![tournament_id], |row| {
            Ok(BracketStanding {
                entry_index: row.get::<_, i64>(0)? as usize,
                profile_name: row.get(1)?,
                seed: row.get(2)?,
                final_placement: row.get(3)?,
                match_wins: row.get(4)?,
                match_losses: row.get(5)?,
                game_wins: row.get(6)?,
                game_losses: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    // Load match summaries with player names
    let mut stmt = conn.prepare(
        r#"SELECT bm.bracket_match_id, bm.side, bm.round, bm.match_in_round,
                  p1.profile_name, p2.profile_name, bm.player1_wins, bm.player2_wins, bm.winner_idx
           FROM bracket_matches bm
           LEFT JOIN bracket_entries p1 ON p1.tournament_id = bm.tournament_id AND p1.entry_index = bm.player1_entry_idx
           LEFT JOIN bracket_entries p2 ON p2.tournament_id = bm.tournament_id AND p2.entry_index = bm.player2_entry_idx
           WHERE bm.tournament_id = ?1
           ORDER BY bm.id"#,
    )?;

    let matches: Vec<BracketMatchSummary> = stmt
        .query_map(params![tournament_id], |row| {
            let p1_name: String = row
                .get::<_, Option<String>>(4)?
                .unwrap_or_else(|| "BYE".to_string());
            let p2_name: String = row
                .get::<_, Option<String>>(5)?
                .unwrap_or_else(|| "BYE".to_string());
            let p1_wins: u32 = row.get(6)?;
            let p2_wins: u32 = row.get(7)?;
            let winner_idx: Option<i64> = row.get(8)?;

            // Determine winner name from winner_idx
            let winner_name = match winner_idx {
                Some(_) => {
                    // winner_idx is the entry_index, need to match it to p1 or p2
                    // This is a simplification - we check which player won more games
                    if p1_wins > p2_wins {
                        p1_name.clone()
                    } else {
                        p2_name.clone()
                    }
                }
                None => "TBD".to_string(),
            };

            Ok(BracketMatchSummary {
                bracket_match_id: row.get(0)?,
                side: row.get(1)?,
                round: row.get(2)?,
                match_in_round: row.get(3)?,
                player1_name: p1_name,
                player2_name: p2_name,
                player1_wins: p1_wins,
                player2_wins: p2_wins,
                winner_name,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    // Count total games and events
    let total_games: u32 = conn.query_row(
        "SELECT COUNT(*) FROM bracket_games WHERE bracket_match_id IN (SELECT id FROM bracket_matches WHERE tournament_id = ?1)",
        params![tournament_id],
        |row| row.get(0),
    )?;

    let total_events: u64 = conn.query_row(
        "SELECT COUNT(*) FROM events WHERE match_id IN (SELECT match_id FROM bracket_games WHERE bracket_match_id IN (SELECT id FROM bracket_matches WHERE tournament_id = ?1))",
        params![tournament_id],
        |row| row.get(0),
    )?;

    Ok(BracketReport {
        tournament,
        standings,
        matches,
        total_games,
        total_events,
    })
}

/// Run bracket analysis and generate report
pub fn run_bracket_analysis(
    db_path: &Path,
    output_dir: &Path,
    rankings_file: Option<&Path>,
) -> Result<BracketReport> {
    let report =
        load_latest_bracket(db_path)?.ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;

    // Create output directory
    std::fs::create_dir_all(output_dir).ok();

    // Write markdown report
    let report_path = output_dir.join(format!(
        "bracket_report_{}.md",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ));
    std::fs::write(&report_path, report.to_markdown())
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    println!("Bracket report written to {}", report_path.display());

    // Export rankings if path provided
    if let Some(rankings_path) = rankings_file {
        report
            .export_rankings(rankings_path)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        println!("Rankings exported to {}", rankings_path.display());
    }

    Ok(report)
}

/// List all bracket tournaments in the database
pub fn list_bracket_tournaments(db_path: &Path) -> Result<Vec<BracketTournament>> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        r#"SELECT id, session_id, format_best_of, format_score_limit, format_duration_limit,
                  seeding_method, entrant_count, champion_profile, is_complete
           FROM bracket_tournaments
           ORDER BY id DESC"#,
    )?;

    let tournaments = stmt
        .query_map([], |row| {
            Ok(BracketTournament {
                id: row.get(0)?,
                session_id: row.get(1)?,
                format_best_of: row.get(2)?,
                format_score_limit: row.get(3)?,
                format_duration_limit: row.get(4)?,
                seeding_method: row.get(5)?,
                entrant_count: row.get(6)?,
                champion_profile: row.get(7)?,
                is_complete: row.get::<_, i32>(8)? != 0,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    Ok(tournaments)
}
