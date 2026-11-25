//! Local SQLite database (growth-engine.db in the app-data folder): the
//! generated ideas, the offers the AI agent drafted, and the tool
//! permissions granted to each AI agent.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Idea {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub short_notes: String,
    #[serde(default)]
    pub pain_addressed: String,
    #[serde(default)]
    pub target_keyword: String,
    #[serde(default)]
    pub pattern: String,
    #[serde(default)]
    pub tag: String,
    pub source: String,
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub channel_name: Option<String>,
    /// The platform the idea was planned for (LinkedIn, Facebook, X or
    /// Website). Kept as planned when the post goes out on other channels.
    #[serde(default)]
    pub channel_platform: Option<String>,
    /// The topic the plan was focused on, if the user gave one.
    #[serde(default)]
    pub topic_title: Option<String>,
    #[serde(default)]
    pub topic_description: Option<String>,
    #[serde(default)]
    pub scheduled_date: Option<String>,
    #[serde(default)]
    pub scheduled_time: Option<String>,
    #[serde(default)]
    pub offer_id: Option<i64>,
    #[serde(default)]
    pub offer_name: Option<String>,
    #[serde(default)]
    pub suggested_draft: String,
    #[serde(default)]
    pub image_prompt: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub provider: Option<String>,
    pub created_at: u64,
    /// When the idea was last sent to the main site, if ever.
    #[serde(default)]
    pub sent_at: Option<u64>,
    /// The idea's id on the main site once sent.
    #[serde(default)]
    pub remote_id: Option<i64>,
    /// The channels picked on the Write post page, each with its schedule.
    #[serde(default)]
    pub publish_channels: Vec<ChannelSchedule>,
    /// When the idea's post was last sent to the main site's publish queue.
    #[serde(default)]
    pub published_at: Option<u64>,
    /// File name of the cover image in the app's images folder, if any.
    #[serde(default)]
    pub image_file: Option<String>,
    /// The idea skill the plan wrote this idea with.
    #[serde(default)]
    pub skill_name: Option<String>,
}

/// A named set of writing instructions sent to the AI agent with a plan.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub content: String,
    pub is_default: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

/// The skill picked by default for one of the planner's channels.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSkill {
    pub channel_id: String,
    pub skill_id: String,
}

/// One channel a post goes out on, and when.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSchedule {
    pub id: String,
    #[serde(default)]
    pub scheduled_date: Option<String>,
    #[serde(default)]
    pub scheduled_time: Option<String>,
}

/// The fields a user can change on the idea's page.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeaEdit {
    pub id: String,
    pub title: String,
    pub short_notes: String,
    pub pain_addressed: String,
    pub target_keyword: String,
    pub pattern: String,
    pub tag: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub suggested_draft: String,
    pub image_prompt: Option<String>,
    #[serde(default)]
    pub publish_channels: Vec<ChannelSchedule>,
}

impl Idea {
    /// An idea the user typed in by hand.
    pub fn manual(title: String, created_at: u64) -> Self {
        Idea {
            id: format!("idea-{created_at}"),
            title,
            short_notes: String::new(),
            pain_addressed: String::new(),
            target_keyword: String::new(),
            pattern: String::new(),
            tag: String::new(),
            source: "Manual".into(),
            channel_id: None,
            channel_name: None,
            channel_platform: None,
            topic_title: None,
            topic_description: None,
            scheduled_date: None,
            scheduled_time: None,
            offer_id: None,
            offer_name: None,
            suggested_draft: String::new(),
            image_prompt: None,
            status: "draft".into(),
            provider: None,
            created_at,
            sent_at: None,
            remote_id: None,
            publish_channels: Vec::new(),
            published_at: None,
            image_file: None,
            skill_name: None,
        }
    }
}

/// How many ideas the Ideas screen shows per page.
pub const IDEAS_PER_PAGE: u32 = 12;

/// One page of the Ideas screen, plus the totals shown on its two tabs.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeaPage {
    pub ideas: Vec<Idea>,
    pub page: u32,
    pub last_page: u32,
    /// Ideas in the requested tab, across all pages.
    pub total: u32,
    pub draft_count: u32,
    pub sent_count: u32,
}

/// How many ideas were generated from one offer. Ideas imported from the
/// old JSON store only know the offer's name, so both keys are reported.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferIdeaCount {
    pub offer_id: Option<i64>,
    pub offer_name: Option<String>,
    pub count: i64,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Permission {
    pub provider: String,
    pub tool: String,
}

/// An offer the AI agent drafted, kept here until it is synced to the main site.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferDraft {
    pub id: String,
    pub name: String,
    pub promise: String,
    pub description: String,
    pub price_range: String,
    pub timeline: String,
    pub deliverables: Vec<String>,
    /// Ids of the main site's ICPs this offer is for.
    pub icp_ids: Vec<i64>,
    /// Ids of the main site's case studies that prove it.
    pub case_study_ids: Vec<i64>,
    /// Why the agent picked this offer.
    pub rationale: String,
    pub provider: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub synced_at: Option<u64>,
    pub remote_id: Option<i64>,
}

