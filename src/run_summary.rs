//! Unified run summary for all binaries
//!
//! Provides consistent end-of-run summary output with:
//! - Visual box formatting to draw attention
//! - List of all files created/modified
//! - Contextual "next steps" suggestions for workflow chaining

use std::time::Duration;

/// Category for output files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCategory {
    /// Database files (.db)
    Database,
    /// Report/markdown files
    Report,
    /// Image files (.png, .jpg)
    Image,
    /// Data files (.txt, .json, .csv)
    Data,
    /// Config files
    Config,
}

impl FileCategory {
    /// Get the display tag for this category
    pub fn tag(&self) -> &'static str {
        match self {
            FileCategory::Database => "[DB]",
            FileCategory::Report => "[REPORT]",
            FileCategory::Image => "[IMG]",
            FileCategory::Data => "[DATA]",
            FileCategory::Config => "[CFG]",
        }
    }
}

/// Priority level for next step suggestions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextStepPriority {
    /// Primary suggestion (shown with arrow)
    Primary,
    /// Secondary suggestion (shown with bullet)
    Secondary,
}

impl NextStepPriority {
    /// Get the bullet character for this priority
    pub fn bullet(&self) -> char {
        match self {
            NextStepPriority::Primary => '\u{2192}',   // →
            NextStepPriority::Secondary => '\u{00B7}', // ·
        }
    }
}

/// Entry for a created/modified file
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// File path (relative preferred)
    pub path: String,
    /// Optional description
    pub description: Option<String>,
    /// Category for display tag
    pub category: FileCategory,
}

impl FileEntry {
    /// Create a new file entry
    pub fn new(path: impl Into<String>, category: FileCategory) -> Self {
        Self {
            path: path.into(),
            description: None,
            category,
        }
    }

    /// Add a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// A suggested next step
#[derive(Debug, Clone)]
pub struct NextStep {
    /// Command to run
    pub command: String,
    /// Description of what it does
    pub description: String,
    /// Priority level
    pub priority: NextStepPriority,
}

impl NextStep {
    /// Create a primary next step
    pub fn primary(command: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            description: description.into(),
            priority: NextStepPriority::Primary,
        }
    }

    /// Create a secondary next step
    pub fn secondary(command: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            description: description.into(),
            priority: NextStepPriority::Secondary,
        }
    }
}

/// Run summary with builder pattern
#[derive(Debug, Clone, Default)]
pub struct RunSummary {
    /// Title displayed in header
    pub title: String,
    /// Optional description below title
    pub description: Option<String>,
    /// Key-value stats to display
    pub stats: Vec<(String, String)>,
    /// Files created during the run
    pub files_created: Vec<FileEntry>,
    /// Suggested next steps
    pub next_steps: Vec<NextStep>,
    /// Run duration
    pub duration: Option<Duration>,
}

/// Box drawing constants
const BOX_WIDTH: usize = 80;
const TOP_LEFT: char = '\u{2554}'; // ╔
const TOP_RIGHT: char = '\u{2557}'; // ╗
const BOTTOM_LEFT: char = '\u{255A}'; // ╚
const BOTTOM_RIGHT: char = '\u{255D}'; // ╝
const HORIZONTAL: char = '\u{2550}'; // ═
const VERTICAL: char = '\u{2551}'; // ║
const T_LEFT: char = '\u{2560}'; // ╠
const T_RIGHT: char = '\u{2563}'; // ╣
const THIN_HORIZONTAL: char = '\u{2500}'; // ─

impl RunSummary {
    /// Create a new run summary with a title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Set the description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a stat (key-value pair)
    pub fn stat(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.stats.push((key.into(), value.into()));
        self
    }

    /// Add a file entry
    pub fn file(mut self, entry: FileEntry) -> Self {
        self.files_created.push(entry);
        self
    }

    /// Add a file with just path and category
    pub fn file_simple(mut self, path: impl Into<String>, category: FileCategory) -> Self {
        self.files_created.push(FileEntry::new(path, category));
        self
    }

    /// Add a next step
    pub fn next_step(mut self, step: NextStep) -> Self {
        self.next_steps.push(step);
        self
    }

    /// Set the duration
    pub fn duration(mut self, dur: Duration) -> Self {
        self.duration = Some(dur);
        self
    }

    /// Print the summary to stdout
    pub fn print(&self) {
        println!();

        // Top border
        print_border(TOP_LEFT, TOP_RIGHT);

        // Title (centered)
        print_centered(&self.title.to_uppercase());

        // Description if present
        if let Some(ref desc) = self.description {
            print_line(desc);
        }

        // Stats section
        if !self.stats.is_empty() || self.duration.is_some() {
            print_separator();

            // Duration first if present
            if let Some(dur) = self.duration {
                let duration_str = format_duration(dur);
                print_line(&format!("Duration: {}", duration_str));
            }

            // Other stats
            for (key, value) in &self.stats {
                print_line(&format!("{}: {}", key, value));
            }
        }

        // Files section
        if !self.files_created.is_empty() {
            print_separator();
            print_line("FILES CREATED");
            print_line_chars(THIN_HORIZONTAL, 13);

            for file in &self.files_created {
                let tag = file.category.tag();
                let line = format!("{:<8} {}", tag, file.path);
                print_line(&line);
            }
        }

        // Next steps section
        if !self.next_steps.is_empty() {
            print_separator();
            print_line("NEXT STEPS");
            print_line_chars(THIN_HORIZONTAL, 10);

            for (i, step) in self.next_steps.iter().enumerate() {
                if i > 0 {
                    print_empty_line();
                }
                let bullet = step.priority.bullet();
                print_line(&format!("{} {}", bullet, step.command));
                print_line(&format!("  {}", step.description));
            }
        }

        // Bottom border
        print_border(BOTTOM_LEFT, BOTTOM_RIGHT);

        println!();
    }
}

