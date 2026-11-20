//! Files the app keeps in the OS app-data directory: config.json (server
//! URL + token), the SQLite database and the ideas' cover images. skill.md
//! held the single idea skill of earlier versions; it seeds the first skill.

use crate::{db, image};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const DEFAULT_SKILL: &str = include_str!("../default-skill.md");

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub base_url: String,
    pub user_name: String,
    pub user_email: String,
    pub company_name: Option<String>,
    pub website_url: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub token: String,
    pub session: Session,
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Couldn't locate the app data folder: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create {}: {e}", dir.display()))?;
    Ok(dir)
}

/// An empty folder the AI CLIs run in, so they don't pick up any project's
/// instructions or files.
pub fn workspace_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = data_dir(app)?.join("workspace");
    fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create {}: {e}", dir.display()))?;
    Ok(dir)
}

pub fn database(app: &AppHandle) -> Result<Connection, String> {
    db::open(&data_dir(app)?.join("growth-engine.db"))
}

/// Moves ideas saved by earlier versions (ideas.json) into the database.
pub fn import_legacy_ideas(app: &AppHandle) -> Result<usize, String> {
    let json_path = data_dir(app)?.join("ideas.json");
    db::import_legacy_json(&mut database(app)?, &json_path)
}

pub fn load_config(app: &AppHandle) -> Result<Option<Config>, String> {
    let path = data_dir(app)?.join("config.json");
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("Couldn't read config.json: {e}"))?;
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|e| format!("config.json is corrupted ({e}). Fix or delete {}.", path.display()))
}

pub fn save_config(app: &AppHandle, config: &Config) -> Result<(), String> {
    let dir = data_dir(app)?;
    let raw = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    // Write-then-rename so a crash mid-write can't truncate the real file.
    let tmp = dir.join("config.json.tmp");
    fs::write(&tmp, raw).map_err(|e| format!("Couldn't save the token: {e}"))?;
    fs::rename(&tmp, dir.join("config.json")).map_err(|e| format!("Couldn't save the token: {e}"))
}

pub fn clear_config(app: &AppHandle) -> Result<(), String> {
    let path = data_dir(app)?.join("config.json");
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Couldn't remove the saved token: {e}"))?;
    }
    Ok(())
}

/// Gives a fresh install its first skill: the one edited in skill.md by
/// earlier versions, else the built-in template.
pub fn seed_skills(app: &AppHandle, now: u64) -> Result<(), String> {
    let legacy = data_dir(app)?.join("skill.md");
    let content = fs::read_to_string(&legacy)
        .ok()
        .filter(|content| !content.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SKILL.to_string());
    db::seed_skills(&database(app)?, &content, now)
}

/// Cover images live as files next to the database, named per idea.
fn images_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = data_dir(app)?.join("images");
    fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create {}: {e}", dir.display()))?;
    Ok(dir)
}

/// Saves the picture for the idea and returns its file name.
pub fn save_image(app: &AppHandle, idea_id: &str, picture: &image::Picture, now: u64) -> Result<String, String> {
    let safe_id: String = idea_id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let file = format!("{safe_id}-{now}.{}", picture.extension);
    fs::write(images_dir(app)?.join(&file), &picture.bytes).map_err(|e| format!("Couldn't save the image: {e}"))?;
    Ok(file)
}

/// The saved image as a data URL, or `None` when the file is gone.
pub fn read_image(app: &AppHandle, file: &str) -> Result<Option<String>, String> {
    let path = images_dir(app)?.join(file);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|e| format!("Couldn't read the image: {e}"))?;
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("png");
    Ok(Some(image::to_data_url(&bytes, extension)))
}

/// Best effort: a leftover file only costs disk space.
pub fn delete_image(app: &AppHandle, file: &str) {
    if let Ok(dir) = images_dir(app) {
        let _ = fs::remove_file(dir.join(file));
    }
}