/// The fields of an offer draft the Offers page can change.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferDraftEdit {
    pub id: String,
    pub name: String,
    pub promise: String,
    pub description: String,
    pub price_range: String,
    pub timeline: String,
    pub deliverables: Vec<String>,
    pub icp_ids: Vec<i64>,
    pub case_study_ids: Vec<i64>,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ideas (
    id              TEXT PRIMARY KEY,
    title           TEXT NOT NULL,
    short_notes     TEXT NOT NULL DEFAULT '',
    pain_addressed  TEXT NOT NULL DEFAULT '',
    target_keyword  TEXT NOT NULL DEFAULT '',
    pattern         TEXT NOT NULL DEFAULT '',
    tag             TEXT NOT NULL DEFAULT '',
    source          TEXT NOT NULL,
    channel_id      TEXT,
    channel_name    TEXT,
    scheduled_date  TEXT,
    scheduled_time  TEXT,
    offer_id        INTEGER,
    offer_name      TEXT,
    suggested_draft TEXT NOT NULL DEFAULT '',
    image_prompt    TEXT,
    status          TEXT NOT NULL DEFAULT 'draft',
    provider        TEXT,
    created_at      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS ideas_offer_id ON ideas (offer_id);
CREATE TABLE IF NOT EXISTS agent_permissions (
    provider TEXT NOT NULL,
    tool     TEXT NOT NULL,
    PRIMARY KEY (provider, tool)
);
CREATE TABLE IF NOT EXISTS skills (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    content    TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS offer_skills (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    content    TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS channel_skills (
    channel_id TEXT PRIMARY KEY,
    skill_id   TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS offer_drafts (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    promise        TEXT NOT NULL,
    description    TEXT NOT NULL DEFAULT '',
    price_range    TEXT NOT NULL DEFAULT '',
    timeline       TEXT NOT NULL DEFAULT '',
    deliverables   TEXT NOT NULL DEFAULT '[]',
    icp_ids        TEXT NOT NULL DEFAULT '[]',
    case_study_ids TEXT NOT NULL DEFAULT '[]',
    rationale      TEXT NOT NULL DEFAULT '',
    provider       TEXT,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL,
    synced_at      INTEGER,
    remote_id      INTEGER
);
CREATE TABLE IF NOT EXISTS lead_research (
    kind          TEXT NOT NULL,
    item_id       INTEGER NOT NULL,
    payload       TEXT NOT NULL,
    researched_at INTEGER NOT NULL,
    PRIMARY KEY (kind, item_id)
);
CREATE TABLE IF NOT EXISTS post_analytics (
    post_id             TEXT PRIMARY KEY,
    platform_post_id    TEXT,
    post_url            TEXT,
    channel_id          TEXT NOT NULL,
    platform            TEXT NOT NULL DEFAULT '',
    impressions         INTEGER,
    impressions_avail   INTEGER NOT NULL DEFAULT 1,
    impressions_note    TEXT,
    views               INTEGER,
    views_avail         INTEGER NOT NULL DEFAULT 1,
    views_note          TEXT,
    likes               INTEGER,
    likes_avail         INTEGER NOT NULL DEFAULT 1,
    likes_note          TEXT,
    comments_count      INTEGER,
    comments_avail      INTEGER NOT NULL DEFAULT 1,
    comments_note       TEXT,
    latest_comments     TEXT NOT NULL DEFAULT '[]',
    last_updated        INTEGER NOT NULL,
    error               TEXT
);
";

const IDEA_COLUMNS: &str = "id, title, short_notes, pain_addressed, target_keyword, pattern, tag, source, \
    channel_id, channel_name, scheduled_date, scheduled_time, offer_id, offer_name, suggested_draft, \
    image_prompt, status, provider, created_at, sent_at, remote_id, publish_channels, published_at, \
    channel_platform, topic_title, topic_description, image_file, skill_name";

/// Columns added after the first release; `migrate` adds any that are missing.
const ADDED_IDEA_COLUMNS: [(&str, &str); 9] = [
    ("sent_at", "INTEGER"),
    ("remote_id", "INTEGER"),
    ("publish_channels", "TEXT NOT NULL DEFAULT '[]'"),
    ("published_at", "INTEGER"),
    ("channel_platform", "TEXT"),
    ("topic_title", "TEXT"),
    ("topic_description", "TEXT"),
    ("image_file", "TEXT"),
    ("skill_name", "TEXT"),
];

/// Ideas planned before `channel_platform` existed still carry the
/// planner's channel id, which names the platform.
const BACKFILL_CHANNEL_PLATFORM: &str = "UPDATE ideas SET channel_platform = CASE \
    WHEN channel_id LIKE 'linkedin%' THEN 'LinkedIn' \
    WHEN channel_id LIKE 'facebook%' THEN 'Facebook' \
    WHEN substr(channel_id, 1, 2) = 'x_' THEN 'X' \
    WHEN channel_id LIKE 'website%' THEN 'Website' END \
    WHERE channel_platform IS NULL AND channel_id IS NOT NULL";

fn channels_json(channels: &[ChannelSchedule]) -> String {
    serde_json::to_string(channels).unwrap_or_else(|_| "[]".into())
}

fn describe(error: rusqlite::Error) -> String {
    format!("The local database reported an error: {error}")
}

fn migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(SCHEMA).map_err(describe)?;
    for (column, kind) in ADDED_IDEA_COLUMNS {
        let exists = conn
            .prepare("SELECT 1 FROM pragma_table_info('ideas') WHERE name = ?1")
            .and_then(|mut statement| statement.exists([column]))
            .map_err(describe)?;
        if !exists {
            conn.execute_batch(&format!("ALTER TABLE ideas ADD COLUMN {column} {kind}")).map_err(describe)?;
            if column == "channel_platform" {
                conn.execute_batch(BACKFILL_CHANNEL_PLATFORM).map_err(describe)?;
            }
        }
    }
    Ok(())
}

pub fn open(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path)
        .map_err(|e| format!("Couldn't open the local database at {}: {e}", path.display()))?;
    conn.busy_timeout(Duration::from_secs(5)).map_err(describe)?;
    migrate(&conn)?;
    Ok(conn)
}

fn idea_from_row(row: &Row) -> rusqlite::Result<Idea> {
    Ok(Idea {
        id: row.get(0)?,
        title: row.get(1)?,
        short_notes: row.get(2)?,
        pain_addressed: row.get(3)?,
        target_keyword: row.get(4)?,
        pattern: row.get(5)?,
        tag: row.get(6)?,
        source: row.get(7)?,
        channel_id: row.get(8)?,
        channel_name: row.get(9)?,
        scheduled_date: row.get(10)?,
        scheduled_time: row.get(11)?,
        offer_id: row.get(12)?,
        offer_name: row.get(13)?,
        suggested_draft: row.get(14)?,
        image_prompt: row.get(15)?,
        status: row.get(16)?,
        provider: row.get(17)?,
        created_at: row.get::<_, i64>(18)? as u64,
        sent_at: row.get::<_, Option<i64>>(19)?.map(|sent_at| sent_at as u64),
        remote_id: row.get(20)?,
        publish_channels: serde_json::from_str(&row.get::<_, String>(21)?).unwrap_or_default(),
        published_at: row.get::<_, Option<i64>>(22)?.map(|published_at| published_at as u64),
        channel_platform: row.get(23)?,
        topic_title: row.get(24)?,
        topic_description: row.get(25)?,
        image_file: row.get(26)?,
        skill_name: row.get(27)?,
    })
}

/// Newest batch first; ideas of one batch keep the order they were planned in.
#[cfg(test)]
pub fn list_ideas(conn: &Connection) -> Result<Vec<Idea>, String> {
    let mut statement = conn
        .prepare(&format!("SELECT {IDEA_COLUMNS} FROM ideas ORDER BY created_at DESC, rowid ASC"))
        .map_err(describe)?;
    let ideas = statement
        .query_map([], idea_from_row)
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<Idea>>>()
        .map_err(describe)?;
    Ok(ideas)
}

/// One page of the drafts (not sent yet) or of the sent ideas, latest first:
/// drafts by newest batch, sent ideas by when they were last sent. A page
/// past the end falls back to the last one, so deleting the only idea on the
/// last page doesn't leave an empty screen.
pub fn list_ideas_page(conn: &Connection, sent: bool, page: u32, per_page: u32) -> Result<IdeaPage, String> {
    let (draft_count, sent_count): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*) FILTER (WHERE sent_at IS NULL), COUNT(*) FILTER (WHERE sent_at IS NOT NULL) FROM ideas",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(describe)?;

    let per_page = per_page.max(1);
    let total = if sent { sent_count } else { draft_count } as u32;
    let last_page = total.div_ceil(per_page).max(1);
    let page = page.clamp(1, last_page);
    let (filter, order) = if sent {
        ("sent_at IS NOT NULL", "sent_at DESC, rowid DESC")
    } else {
        ("sent_at IS NULL", "created_at DESC, rowid ASC")
    };

    let mut statement = conn
        .prepare(&format!("SELECT {IDEA_COLUMNS} FROM ideas WHERE {filter} ORDER BY {order} LIMIT ?1 OFFSET ?2"))
        .map_err(describe)?;
    let ideas = statement
        .query_map(params![per_page, (page - 1) * per_page], idea_from_row)
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<Idea>>>()
        .map_err(describe)?;

    Ok(IdeaPage { ideas, page, last_page, total, draft_count: draft_count as u32, sent_count: sent_count as u32 })
}

/// Titles of the newest ideas, so a new plan can avoid repeating them.
pub fn recent_idea_titles(conn: &Connection, limit: u32) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare("SELECT title FROM ideas ORDER BY created_at DESC, rowid ASC LIMIT ?1")
        .map_err(describe)?;
    let titles = statement
        .query_map([limit], |row| row.get(0))
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<String>>>()
        .map_err(describe)?;
    Ok(titles)
}

