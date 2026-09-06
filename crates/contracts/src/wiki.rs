use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WikiHit {
    pub article_id: String,
    pub title: String,
    pub locale: String,
    pub excerpt: String,
    pub engine_support: String,
    pub technique_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WikiArticle {
    pub article_id: String,
    pub title: String,
    pub locale: String,
    pub revision: u32,
    pub category: String,
    pub technique_ids: Vec<String>,
    pub aliases: Vec<String>,
    pub difficulty: String,
    pub engine_support: String,
    pub citations: Vec<String>,
    pub body_markdown: String,
    pub license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineItem {
    pub year: String,
    pub title: String,
    pub note: String,
    pub citation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlossaryEntry {
    pub term: String,
    pub definition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportDocument {
    pub report_id: String,
    pub session_id: String,
    pub format: String,
    pub json: String,
    pub html: String,
    pub included_original: bool,
    pub included_bodies: bool,
    pub redactions: Vec<String>,
    pub reproducibility_gaps: Vec<String>,
}
