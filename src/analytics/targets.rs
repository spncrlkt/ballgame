//! Tuning target loading and comparison

use std::fs;
use std::path::Path;

use super::metrics::AggregateMetrics;

/// Status of a target comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetStatus {
    /// Within tolerance
    Ok,
    /// Outside tolerance but within 2x tolerance
    Warn,
    /// Way outside tolerance
    Fail,
}

impl TargetStatus {
    pub fn label(&self) -> &'static str {
        match self {
            TargetStatus::Ok => "OK",
            TargetStatus::Warn => "WARN",
            TargetStatus::Fail => "FAIL",
        }
    }
}

/// Single target with value and tolerance
#[derive(Debug, Clone)]
pub struct Target {
    pub name: String,
    pub target: f32,
    pub tolerance: f32,
}

impl Target {
    /// Check how actual compares to target
    pub fn check(&self, actual: f32) -> TargetDelta {
        let diff = actual - self.target;
        let abs_diff = diff.abs();
        let pct_diff = if self.target != 0.0 {
            (diff / self.target) * 100.0
        } else {
            0.0
        };

        let status = if abs_diff <= self.tolerance {
            TargetStatus::Ok
        } else if abs_diff <= self.tolerance * 2.0 {
            TargetStatus::Warn
        } else {
            TargetStatus::Fail
        };

        TargetDelta {
            name: self.name.clone(),
            target: self.target,
            actual,
            diff,
            pct_diff,
            status,
        }
    }
}

/// Result of comparing actual value to target
#[derive(Debug, Clone)]
pub struct TargetDelta {
    pub name: String,
    pub target: f32,
    pub actual: f32,
    pub diff: f32,
    pub pct_diff: f32,
    pub status: TargetStatus,
}

impl TargetDelta {
    pub fn format(&self) -> String {
        let sign = if self.pct_diff >= 0.0 { "+" } else { "" };
        format!(
            "  {:<22} {:>6.1}  (target: {:>5.1})  [{}: {}{}%]",
            self.name,
            self.actual,
            self.target,
            self.status.label(),
            sign,
            self.pct_diff as i32,
        )
    }
}

/// Collection of tuning targets
#[derive(Debug, Clone, Default)]
pub struct TuningTargets {
    pub avg_score: Option<Target>,
    pub score_differential: Option<Target>,
    pub match_duration: Option<Target>,
    pub turnovers_per_match: Option<Target>,
    pub missed_shots_per_match: Option<Target>,
}

impl TuningTargets {
    /// Compare metrics against targets
    pub fn compare(&self, metrics: &AggregateMetrics) -> Vec<TargetDelta> {
        let mut deltas = Vec::new();

        if let Some(t) = &self.avg_score {
            // Note: our avg_total_score is left+right, target is per-side average
            // So we compare half of total to target
            deltas.push(t.check(metrics.avg_total_score / 2.0));
        }

        if let Some(t) = &self.score_differential {
            deltas.push(t.check(metrics.avg_score_differential));
        }

        if let Some(t) = &self.match_duration {
            deltas.push(t.check(metrics.avg_duration));
        }

        if let Some(t) = &self.turnovers_per_match {
            deltas.push(t.check(metrics.avg_turnovers));
        }

        if let Some(t) = &self.missed_shots_per_match {
            deltas.push(t.check(metrics.avg_missed_shots));
        }

        deltas
    }

    /// Format comparison report
    pub fn format_report(&self, metrics: &AggregateMetrics) -> String {
        let deltas = self.compare(metrics);
        if deltas.is_empty() {
            return "No targets defined.\n".to_string();
        }

        let mut output = String::new();
        output.push_str("\nTARGETS vs ACTUAL:\n");

        for delta in &deltas {
            output.push_str(&delta.format());
            output.push('\n');
        }

        // Summary
        let ok_count = deltas.iter().filter(|d| d.status == TargetStatus::Ok).count();
        let warn_count = deltas.iter().filter(|d| d.status == TargetStatus::Warn).count();
        let fail_count = deltas.iter().filter(|d| d.status == TargetStatus::Fail).count();

        output.push_str(&format!(
            "\n  Summary: {} OK, {} WARN, {} FAIL\n",
            ok_count, warn_count, fail_count
        ));

        output
    }
}

/// Load tuning targets from a TOML file
pub fn load_targets(path: &Path) -> Option<TuningTargets> {
    let content = fs::read_to_string(path).ok()?;
    parse_targets_toml(&content)
}

/// Parse TOML content into TuningTargets
fn parse_targets_toml(content: &str) -> Option<TuningTargets> {
    let mut targets = TuningTargets::default();

    // Simple TOML parser for our format:
    // [targets]
    // avg_score = { target = 14.0, tolerance = 1.0 }

    let mut in_targets_section = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("[targets]") {
            in_targets_section = true;
            continue;
        }

        if line.starts_with('[') {
            in_targets_section = false;
            continue;
        }

        if !in_targets_section || line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse "key = { target = X, tolerance = Y }"
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            if let Some(target) = parse_target_value(key, value) {
                match key {
                    "avg_score" => targets.avg_score = Some(target),
                    "score_differential" => targets.score_differential = Some(target),
                    "match_duration_secs" | "match_duration" => targets.match_duration = Some(target),
                    "turnovers_per_match" => targets.turnovers_per_match = Some(target),
                    "missed_shots_per_match" => targets.missed_shots_per_match = Some(target),
                    _ => {}
                }
            }
        }
    }

    Some(targets)
}

/// Parse a single target value like "{ target = 14.0, tolerance = 1.0 }"
fn parse_target_value(name: &str, value: &str) -> Option<Target> {
    // Remove braces and split by comma
    let value = value.trim_start_matches('{').trim_end_matches('}');

    let mut target_val = None;
    let mut tolerance_val = None;

    for part in value.split(',') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            let k = k.trim();
            let v = v.trim();
            match k {
                "target" => target_val = v.parse().ok(),
                "tolerance" => tolerance_val = v.parse().ok(),
                _ => {}
            }
        }
    }

    Some(Target {
        name: name.replace('_', " "),
        target: target_val?,
        tolerance: tolerance_val.unwrap_or(1.0),
    })
}

/// Create default targets based on the plan's tuning goals
pub fn default_targets() -> TuningTargets {
    TuningTargets {
        avg_score: Some(Target {
            name: "avg score".to_string(),
            target: 14.0,
            tolerance: 1.0,
        }),
        score_differential: Some(Target {
            name: "score differential".to_string(),
            target: 2.0,
            tolerance: 1.0,
        }),
        match_duration: Some(Target {
            name: "match duration".to_string(),
            target: 180.0,
            tolerance: 15.0,
        }),
        turnovers_per_match: Some(Target {
            name: "turnovers per match".to_string(),
            target: 20.0,
            tolerance: 5.0,
        }),
        missed_shots_per_match: Some(Target {
            name: "missed shots per match".to_string(),
            target: 20.0,
            tolerance: 5.0,
        }),
    }
}