/// Inserts all ideas or none. Ideas whose id already exists are skipped.
pub fn insert_ideas(conn: &mut Connection, ideas: &[Idea]) -> Result<(), String> {
    let transaction = conn.transaction().map_err(describe)?;
    {
        let mut statement = transaction
            .prepare(&format!(
                "INSERT OR IGNORE INTO ideas ({IDEA_COLUMNS}) VALUES \
                 (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)"
            ))
            .map_err(describe)?;
        for idea in ideas {
            statement
                .execute(params![
                    idea.id,
                    idea.title,
                    idea.short_notes,
                    idea.pain_addressed,
                    idea.target_keyword,
                    idea.pattern,
                    idea.tag,
                    idea.source,
                    idea.channel_id,
                    idea.channel_name,
                    idea.scheduled_date,
                    idea.scheduled_time,
                    idea.offer_id,
                    idea.offer_name,
                    idea.suggested_draft,
                    idea.image_prompt,
                    idea.status,
                    idea.provider,
                    idea.created_at as i64,
                    idea.sent_at.map(|sent_at| sent_at as i64),
                    idea.remote_id,
                    channels_json(&idea.publish_channels),
                    idea.published_at.map(|published_at| published_at as i64),
                    idea.channel_platform,
                    idea.topic_title,
                    idea.topic_description,
                    idea.image_file,
                    idea.skill_name,
                ])
                .map_err(describe)?;
        }
    }
    transaction.commit().map_err(describe)
}

pub fn get_idea(conn: &Connection, id: &str) -> Result<Idea, String> {
    conn.query_row(&format!("SELECT {IDEA_COLUMNS} FROM ideas WHERE id = ?1"), [id], idea_from_row)
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "This idea no longer exists.".to_string(),
            other => describe(other),
        })
}

pub fn update_idea(conn: &Connection, edit: &IdeaEdit) -> Result<Idea, String> {
    conn.execute(
        "UPDATE ideas SET title = ?2, short_notes = ?3, pain_addressed = ?4, target_keyword = ?5, \
         pattern = ?6, tag = ?7, channel_id = ?8, channel_name = ?9, suggested_draft = ?10, \
         image_prompt = ?11, publish_channels = ?12 WHERE id = ?1",
        params![
            edit.id,
            edit.title,
            edit.short_notes,
            edit.pain_addressed,
            edit.target_keyword,
            edit.pattern,
            edit.tag,
            edit.channel_id,
            edit.channel_name,
            edit.suggested_draft,
            edit.image_prompt,
            channels_json(&edit.publish_channels),
        ],
    )
    .map_err(describe)?;
    get_idea(conn, &edit.id)
}

/// Records that the idea's post joined the main site's publish queue.
pub fn mark_published(conn: &Connection, id: &str, published_at: u64) -> Result<Idea, String> {
    conn.execute("UPDATE ideas SET published_at = ?2 WHERE id = ?1", params![id, published_at as i64])
        .map_err(describe)?;
    get_idea(conn, id)
}

/// Every writing pattern used by an idea, for the Write post page's picker.
pub fn list_patterns(conn: &Connection) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare("SELECT DISTINCT pattern FROM ideas WHERE pattern != '' ORDER BY pattern")
        .map_err(describe)?;
    let patterns = statement
        .query_map([], |row| row.get(0))
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<String>>>()
        .map_err(describe)?;
    Ok(patterns)
}

/// Records that the idea now lives on the main site under `remote_id`.
pub fn mark_sent(conn: &Connection, id: &str, remote_id: Option<i64>, sent_at: u64) -> Result<Idea, String> {
    conn.execute(
        "UPDATE ideas SET sent_at = ?2, remote_id = ?3 WHERE id = ?1",
        params![id, sent_at as i64, remote_id],
    )
    .map_err(describe)?;
    get_idea(conn, id)
}

pub fn delete_idea(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM ideas WHERE id = ?1", [id]).map(|_| ()).map_err(describe)
}

pub fn offer_idea_counts(conn: &Connection) -> Result<Vec<OfferIdeaCount>, String> {
    let mut statement = conn
        .prepare(
            "SELECT offer_id, offer_name, COUNT(*) FROM ideas \
             WHERE offer_id IS NOT NULL OR offer_name IS NOT NULL \
             GROUP BY offer_id, offer_name",
        )
        .map_err(describe)?;
    let counts = statement
        .query_map([], |row| {
            Ok(OfferIdeaCount { offer_id: row.get(0)?, offer_name: row.get(1)?, count: row.get(2)? })
        })
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(describe)?;
    Ok(counts)
}

pub fn list_permissions(conn: &Connection) -> Result<Vec<Permission>, String> {
    let mut statement = conn
        .prepare("SELECT provider, tool FROM agent_permissions ORDER BY provider, tool")
        .map_err(describe)?;
    let permissions = statement
        .query_map([], |row| Ok(Permission { provider: row.get(0)?, tool: row.get(1)? }))
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(describe)?;
    Ok(permissions)
}

pub fn allowed_tools(conn: &Connection, provider: &str) -> Result<Vec<String>, String> {
    Ok(list_permissions(conn)?
        .into_iter()
        .filter(|permission| permission.provider == provider)
        .map(|permission| permission.tool)
        .collect())
}

pub fn grant_permissions(conn: &Connection, provider: &str, tools: &[String]) -> Result<(), String> {
    for tool in tools {
        conn.execute(
            "INSERT OR IGNORE INTO agent_permissions (provider, tool) VALUES (?1, ?2)",
            params![provider, tool],
        )
        .map_err(describe)?;
    }
    Ok(())
}

pub fn revoke_permission(conn: &Connection, provider: &str, tool: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM agent_permissions WHERE provider = ?1 AND tool = ?2",
        params![provider, tool],
    )
    .map(|_| ())
    .map_err(describe)
}

/// Points the idea at a new cover image file (or none) and returns the
/// file it replaced, so the caller can delete it.
pub fn set_idea_image(conn: &Connection, id: &str, file: Option<&str>) -> Result<(Idea, Option<String>), String> {
    let previous = get_idea(conn, id)?.image_file;
    conn.execute("UPDATE ideas SET image_file = ?2 WHERE id = ?1", params![id, file])
        .map_err(describe)?;
    Ok((get_idea(conn, id)?, previous))
}

/* Idea skills */

const SKILL_COLUMNS: &str = "id, name, content, is_default, created_at, updated_at";

fn skill_from_row(row: &Row) -> rusqlite::Result<Skill> {
    Ok(Skill {
        id: row.get(0)?,
        name: row.get(1)?,
        content: row.get(2)?,
        is_default: row.get::<_, i64>(3)? != 0,
        created_at: row.get::<_, i64>(4)? as u64,
        updated_at: row.get::<_, i64>(5)? as u64,
    })
}

/// The default skill first, then by name.
pub fn list_skills(conn: &Connection) -> Result<Vec<Skill>, String> {
    let mut statement = conn
        .prepare(&format!("SELECT {SKILL_COLUMNS} FROM skills ORDER BY is_default DESC, name COLLATE NOCASE, created_at"))
        .map_err(describe)?;
    let skills = statement
        .query_map([], skill_from_row)
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(describe)?;
    Ok(skills)
}

pub fn get_skill(conn: &Connection, id: &str) -> Result<Skill, String> {
    conn.query_row(&format!("SELECT {SKILL_COLUMNS} FROM skills WHERE id = ?1"), [id], skill_from_row)
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "This skill no longer exists.".to_string(),
            other => describe(other),
        })
}

/// The skill used when a channel has none picked.
pub fn default_skill(conn: &Connection) -> Result<Skill, String> {
    conn.query_row(
        &format!("SELECT {SKILL_COLUMNS} FROM skills ORDER BY is_default DESC, created_at LIMIT 1"),
        [],
        skill_from_row,
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => "There are no idea skills. Add one in Settings > Idea skills.".to_string(),
        other => describe(other),
    })
}

/// Creates the first skill from `content` when there are none yet.
pub fn seed_skills(conn: &Connection, content: &str, now: u64) -> Result<(), String> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM skills", [], |row| row.get(0)).map_err(describe)?;
    if count == 0 {
        conn.execute(
            "INSERT INTO skills (id, name, content, is_default, created_at, updated_at) VALUES (?1, ?2, ?3, 1, ?4, ?4)",
            params![format!("skill-{now}"), "Content ideas", content, now as i64],
        )
        .map_err(describe)?;
    }
    Ok(())
}