/// Format duration as human-readable string
fn format_duration(dur: Duration) -> String {
    let secs = dur.as_secs();
    if secs >= 60 {
        let mins = secs / 60;
        let remaining = secs % 60;
        format!("{}m {}s", mins, remaining)
    } else {
        format!("{:.1}s", dur.as_secs_f32())
    }
}

/// Print a horizontal border line
fn print_border(left: char, right: char) {
    print!("{}", left);
    for _ in 0..(BOX_WIDTH - 2) {
        print!("{}", HORIZONTAL);
    }
    println!("{}", right);
}

/// Print a separator line with T-junctions
fn print_separator() {
    print!("{}", T_LEFT);
    for _ in 0..(BOX_WIDTH - 2) {
        print!("{}", HORIZONTAL);
    }
    println!("{}", T_RIGHT);
}

/// Print a line of repeated characters
fn print_line_chars(ch: char, count: usize) {
    print!("{}  ", VERTICAL);
    for _ in 0..count {
        print!("{}", ch);
    }
    // Pad to fill width
    let padding = BOX_WIDTH - 4 - count;
    for _ in 0..padding {
        print!(" ");
    }
    println!("{}", VERTICAL);
}

/// Print an empty line inside the box
fn print_empty_line() {
    print!("{}", VERTICAL);
    for _ in 0..(BOX_WIDTH - 2) {
        print!(" ");
    }
    println!("{}", VERTICAL);
}

/// Print centered text in the box
fn print_centered(text: &str) {
    let content_width = BOX_WIDTH - 4; // Account for borders and padding
    let text_len = text.chars().count();

    if text_len >= content_width {
        // Text too long, just print it left-aligned
        print_line(text);
        return;
    }

    let left_pad = (content_width - text_len) / 2;
    let right_pad = content_width - text_len - left_pad;

    print!("{}", VERTICAL);
    for _ in 0..(left_pad + 1) {
        print!(" ");
    }
    print!("{}", text);
    for _ in 0..(right_pad + 1) {
        print!(" ");
    }
    println!("{}", VERTICAL);
}

/// Print a line of text in the box (left-aligned with padding)
fn print_line(text: &str) {
    let content_width = BOX_WIDTH - 4; // Account for borders and padding

    // Handle multi-line by wrapping
    let mut remaining = text;
    while !remaining.is_empty() {
        let (line, rest) = if remaining.chars().count() <= content_width {
            (remaining, "")
        } else {
            // Find a good break point
            let mut break_point = content_width;
            for (i, c) in remaining.char_indices() {
                if i >= content_width {
                    break;
                }
                if c == ' ' {
                    break_point = i + 1;
                }
            }
            let (l, r) = remaining.split_at(
                remaining
                    .char_indices()
                    .nth(break_point)
                    .map(|(i, _)| i)
                    .unwrap_or(remaining.len()),
            );
            (l, r.trim_start())
        };

        let line_len = line.chars().count();
        let padding = content_width.saturating_sub(line_len);

        print!("{}  {}", VERTICAL, line);
        for _ in 0..padding {
            print!(" ");
        }
        println!("{}", VERTICAL);

        remaining = rest;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_entry() {
        let entry = FileEntry::new("db/training.db", FileCategory::Database)
            .with_description("Training database");
        assert_eq!(entry.category.tag(), "[DB]");
        assert_eq!(entry.description.as_deref(), Some("Training database"));
    }

    #[test]
    fn test_next_step() {
        let primary = NextStep::primary("cargo run", "Run the game");
        let secondary = NextStep::secondary("cargo test", "Run tests");
        assert_eq!(primary.priority, NextStepPriority::Primary);
        assert_eq!(secondary.priority, NextStepPriority::Secondary);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30.0s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "61m 1s");
    }

    #[test]
    fn test_builder_pattern() {
        let summary = RunSummary::new("Test Run")
            .description("A test run")
            .stat("Items", "10")
            .file_simple("output.txt", FileCategory::Data)
            .next_step(NextStep::primary("next cmd", "Do next thing"))
            .duration(Duration::from_secs(42));

        assert_eq!(summary.title, "Test Run");
        assert_eq!(summary.stats.len(), 1);
        assert_eq!(summary.files_created.len(), 1);
        assert_eq!(summary.next_steps.len(), 1);
    }
}
