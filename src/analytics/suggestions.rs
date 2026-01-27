//! Parameter change suggestions based on target deltas

use super::targets::{TargetDelta, TargetStatus};

/// A suggested parameter change
#[derive(Debug, Clone)]
pub struct ParameterSuggestion {
    /// Parameter name (e.g., "shot_max_power")
    pub parameter: String,
    /// Suggested change direction and magnitude
    pub change: String,
    /// Reason for the suggestion
    pub reason: String,
    /// Priority (1 = highest)
    pub priority: u8,
}

impl ParameterSuggestion {
    pub fn format(&self) -> String {
        format!(
            "  {}. {} {} ({})",
            self.priority, self.change, self.parameter, self.reason
        )
    }
}

/// Generate parameter suggestions from target deltas
pub fn generate_suggestions(deltas: &[TargetDelta]) -> Vec<ParameterSuggestion> {
    let mut suggestions = Vec::new();

    for delta in deltas {
        if delta.status == TargetStatus::Ok {
            continue;
        }

        let suggestion = match delta.name.as_str() {
            "avg score" => suggest_for_avg_score(delta),
            "score differential" => suggest_for_score_diff(delta),
            "match duration" => suggest_for_duration(delta),
            "turnovers per match" => suggest_for_turnovers(delta),
            "missed shots per match" => suggest_for_missed_shots(delta),
            _ => None,
        };

        if let Some(s) = suggestion {
            suggestions.push(s);
        }
    }

    // Sort by priority
    suggestions.sort_by_key(|s| s.priority);

    // Assign sequential priority numbers
    for (i, s) in suggestions.iter_mut().enumerate() {
        s.priority = (i + 1) as u8;
    }

    suggestions
}

fn suggest_for_avg_score(delta: &TargetDelta) -> Option<ParameterSuggestion> {
    let pct = delta.pct_diff;

    if pct < -10.0 {
        // Scores too low
        Some(ParameterSuggestion {
            parameter: "SHOT_MAX_POWER".to_string(),
            change: format!("Increase by ~{:.0}%", pct.abs() * 0.5),
            reason: "scores too low".to_string(),
            priority: 1,
        })
    } else if pct > 10.0 {
        // Scores too high
        Some(ParameterSuggestion {
            parameter: "SHOT_MAX_POWER".to_string(),
            change: format!("Decrease by ~{:.0}%", pct * 0.5),
            reason: "scores too high".to_string(),
            priority: 1,
        })
    } else {
        None
    }
}

fn suggest_for_score_diff(delta: &TargetDelta) -> Option<ParameterSuggestion> {
    if delta.actual > delta.target + delta.target * 0.5 {
        // Matches too one-sided
        Some(ParameterSuggestion {
            parameter: "AI profile balance".to_string(),
            change: "Review".to_string(),
            reason: "matches too one-sided, check profile effectiveness".to_string(),
            priority: 2,
        })
    } else {
        None
    }
}

fn suggest_for_duration(delta: &TargetDelta) -> Option<ParameterSuggestion> {
    let pct = delta.pct_diff;

    if pct < -15.0 {
        // Matches too short
        Some(ParameterSuggestion {
            parameter: "SHOT_MAX_VARIANCE".to_string(),
            change: format!("Increase by ~{:.0}%", pct.abs() * 0.3),
            reason: "matches ending too quickly".to_string(),
            priority: 3,
        })
    } else if pct > 15.0 {
        // Matches too long
        Some(ParameterSuggestion {
            parameter: "SHOT_MAX_SPEED".to_string(),
            change: format!("Increase by ~{:.0}%", pct * 0.3),
            reason: "matches taking too long".to_string(),
            priority: 3,
        })
    } else {
        None
    }
}

fn suggest_for_turnovers(delta: &TargetDelta) -> Option<ParameterSuggestion> {
    let pct = delta.pct_diff;

    if pct > 20.0 {
        // Too many turnovers
        Some(ParameterSuggestion {
            parameter: "STEAL_COOLDOWN".to_string(),
            change: format!("Increase from 0.3s to ~{:.1}s", 0.3 * (1.0 + pct / 100.0)),
            reason: "too many turnovers".to_string(),
            priority: 2,
        })
    } else if pct < -20.0 {
        // Too few turnovers
        Some(ParameterSuggestion {
            parameter: "STEAL_SUCCESS_CHANCE".to_string(),
            change: format!("Increase by ~{:.0}%", pct.abs() * 0.3),
            reason: "too few turnovers, game may feel static".to_string(),
            priority: 4,
        })
    } else {
        None
    }
}

fn suggest_for_missed_shots(delta: &TargetDelta) -> Option<ParameterSuggestion> {
    let pct = delta.pct_diff;

    if pct < -20.0 {
        // Too few missed shots (accuracy too high)
        Some(ParameterSuggestion {
            parameter: "SHOT_MIN_VARIANCE".to_string(),
            change: format!("Increase by ~{:.0}%", pct.abs() * 0.5),
            reason: "shots too accurate".to_string(),
            priority: 3,
        })
    } else if pct > 30.0 {
        // Too many missed shots
        Some(ParameterSuggestion {
            parameter: "SHOT_MAX_VARIANCE".to_string(),
            change: format!("Decrease by ~{:.0}%", pct * 0.3),
            reason: "shots too inaccurate".to_string(),
            priority: 3,
        })
    } else {
        None
    }
}

/// Format all suggestions as a report
pub fn format_suggestions(suggestions: &[ParameterSuggestion]) -> String {
    if suggestions.is_empty() {
        return "\nNo parameter changes suggested - targets are being met.\n".to_string();
    }

    let mut output = String::new();
    output.push_str("\nSUGGESTED CHANGES:\n");

    for s in suggestions {
        output.push_str(&s.format());
        output.push('\n');
    }

    output
}