pub fn create_skill(conn: &Connection, name: &str, content: &str, now: u64) -> Result<Skill, String> {
    let id = format!("skill-{now}");
    conn.execute(
        "INSERT INTO skills (id, name, content, is_default, created_at, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?4)",
        params![id, name, content, now as i64],
    )
    .map_err(describe)?;
    get_skill(conn, &id)
}

pub fn update_skill(conn: &Connection, id: &str, name: &str, content: &str, now: u64) -> Result<Skill, String> {
    let changed = conn
        .execute(
            "UPDATE skills SET name = ?2, content = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, name, content, now as i64],
        )
        .map_err(describe)?;
    if changed == 0 {
        return Err("This skill no longer exists.".into());
    }
    get_skill(conn, id)
}

/// Deletes a skill that is not the default. Channels that used it fall
/// back to the default skill.
pub fn delete_skill(conn: &Connection, id: &str) -> Result<(), String> {
    if get_skill(conn, id)?.is_default {
        return Err("The default skill can't be deleted. Make another skill the default first.".into());
    }
    conn.execute("DELETE FROM channel_skills WHERE skill_id = ?1", [id]).map_err(describe)?;
    conn.execute("DELETE FROM skills WHERE id = ?1", [id]).map(|_| ()).map_err(describe)
}

pub fn set_default_skill(conn: &Connection, id: &str) -> Result<(), String> {
    get_skill(conn, id)?;
    conn.execute("UPDATE skills SET is_default = (id = ?1)", [id]).map(|_| ()).map_err(describe)
}

/* Offer skills: instructions the agent follows when it drafts offers */

const OFFER_SKILL_COLUMNS: &str = "id, name, content, 0, created_at, updated_at";

pub fn list_offer_skills(conn: &Connection) -> Result<Vec<Skill>, String> {
    let mut statement = conn
        .prepare(&format!("SELECT {OFFER_SKILL_COLUMNS} FROM offer_skills ORDER BY name COLLATE NOCASE, created_at"))
        .map_err(describe)?;
    let skills = statement
        .query_map([], skill_from_row)
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(describe)?;
    Ok(skills)
}

pub fn get_offer_skill(conn: &Connection, id: &str) -> Result<Skill, String> {
    conn.query_row(&format!("SELECT {OFFER_SKILL_COLUMNS} FROM offer_skills WHERE id = ?1"), [id], skill_from_row)
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "This offer skill no longer exists.".to_string(),
            other => describe(other),
        })
}

pub fn create_offer_skill(conn: &Connection, name: &str, content: &str, now: u64) -> Result<Skill, String> {
    let id = format!("offer-skill-{now}");
    conn.execute(
        "INSERT INTO offer_skills (id, name, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, name, content, now as i64],
    )
    .map_err(describe)?;
    get_offer_skill(conn, &id)
}

pub fn update_offer_skill(conn: &Connection, id: &str, name: &str, content: &str, now: u64) -> Result<Skill, String> {
    let changed = conn
        .execute(
            "UPDATE offer_skills SET name = ?2, content = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, name, content, now as i64],
        )
        .map_err(describe)?;
    if changed == 0 {
        return Err("This offer skill no longer exists.".into());
    }
    get_offer_skill(conn, id)
}

pub fn delete_offer_skill(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM offer_skills WHERE id = ?1", [id]).map(|_| ()).map_err(describe)
}

pub fn channel_skills(conn: &Connection) -> Result<Vec<ChannelSkill>, String> {
    let mut statement = conn
        .prepare("SELECT channel_id, skill_id FROM channel_skills WHERE skill_id IN (SELECT id FROM skills) ORDER BY channel_id")
        .map_err(describe)?;
    let rows = statement
        .query_map([], |row| Ok(ChannelSkill { channel_id: row.get(0)?, skill_id: row.get(1)? }))
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(describe)?;
    Ok(rows)
}

/// Picks the channel's default skill; `None` makes it use the default skill.
pub fn set_channel_skill(conn: &Connection, channel_id: &str, skill_id: Option<&str>) -> Result<(), String> {
    match skill_id {
        Some(skill_id) => {
            get_skill(conn, skill_id)?;
            conn.execute(
                "INSERT INTO channel_skills (channel_id, skill_id) VALUES (?1, ?2) \
                 ON CONFLICT(channel_id) DO UPDATE SET skill_id = excluded.skill_id",
                params![channel_id, skill_id],
            )
        }
        None => conn.execute("DELETE FROM channel_skills WHERE channel_id = ?1", [channel_id]),
    }
    .map(|_| ())
    .map_err(describe)
}

/* App settings */

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    match conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get(0)) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(other) => Err(describe(other)),
    }
}

/// Saves the setting; `None` removes it.
pub fn set_setting(conn: &Connection, key: &str, value: Option<&str>) -> Result<(), String> {
    match value {
        Some(value) => conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        ),
        None => conn.execute("DELETE FROM settings WHERE key = ?1", [key]),
    }
    .map(|_| ())
    .map_err(describe)
}

/* Offers drafted by the AI agent */

const OFFER_COLUMNS: &str = "id, name, promise, description, price_range, timeline, deliverables, icp_ids, \
    case_study_ids, rationale, provider, created_at, updated_at, synced_at, remote_id";

fn json_list<T: serde::de::DeserializeOwned>(raw: String) -> Vec<T> {
    serde_json::from_str(&raw).unwrap_or_default()
}

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".into())
}

fn offer_draft_from_row(row: &Row) -> rusqlite::Result<OfferDraft> {
    Ok(OfferDraft {
        id: row.get(0)?,
        name: row.get(1)?,
        promise: row.get(2)?,
        description: row.get(3)?,
        price_range: row.get(4)?,
        timeline: row.get(5)?,
        deliverables: json_list(row.get(6)?),
        icp_ids: json_list(row.get(7)?),
        case_study_ids: json_list(row.get(8)?),
        rationale: row.get(9)?,
        provider: row.get(10)?,
        created_at: row.get::<_, i64>(11)? as u64,
        updated_at: row.get::<_, i64>(12)? as u64,
        synced_at: row.get::<_, Option<i64>>(13)?.map(|synced_at| synced_at as u64),
        remote_id: row.get(14)?,
    })
}

pub fn insert_offer_drafts(conn: &mut Connection, drafts: &[OfferDraft]) -> Result<(), String> {
    let tx = conn.transaction().map_err(describe)?;
    for draft in drafts {
        tx.execute(
            &format!(
                "INSERT INTO offer_drafts ({OFFER_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"
            ),
            params![
                draft.id,
                draft.name,
                draft.promise,
                draft.description,
                draft.price_range,
                draft.timeline,
                to_json(&draft.deliverables),
                to_json(&draft.icp_ids),
                to_json(&draft.case_study_ids),
                draft.rationale,
                draft.provider,
                draft.created_at as i64,
                draft.updated_at as i64,
                draft.synced_at.map(|synced_at| synced_at as i64),
                draft.remote_id,
            ],
        )
        .map_err(describe)?;
    }
    tx.commit().map_err(describe)
}

/// The drafts not on the main site yet, newest batch first, each batch in
/// the order the agent wrote it.
pub fn pending_offer_drafts(conn: &Connection) -> Result<Vec<OfferDraft>, String> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT {OFFER_COLUMNS} FROM offer_drafts WHERE synced_at IS NULL ORDER BY created_at DESC, rowid ASC"
        ))
        .map_err(describe)?;
    let drafts = statement
        .query_map([], offer_draft_from_row)
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(describe)?;
    Ok(drafts)
}

