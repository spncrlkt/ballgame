//! Stored SQL analysis requests and generic query runner.

use std::fs;
use std::path::Path;

use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequestFile {
    pub requests: Vec<AnalysisRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequest {
    pub name: String,
    pub description: Option<String>,
    pub db_path: Option<String>,
    pub db_label: Option<String>,
    pub queries: Vec<AnalysisQuery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisQuery {
    pub name: String,
    pub sql: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub name: String,
    pub sql: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnalysisRunReport {
    pub request_name: String,
    pub db_path: String,
    pub db_label: Option<String>,
    pub queries: Vec<QueryResult>,
}

impl AnalysisRequestFile {
    pub fn load(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json =
            serde_json::to_string_pretty(self).unwrap_or_else(|_| "{\"requests\":[]}".to_string());
        fs::write(path, json)
    }

    pub fn add_request(&mut self, request: AnalysisRequest) {
        if let Some(existing) = self.requests.iter_mut().find(|r| r.name == request.name) {
            for query in request.queries {
                existing.queries.push(query);
            }
            if existing.description.is_none() {
                existing.description = request.description;
            }
            if existing.db_path.is_none() {
                existing.db_path = request.db_path;
            }
            if existing.db_label.is_none() {
                existing.db_label = request.db_label;
            }
        } else {
            self.requests.push(request);
        }
    }
}

fn value_to_string(value: rusqlite::types::ValueRef<'_>) -> String {
    match value {
        rusqlite::types::ValueRef::Null => "NULL".to_string(),
        rusqlite::types::ValueRef::Integer(v) => v.to_string(),
        rusqlite::types::ValueRef::Real(v) => format!("{:.4}", v),
        rusqlite::types::ValueRef::Text(v) => String::from_utf8_lossy(v).to_string(),
        rusqlite::types::ValueRef::Blob(v) => format!("<blob {} bytes>", v.len()),
    }
}

pub fn run_request(
    request: &AnalysisRequest,
    db_override: Option<&Path>,
) -> Result<AnalysisRunReport> {
    let db_path = db_override
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .or_else(|| request.db_path.clone())
        .unwrap_or_else(|| "db/training.db".to_string());

    let conn = Connection::open(Path::new(&db_path))?;

    let mut results = Vec::new();
    for query in &request.queries {
        let mut stmt = conn.prepare(&query.sql)?;
        let column_count = stmt.column_count();
        let columns = (0..column_count)
            .map(|i| stmt.column_name(i).unwrap_or("").to_string())
            .collect::<Vec<_>>();

        let mut rows_out = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let mut out_row = Vec::with_capacity(column_count);
            for i in 0..column_count {
                let value = row.get_ref(i)?;
                out_row.push(value_to_string(value));
            }
            rows_out.push(out_row);
        }

        results.push(QueryResult {
            name: query.name.clone(),
            sql: query.sql.clone(),
            columns,
            rows: rows_out,
            notes: query.notes.clone(),
        });
    }

    Ok(AnalysisRunReport {
        request_name: request.name.clone(),
        db_path,
        db_label: request.db_label.clone(),
        queries: results,
    })
}

impl AnalysisRunReport {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Analysis Request Report\n\n");
        out.push_str(&format!("Request: `{}`\n\n", self.request_name));
        if let Some(label) = &self.db_label {
            out.push_str(&format!("DB: `{}` ({})\n\n", label, self.db_path));
        } else {
            out.push_str(&format!("DB: `{}`\n\n", self.db_path));
        }

        for query in &self.queries {
            out.push_str(&format!("## {}\n", query.name));
            if let Some(notes) = &query.notes {
                out.push_str(&format!("Notes: {}\n\n", notes));
            }
            out.push_str("SQL:\n```\n");
            out.push_str(&query.sql);
            out.push_str("\n```\n\n");
            if query.columns.is_empty() {
                out.push_str("_No columns returned_\n\n");
                continue;
            }
            out.push_str(&query.columns.join(" | "));
            out.push_str("\n");
            out.push_str(
                &query
                    .columns
                    .iter()
                    .map(|_| "---")
                    .collect::<Vec<_>>()
                    .join(" | "),
            );
            out.push_str("\n");
            if query.rows.is_empty() {
                out.push_str("_No rows_\n\n");
                continue;
            }
            for row in &query.rows {
                out.push_str(&row.join(" | "));
                out.push_str("\n");
            }
            out.push_str("\n");
        }

        out
    }
}
