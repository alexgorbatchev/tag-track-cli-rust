use audio_metadata::metadata::Metadata;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct MetadataInfo<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub genre: &'a str,
    pub release_date: &'a str,
    pub release_year: String,
    pub cover_art_url: &'a str,
    pub source: &'a str,
}
impl<'a> From<&'a Metadata> for MetadataInfo<'a> {
    fn from(v: &'a Metadata) -> Self {
        Self {
            title: &v.title,
            artist: &v.artist,
            album: &v.album,
            genre: &v.genre,
            release_date: &v.release_date,
            release_year: v.release_year(),
            cover_art_url: &v.cover_url,
            source: &v.provider,
        }
    }
}
#[derive(Serialize)]
pub struct ResultInfo<'a> {
    pub path: PathBuf,
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub release_year: String,
    pub source: &'a str,
    pub status: &'a str,
}
#[derive(Serialize)]
pub struct Event<'a> {
    pub timestamp: DateTime<Utc>,
    pub r#type: &'a str,
    pub phase: &'a str,
    pub step: usize,
    pub total_steps: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MetadataInfo<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ResultInfo<'a>>,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub error: &'a str,
}
impl<'a> Event<'a> {
    pub fn new(kind: &'a str, phase: &'a str, step: usize, total_steps: usize) -> Self {
        Self {
            timestamp: Utc::now(),
            r#type: kind,
            phase,
            step,
            total_steps,
            metadata: None,
            result: None,
            error: "",
        }
    }
}