/// Names of the latest drafted offers, synced or not, so a new run does not repeat them.
pub fn offer_draft_names(conn: &Connection) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare("SELECT name FROM offer_drafts ORDER BY created_at DESC LIMIT 100")
        .map_err(describe)?;
    let names = statement
        .query_map([], |row| row.get(0))
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<String>>>()
        .map_err(describe)?;
    Ok(names)
}

pub fn get_offer_draft(conn: &Connection, id: &str) -> Result<OfferDraft, String> {
    conn.query_row(&format!("SELECT {OFFER_COLUMNS} FROM offer_drafts WHERE id = ?1"), [id], offer_draft_from_row)
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "This offer draft no longer exists.".to_string(),
            other => describe(other),
        })
}

pub fn update_offer_draft(conn: &Connection, edit: &OfferDraftEdit, now: u64) -> Result<OfferDraft, String> {
    let changed = conn
        .execute(
            "UPDATE offer_drafts SET name = ?2, promise = ?3, description = ?4, price_range = ?5, timeline = ?6, \
             deliverables = ?7, icp_ids = ?8, case_study_ids = ?9, updated_at = ?10 WHERE id = ?1",
            params![
                edit.id,
                edit.name,
                edit.promise,
                edit.description,
                edit.price_range,
                edit.timeline,
                to_json(&edit.deliverables),
                to_json(&edit.icp_ids),
                to_json(&edit.case_study_ids),
                now as i64,
            ],
        )
        .map_err(describe)?;
    if changed == 0 {
        return Err("This offer draft no longer exists.".into());
    }
    get_offer_draft(conn, &edit.id)
}

pub fn delete_offer_draft(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM offer_drafts WHERE id = ?1", [id]).map(|_| ()).map_err(describe)
}

pub fn mark_offer_synced(conn: &Connection, id: &str, remote_id: Option<i64>, synced_at: u64) -> Result<(), String> {
    conn.execute(
        "UPDATE offer_drafts SET synced_at = ?2, remote_id = ?3 WHERE id = ?1",
        params![id, synced_at as i64, remote_id],
    )
    .map(|_| ())
    .map_err(describe)
}

/* Lead research waiting to be sent to the main site */

/// What the research agent found for one queue item, not yet synced.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingResearch {
    pub kind: String,
    pub id: i64,
    /// The body for `/api/leads/research`: type, id and the fields found.
    pub payload: serde_json::Value,
    pub researched_at: u64,
}

/// Keeps the findings for an item; a newer run replaces an unsynced older one.
pub fn save_lead_research(conn: &Connection, kind: &str, id: i64, payload: &serde_json::Value, researched_at: u64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO lead_research (kind, item_id, payload, researched_at) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(kind, item_id) DO UPDATE SET payload = excluded.payload, researched_at = excluded.researched_at",
        params![kind, id, payload.to_string(), researched_at as i64],
    )
    .map(|_| ())
    .map_err(describe)
}

pub fn pending_lead_research(conn: &Connection) -> Result<Vec<PendingResearch>, String> {
    let mut statement = conn
        .prepare("SELECT kind, item_id, payload, researched_at FROM lead_research ORDER BY researched_at, rowid")
        .map_err(describe)?;
    let rows = statement
        .query_map([], |row| {
            let payload: String = row.get(2)?;
            Ok(PendingResearch {
                kind: row.get(0)?,
                id: row.get(1)?,
                payload: serde_json::from_str(&payload).unwrap_or_default(),
                researched_at: row.get::<_, i64>(3)? as u64,
            })
        })
        .map_err(describe)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(describe)?;
    Ok(rows)
}

/// Forgets findings that reached the main site.
pub fn clear_lead_research(conn: &Connection, sent: &[PendingResearch]) -> Result<(), String> {
    for item in sent {
        conn.execute("DELETE FROM lead_research WHERE kind = ?1 AND item_id = ?2 AND researched_at = ?3", params![item.kind, item.id, item.researched_at as i64])
            .map_err(describe)?;
    }
    Ok(())
}

pub fn get_post_analytics(conn: &Connection, post_id: &str) -> Result<Option<crate::analytics::PostAnalytics>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT post_id, platform_post_id, post_url, channel_id, platform, \
             impressions, impressions_avail, impressions_note, \
             views, views_avail, views_note, \
             likes, likes_avail, likes_note, \
             comments_count, comments_avail, comments_note, \
             latest_comments, last_updated, error \
             FROM post_analytics WHERE post_id = ?1",
        )
        .map_err(describe)?;

    let mut rows = stmt.query(params![post_id]).map_err(describe)?;
    if let Some(row) = rows.next().map_err(describe)? {
        let impressions_val: Option<i64> = row.get(5).ok();
        let impressions_avail: i64 = row.get(6).unwrap_or(0);
        let impressions_note: Option<String> = row.get(7).ok();

        let views_val: Option<i64> = row.get(8).ok();
        let views_avail: i64 = row.get(9).unwrap_or(0);
        let views_note: Option<String> = row.get(10).ok();

        let likes_val: Option<i64> = row.get(11).ok();
        let likes_avail: i64 = row.get(12).unwrap_or(0);
        let likes_note: Option<String> = row.get(13).ok();

        let comments_val: Option<i64> = row.get(14).ok();
        let comments_avail: i64 = row.get(15).unwrap_or(0);
        let comments_note: Option<String> = row.get(16).ok();

        let latest_comments_raw: String = row.get(17).unwrap_or_else(|_| "[]".to_string());
        let latest_comments: Vec<crate::analytics::PostComment> =
            serde_json::from_str(&latest_comments_raw).unwrap_or_default();

        let make_metric = |val: Option<i64>, avail: bool, note: Option<String>| {
            if avail && val.is_some() {
                crate::analytics::MetricValue::available(val.unwrap())
            } else {
                crate::analytics::MetricValue::unavailable(note.unwrap_or_else(|| "Unavailable".into()))
            }
        };

        Ok(Some(crate::analytics::PostAnalytics {
            post_id: row.get(0).map_err(describe)?,
            platform_post_id: row.get(1).ok(),
            post_url: row.get(2).ok(),
            channel_id: row.get(3).map_err(describe)?,
            platform: row.get(4).unwrap_or_else(|_| "".to_string()),
            impressions: make_metric(impressions_val, impressions_avail == 1, impressions_note),
            views: make_metric(views_val, views_avail == 1, views_note),
            likes: make_metric(likes_val, likes_avail == 1, likes_note),
            comments: make_metric(comments_val, comments_avail == 1, comments_note),
            latest_comments,
            last_updated: row.get::<_, i64>(18).unwrap_or(0) as u64,
            error: row.get(19).ok(),
        }))
    } else {
        Ok(None)
    }
}

