use std::fs;
use std::path::{Path, PathBuf};

use unveil_contracts::{GlossaryEntry, TimelineItem, WikiArticle, WikiHit};

pub struct WikiPack {
    pub articles: Vec<WikiArticle>,
    pub timeline: Vec<TimelineItem>,
    pub glossary: Vec<GlossaryEntry>,
    #[allow(dead_code)]
    pub root: PathBuf,
}

impl WikiPack {
    pub fn load(root: PathBuf) -> Self {
        let articles_dir = root.join("articles");
        let mut articles = Vec::new();
        if let Ok(entries) = fs::read_dir(&articles_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                if let Ok(text) = fs::read_to_string(&path) {
                    if let Some(article) = parse_article(&text) {
                        articles.push(article);
                    }
                }
            }
        }
        articles.sort_by(|a, b| a.article_id.cmp(&b.article_id));
        let timeline = load_timeline(&root.join("timeline.json"));
        let glossary = load_glossary(&root.join("glossary.json"));
        Self {
            articles,
            timeline,
            glossary,
            root,
        }
    }

    pub fn get(&self, article_id: &str) -> Option<WikiArticle> {
        self.articles
            .iter()
            .find(|a| a.article_id == article_id)
            .cloned()
    }

    pub fn search(&self, query: &str) -> Vec<WikiHit> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self
                .articles
                .iter()
                .take(20)
                .map(|a| hit(a, "index"))
                .collect();
        }
        let mut scored: Vec<(i32, WikiHit)> = self
            .articles
            .iter()
            .filter_map(|a| {
                let mut score = 0;
                if a.technique_ids.iter().any(|t| t.to_lowercase() == q) {
                    score += 100;
                }
                if a.title.to_lowercase().contains(&q) {
                    score += 50;
                }
                if a.aliases.iter().any(|n| n.to_lowercase().contains(&q)) {
                    score += 40;
                }
                if a.body_markdown.to_lowercase().contains(&q) {
                    score += 10;
                }
                if score == 0 {
                    return None;
                }
                Some((score, hit(a, &excerpt_for(&a.body_markdown, &q))))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.article_id.cmp(&b.1.article_id)));
        scored.into_iter().take(20).map(|(_, h)| h).collect()
    }

    #[allow(dead_code)]
    pub fn by_technique(&self, technique_id: &str) -> Option<WikiArticle> {
        self.articles
            .iter()
            .find(|a| a.technique_ids.iter().any(|t| t == technique_id))
            .cloned()
    }
}

fn hit(article: &WikiArticle, excerpt: &str) -> WikiHit {
    WikiHit {
        article_id: article.article_id.clone(),
        title: article.title.clone(),
        locale: article.locale.clone(),
        excerpt: excerpt.chars().take(180).collect(),
        engine_support: article.engine_support.clone(),
        technique_ids: article.technique_ids.clone(),
    }
}

fn excerpt_for(body: &str, q: &str) -> String {
    let lower = body.to_lowercase();
    if let Some(pos) = lower.find(q) {
        let start = pos.saturating_sub(40);
        body.chars().skip(start).take(160).collect()
    } else {
        body.chars().take(160).collect()
    }
}

fn parse_article(text: &str) -> Option<WikiArticle> {
    let rest = text.strip_prefix("---")?;
    let (fm, body) = rest.split_once("\n---")?;
    let mut article = WikiArticle {
        article_id: String::new(),
        title: String::new(),
        locale: "ja-JP".into(),
        revision: 1,
        category: String::new(),
        technique_ids: Vec::new(),
        aliases: Vec::new(),
        difficulty: "beginner".into(),
        engine_support: "wiki_only".into(),
        citations: Vec::new(),
        body_markdown: body.trim().to_string(),
        license: "CC-BY-4.0 research notes. Citations remain with original authors.".into(),
    };
    for line in fm.lines() {
        let line = line.trim();
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            let val = v.trim().trim_matches('"');
            match key {
                "article_id" => article.article_id = val.into(),
                "title" => article.title = val.into(),
                "locale" => article.locale = val.into(),
                "revision" => article.revision = val.parse().unwrap_or(1),
                "category" => article.category = val.into(),
                "difficulty" => article.difficulty = val.into(),
                "engine_support" => article.engine_support = val.into(),
                "technique_ids" => article.technique_ids = parse_list(val),
                "aliases" => article.aliases = parse_list(val),
                "citations" => article.citations = parse_list(val),
                _ => {}
            }
        }
    }
    if article.article_id.is_empty() {
        None
    } else {
        Some(article)
    }
}

fn parse_list(val: &str) -> Vec<String> {
    val.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn load_timeline(path: &Path) -> Vec<TimelineItem> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn load_glossary(path: &Path) -> Vec<GlossaryEntry> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn default_knowledge_dir() -> PathBuf {
    if let Ok(path) = std::env::var("UNVEIL_KNOWLEDGE") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../knowledge/pack-1.0.0")
}