pub fn save_post_analytics(conn: &Connection, analytics: &crate::analytics::PostAnalytics) -> Result<(), String> {
    let latest_comments_raw = serde_json::to_string(&analytics.latest_comments).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO post_analytics (
            post_id, platform_post_id, post_url, channel_id, platform,
            impressions, impressions_avail, impressions_note,
            views, views_avail, views_note,
            likes, likes_avail, likes_note,
            comments_count, comments_avail, comments_note,
            latest_comments, last_updated, error
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
        ON CONFLICT(post_id) DO UPDATE SET
            platform_post_id = excluded.platform_post_id,
            post_url = COALESCE(excluded.post_url, post_analytics.post_url),
            channel_id = excluded.channel_id,
            platform = excluded.platform,
            impressions = excluded.impressions,
            impressions_avail = excluded.impressions_avail,
            impressions_note = excluded.impressions_note,
            views = excluded.views,
            views_avail = excluded.views_avail,
            views_note = excluded.views_note,
            likes = excluded.likes,
            likes_avail = excluded.likes_avail,
            likes_note = excluded.likes_note,
            comments_count = excluded.comments_count,
            comments_avail = excluded.comments_avail,
            comments_note = excluded.comments_note,
            latest_comments = excluded.latest_comments,
            last_updated = excluded.last_updated,
            error = excluded.error",
        params![
            analytics.post_id,
            analytics.platform_post_id,
            analytics.post_url,
            analytics.channel_id,
            analytics.platform,
            analytics.impressions.value,
            if analytics.impressions.available { 1 } else { 0 },
            analytics.impressions.unavailable_reason,
            analytics.views.value,
            if analytics.views.available { 1 } else { 0 },
            analytics.views.unavailable_reason,
            analytics.likes.value,
            if analytics.likes.available { 1 } else { 0 },
            analytics.likes.unavailable_reason,
            analytics.comments.value,
            if analytics.comments.available { 1 } else { 0 },
            analytics.comments.unavailable_reason,
            latest_comments_raw,
            analytics.last_updated as i64,
            analytics.error,
        ],
    )
    .map(|_| ())
    .map_err(describe)
}

pub fn set_post_platform_id(conn: &Connection, post_id: &str, platform_post_id: &str, post_url: Option<&str>) -> Result<(), String> {
    conn.execute(
        "INSERT INTO post_analytics (post_id, platform_post_id, post_url, channel_id, platform, last_updated)
         VALUES (?1, ?2, ?3, '', '', 0)
         ON CONFLICT(post_id) DO UPDATE SET
            platform_post_id = excluded.platform_post_id,
            post_url = COALESCE(excluded.post_url, post_analytics.post_url)",
        params![post_id, platform_post_id, post_url],
    )
    .map(|_| ())
    .map_err(describe)
}

pub fn get_post_platform_id(conn: &Connection, post_id: &str) -> Result<Option<(String, Option<String>)>, String> {
    let mut stmt = conn
        .prepare("SELECT platform_post_id, post_url FROM post_analytics WHERE post_id = ?1")
        .map_err(describe)?;
    let mut rows = stmt.query(params![post_id]).map_err(describe)?;
    if let Some(row) = rows.next().map_err(describe)? {
        let pid: Option<String> = row.get(0).ok();
        let url: Option<String> = row.get(1).ok();
        if let Some(pid) = pid.filter(|s| !s.trim().is_empty()) {
            return Ok(Some((pid, url)));
        }
    }
    Ok(None)
}

/// One-time move of ideas from the pre-SQLite ideas.json into the database.
/// The file is renamed afterwards so it is only imported once.
pub fn import_legacy_json(conn: &mut Connection, json_path: &Path) -> Result<usize, String> {
    if !json_path.exists() {
        return Ok(0);
    }
    let raw = fs::read_to_string(json_path).map_err(|e| format!("Couldn't read ideas.json: {e}"))?;
    let ideas: Vec<Idea> =
        serde_json::from_str(&raw).map_err(|e| format!("Couldn't import ideas.json: {e}"))?;
    insert_ideas(conn, &ideas)?;
    fs::rename(json_path, json_path.with_extension("json.imported"))
        .map_err(|e| format!("Imported ideas.json but couldn't rename it: {e}"))?;
    Ok(ideas.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn planned(id: &str, created_at: u64, offer_id: Option<i64>, offer_name: &str) -> Idea {
        Idea {
            id: id.into(),
            source: "Agent plan".into(),
            offer_id,
            offer_name: Some(offer_name.into()),
            image_prompt: Some("desk".into()),
            ..Idea::manual(format!("Title {id}"), created_at)
        }
    }

    #[test]
    fn ideas_round_trip_newest_batch_first_in_planned_order() {
        let mut conn = memory_db();
        insert_ideas(&mut conn, &[planned("old", 100, Some(1), "Sprint")]).unwrap();
        insert_ideas(&mut conn, &[planned("new-a", 200, Some(1), "Sprint"), planned("new-b", 200, None, "Audit")]).unwrap();

        let ideas = list_ideas(&conn).unwrap();

        assert_eq!(ideas.iter().map(|idea| idea.id.as_str()).collect::<Vec<_>>(), ["new-a", "new-b", "old"]);
        assert_eq!(ideas[0], planned("new-a", 200, Some(1), "Sprint"));
    }

    #[test]
    fn drafts_are_paged_newest_first_and_a_page_past_the_end_falls_back() {
        let mut conn = memory_db();
        for n in 1..=5 {
            insert_ideas(&mut conn, &[planned(&format!("d{n}"), n * 100, None, "Sprint")]).unwrap();
        }
        insert_ideas(&mut conn, &[planned("sent", 900, None, "Sprint")]).unwrap();
        mark_sent(&conn, "sent", Some(1), 1000).unwrap();
        let ids = |page: &IdeaPage| page.ideas.iter().map(|idea| idea.id.clone()).collect::<Vec<_>>();

        let first = list_ideas_page(&conn, false, 1, 2).unwrap();
        let beyond = list_ideas_page(&conn, false, 9, 2).unwrap();

        assert_eq!(ids(&first), ["d5", "d4"]);
        assert_eq!((first.page, first.last_page, first.total), (1, 3, 5));
        assert_eq!((first.draft_count, first.sent_count), (5, 1));
        assert_eq!(ids(&beyond), ["d1"]);
        assert_eq!(beyond.page, 3);
    }

    #[test]
    fn sent_ideas_are_paged_by_the_latest_sent_first() {
        let mut conn = memory_db();
        insert_ideas(&mut conn, &[planned("a", 300, None, "Sprint"), planned("b", 200, None, "Sprint")]).unwrap();
        insert_ideas(&mut conn, &[planned("c", 100, None, "Sprint")]).unwrap();
        mark_sent(&conn, "a", None, 10).unwrap();
        mark_sent(&conn, "c", None, 30).unwrap();
        mark_sent(&conn, "b", None, 20).unwrap();

        let page = list_ideas_page(&conn, true, 1, 2).unwrap();
        let empty = list_ideas_page(&conn, false, 1, 2).unwrap();

        assert_eq!(page.ideas.iter().map(|idea| idea.id.as_str()).collect::<Vec<_>>(), ["c", "b"]);
        assert_eq!((page.last_page, page.total), (2, 3));
        assert_eq!((empty.ideas.len(), empty.page, empty.last_page, empty.total), (0, 1, 1, 0));
    }

    #[test]
    fn recent_titles_come_newest_first_and_stop_at_the_limit() {
        let mut conn = memory_db();
        insert_ideas(&mut conn, &[planned("old", 100, None, "Sprint")]).unwrap();
        insert_ideas(&mut conn, &[planned("new-a", 200, None, "Sprint"), planned("new-b", 200, None, "Sprint")]).unwrap();

        assert_eq!(recent_idea_titles(&conn, 2).unwrap(), ["Title new-a", "Title new-b"]);
    }

    #[test]
    fn editing_an_idea_changes_its_fields_and_keeps_the_rest() {
        let mut conn = memory_db();
        insert_ideas(&mut conn, &[planned("a", 1, Some(7), "Sprint")]).unwrap();

        let edited = update_idea(
            &conn,
            &IdeaEdit {
                id: "a".into(),
                title: "Sharper hook".into(),
                short_notes: "notes".into(),
                pain_addressed: "pain".into(),
                target_keyword: "keyword".into(),
                pattern: "Founder lesson".into(),
                tag: "Ops".into(),
                channel_id: Some("x_profile".into()),
                channel_name: Some("X · account".into()),
                suggested_draft: "New draft".into(),
                image_prompt: None,
                publish_channels: vec![ChannelSchedule {
                    id: "linkedin_personal".into(),
                    scheduled_date: Some("2026-12-01".into()),
                    scheduled_time: Some("09:00".into()),
                }],
            },
        )
        .unwrap();

        assert_eq!(edited.title, "Sharper hook");
        assert_eq!(edited.publish_channels[0].scheduled_date.as_deref(), Some("2026-12-01"));
        assert_eq!(edited.channel_id.as_deref(), Some("x_profile"));
        assert_eq!(edited.suggested_draft, "New draft");
        assert_eq!(edited.image_prompt, None);
        assert_eq!(edited.offer_id, Some(7));
        assert_eq!(list_ideas(&conn).unwrap()[0], edited);
    }

    #[test]
    fn a_sent_idea_remembers_when_and_where_it_went() {
        let mut conn = memory_db();
        insert_ideas(&mut conn, &[planned("a", 1, None, "Sprint")]).unwrap();

        let sent = mark_sent(&conn, "a", Some(42), 999).unwrap();

        assert_eq!((sent.sent_at, sent.remote_id), (Some(999), Some(42)));
        assert!(mark_sent(&conn, "missing", None, 1).unwrap_err().contains("no longer exists"));
    }

    #[test]
    fn a_published_idea_remembers_when_and_patterns_are_listed_once() {
        let mut conn = memory_db();
        let mut b = planned("b", 1, None, "Sprint");
        b.pattern = "Contrarian breakdown".into();
        let mut c = planned("c", 1, None, "Sprint");
        c.pattern = "Contrarian breakdown".into();
        insert_ideas(&mut conn, &[planned("a", 1, None, "Sprint"), b, c]).unwrap();

        let published = mark_published(&conn, "a", 500).unwrap();

        assert_eq!(published.published_at, Some(500));
        assert_eq!(list_patterns(&conn).unwrap(), ["Contrarian breakdown"]);
    }

    #[test]
    fn a_database_from_the_first_release_gains_the_sent_columns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE ideas (id TEXT PRIMARY KEY, title TEXT NOT NULL, short_notes TEXT NOT NULL DEFAULT '', \
             pain_addressed TEXT NOT NULL DEFAULT '', target_keyword TEXT NOT NULL DEFAULT '', \
             pattern TEXT NOT NULL DEFAULT '', tag TEXT NOT NULL DEFAULT '', source TEXT NOT NULL, \
             channel_id TEXT, channel_name TEXT, scheduled_date TEXT, scheduled_time TEXT, offer_id INTEGER, \
             offer_name TEXT, suggested_draft TEXT NOT NULL DEFAULT '', image_prompt TEXT, \
             status TEXT NOT NULL DEFAULT 'draft', provider TEXT, created_at INTEGER NOT NULL);
             INSERT INTO ideas (id, title, source, created_at) VALUES ('old', 'Old idea', 'Manual', 1);",
        )
        .unwrap();

        migrate(&conn).unwrap();
        migrate(&conn).unwrap();

        let ideas = list_ideas(&conn).unwrap();
        assert_eq!(ideas[0].title, "Old idea");
        assert_eq!(ideas[0].sent_at, None);
        assert!(ideas[0].publish_channels.is_empty());
        assert_eq!(ideas[0].published_at, None);
        assert_eq!(ideas[0].channel_platform, None);
        assert_eq!(ideas[0].topic_title, None);
    }

    #[test]
    fn planned_ideas_from_before_the_platform_column_get_their_platform() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE ideas (id TEXT PRIMARY KEY, title TEXT NOT NULL, short_notes TEXT NOT NULL DEFAULT '', \
             pain_addressed TEXT NOT NULL DEFAULT '', target_keyword TEXT NOT NULL DEFAULT '', \
             pattern TEXT NOT NULL DEFAULT '', tag TEXT NOT NULL DEFAULT '', source TEXT NOT NULL, \
             channel_id TEXT, channel_name TEXT, scheduled_date TEXT, scheduled_time TEXT, offer_id INTEGER, \
             offer_name TEXT, suggested_draft TEXT NOT NULL DEFAULT '', image_prompt TEXT, \
             status TEXT NOT NULL DEFAULT 'draft', provider TEXT, created_at INTEGER NOT NULL);
             INSERT INTO ideas (id, title, source, channel_id, created_at) VALUES
                 ('li', 'A', 'Agent plan', 'linkedin_personal', 4),
                 ('fb', 'B', 'Agent plan', 'facebook_page', 3),
                 ('x', 'C', 'Agent plan', 'x_profile', 2),
                 ('web', 'D', 'Agent plan', 'website_blog', 1),
                 ('real', 'E', 'Agent plan', '42', 0);",
        )
        .unwrap();

        migrate(&conn).unwrap();

        let platforms: Vec<Option<String>> = list_ideas(&conn).unwrap().into_iter().map(|idea| idea.channel_platform).collect();
        assert_eq!(
            platforms,
            [Some("LinkedIn".into()), Some("Facebook".into()), Some("X".into()), Some("Website".into()), None]
        );
    }

    #[test]
    fn deleting_an_idea_removes_only_that_idea() {
        let mut conn = memory_db();
        insert_ideas(&mut conn, &[planned("a", 1, None, "Sprint"), planned("b", 1, None, "Sprint")]).unwrap();

        delete_idea(&conn, "a").unwrap();

        assert_eq!(list_ideas(&conn).unwrap().len(), 1);
        assert_eq!(list_ideas(&conn).unwrap()[0].id, "b");
    }

    #[test]
    fn idea_counts_are_grouped_per_offer_and_skip_manual_ideas() {
        let mut conn = memory_db();
        insert_ideas(
            &mut conn,
            &[
                planned("a", 1, Some(7), "Sprint"),
                planned("b", 1, Some(7), "Sprint"),
                planned("c", 1, None, "Legacy offer"),
                Idea::manual("typed by hand".into(), 2),
            ],
        )
        .unwrap();

        let mut counts = offer_idea_counts(&conn).unwrap();
        counts.sort_by_key(|count| count.count);

        assert_eq!(
            counts,
            [
                OfferIdeaCount { offer_id: None, offer_name: Some("Legacy offer".into()), count: 1 },
                OfferIdeaCount { offer_id: Some(7), offer_name: Some("Sprint".into()), count: 2 },
            ]
        );
    }

    #[test]
    fn permissions_are_granted_once_per_agent_and_can_be_revoked() {
        let conn = memory_db();
        grant_permissions(&conn, "antigravity", &["write_file".into(), "write_file".into()]).unwrap();
        grant_permissions(&conn, "claude", &["Write".into()]).unwrap();

        assert_eq!(allowed_tools(&conn, "antigravity").unwrap(), ["write_file"]);

        revoke_permission(&conn, "antigravity", "write_file").unwrap();

        assert!(allowed_tools(&conn, "antigravity").unwrap().is_empty());
        assert_eq!(allowed_tools(&conn, "claude").unwrap(), ["Write"]);
    }

    #[test]
    fn the_first_skill_is_seeded_once_as_the_default() {
        let conn = memory_db();
        seed_skills(&conn, "RULES", 1).unwrap();
        seed_skills(&conn, "OTHER", 2).unwrap();

        let skills = list_skills(&conn).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!((skills[0].content.as_str(), skills[0].is_default), ("RULES", true));
        assert_eq!(default_skill(&conn).unwrap(), skills[0]);
    }

    #[test]
    fn skills_can_be_added_edited_made_default_and_deleted() {
        let conn = memory_db();
        seed_skills(&conn, "RULES", 1).unwrap();
        let punchy = create_skill(&conn, "Punchy", "SHORT", 2).unwrap();

        let edited = update_skill(&conn, &punchy.id, "Punchy X", "SHORTER", 3).unwrap();
        assert_eq!((edited.name.as_str(), edited.content.as_str(), edited.updated_at), ("Punchy X", "SHORTER", 3));

        set_default_skill(&conn, &punchy.id).unwrap();
        assert_eq!(default_skill(&conn).unwrap().id, punchy.id);
        assert_eq!(list_skills(&conn).unwrap().iter().filter(|skill| skill.is_default).count(), 1);
        assert!(delete_skill(&conn, &punchy.id).unwrap_err().contains("default"));

        let first = list_skills(&conn).unwrap().into_iter().find(|skill| !skill.is_default).unwrap();
        delete_skill(&conn, &first.id).unwrap();
        assert_eq!(list_skills(&conn).unwrap().len(), 1);
    }

    #[test]
    fn channel_skills_are_set_cleared_and_dropped_with_their_skill() {
        let conn = memory_db();
        seed_skills(&conn, "RULES", 1).unwrap();
        let punchy = create_skill(&conn, "Punchy", "SHORT", 2).unwrap();

        set_channel_skill(&conn, "x_profile", Some(&punchy.id)).unwrap();
        set_channel_skill(&conn, "facebook_page", Some(&punchy.id)).unwrap();
        set_channel_skill(&conn, "facebook_page", None).unwrap();
        assert_eq!(channel_skills(&conn).unwrap(), [ChannelSkill { channel_id: "x_profile".into(), skill_id: punchy.id.clone() }]);
        assert!(set_channel_skill(&conn, "x_profile", Some("missing")).is_err());

        delete_skill(&conn, &punchy.id).unwrap();
        assert!(channel_skills(&conn).unwrap().is_empty());
    }

    #[test]
    fn settings_are_saved_replaced_and_removed() {
        let conn = memory_db();
        assert_eq!(get_setting(&conn, "image_api").unwrap(), None);
        set_setting(&conn, "image_api", Some("a")).unwrap();
        set_setting(&conn, "image_api", Some("b")).unwrap();
        assert_eq!(get_setting(&conn, "image_api").unwrap().as_deref(), Some("b"));
        set_setting(&conn, "image_api", None).unwrap();
        assert_eq!(get_setting(&conn, "image_api").unwrap(), None);
    }

    #[test]
    fn an_idea_image_can_be_set_and_reports_the_file_it_replaced() {
        let mut conn = memory_db();
        insert_ideas(&mut conn, &[planned("a", 1, None, "Sprint")]).unwrap();

        let (idea, previous) = set_idea_image(&conn, "a", Some("a-1.png")).unwrap();
        assert_eq!((idea.image_file.as_deref(), previous), (Some("a-1.png"), None));

        let (idea, previous) = set_idea_image(&conn, "a", None).unwrap();
        assert_eq!((idea.image_file, previous.as_deref()), (None, Some("a-1.png")));
    }

    #[test]
    fn legacy_json_ideas_are_imported_once() {
        let dir = std::env::temp_dir().join(format!("ge-db-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let json_path = dir.join("ideas.json");
        fs::write(
            &json_path,
            r#"[{"id":"idea-1","title":"Old idea","source":"Agent plan","offerName":"Sprint","createdAt":5}]"#,
        )
        .unwrap();
        let mut conn = memory_db();

        let first = import_legacy_json(&mut conn, &json_path).unwrap();
        let second = import_legacy_json(&mut conn, &json_path).unwrap();
        let ideas = list_ideas(&conn).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!((first, second), (1, 0));
        assert_eq!(ideas[0].title, "Old idea");
        assert_eq!(ideas[0].offer_name.as_deref(), Some("Sprint"));
    }

    fn offer_draft(id: &str, created_at: u64) -> OfferDraft {
        OfferDraft {
            id: id.into(),
            name: format!("Offer {id}"),
            promise: "Faster estimates".into(),
            description: String::new(),
            price_range: String::new(),
            timeline: String::new(),
            deliverables: vec!["Workflow".into()],
            icp_ids: vec![3],
            case_study_ids: Vec::new(),
            rationale: String::new(),
            provider: Some("claude".into()),
            created_at,
            updated_at: created_at,
            synced_at: None,
            remote_id: None,
        }
    }

    #[test]
    fn offer_drafts_are_listed_until_synced_and_can_be_edited() {
        let mut conn = memory_db();
        insert_offer_drafts(&mut conn, &[offer_draft("a", 1), offer_draft("b", 2), offer_draft("c", 2)]).unwrap();

        let ids = |conn: &Connection| -> Vec<String> { pending_offer_drafts(conn).unwrap().into_iter().map(|draft| draft.id).collect() };
        assert_eq!(ids(&conn), ["b", "c", "a"]);
        assert_eq!(pending_offer_drafts(&conn).unwrap()[0].icp_ids, [3]);

        let edit = OfferDraftEdit {
            id: "a".into(),
            name: "Same-day estimates".into(),
            promise: "Quotes out today".into(),
            description: "How".into(),
            price_range: "$5k".into(),
            timeline: "2 weeks".into(),
            deliverables: vec!["One".into(), "Two".into()],
            icp_ids: Vec::new(),
            case_study_ids: vec![9],
        };
        let edited = update_offer_draft(&conn, &edit, 50).unwrap();
        assert_eq!(edited.name, "Same-day estimates");
        assert_eq!(edited.updated_at, 50);
        assert_eq!(edited.case_study_ids, [9]);

        mark_offer_synced(&conn, "b", Some(77), 99).unwrap();
        delete_offer_draft(&conn, "c").unwrap();

        assert_eq!(ids(&conn), ["a"]);
        assert_eq!(get_offer_draft(&conn, "b").unwrap().remote_id, Some(77));
        assert_eq!(offer_draft_names(&conn).unwrap().len(), 2);
        assert!(get_offer_draft(&conn, "c").unwrap_err().contains("no longer exists"));
    }

    #[test]
    fn post_analytics_round_trips_and_updates_on_conflict() {
        let conn = memory_db();
        let initial = crate::analytics::PostAnalytics {
            post_id: "post-1".into(),
            platform_post_id: Some("urn:li:share:123".into()),
            post_url: Some("https://linkedin.com/...".into()),
            channel_id: "linkedin_personal".into(),
            platform: "LinkedIn".into(),
            impressions: crate::analytics::MetricValue::available(120),
            views: crate::analytics::MetricValue::available(95),
            likes: crate::analytics::MetricValue::available(10),
            comments: crate::analytics::MetricValue::available(2),
            latest_comments: vec![crate::analytics::PostComment {
                id: "c1".into(),
                author: "Alice".into(),
                author_avatar: None,
                text: "Nice!".into(),
                created_at: None,
            }],
            last_updated: 5000,
            error: None,
        };

        save_post_analytics(&conn, &initial).unwrap();
        let retrieved = get_post_analytics(&conn, "post-1").unwrap().unwrap();
        assert_eq!(retrieved, initial);

        // Update with fresh metrics
        let mut updated = initial.clone();
        updated.likes = crate::analytics::MetricValue::available(15);
        updated.last_updated = 6000;
        save_post_analytics(&conn, &updated).unwrap();

        let retrieved_updated = get_post_analytics(&conn, "post-1").unwrap().unwrap();
        assert_eq!(retrieved_updated.likes.value, Some(15));
        assert_eq!(retrieved_updated.last_updated, 6000);
    }

    #[test]
    fn post_platform_id_can_be_set_and_retrieved() {
        let conn = memory_db();
        assert_eq!(get_post_platform_id(&conn, "post-42").unwrap(), None);

        set_post_platform_id(&conn, "post-42", "urn:li:activity:999", Some("https://linkedin.com/feed")).unwrap();
        let (pid, url) = get_post_platform_id(&conn, "post-42").unwrap().unwrap();
        assert_eq!(pid, "urn:li:activity:999");
        assert_eq!(url.as_deref(), Some("https://linkedin.com/feed"));
    }
}
