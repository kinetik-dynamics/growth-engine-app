//! Local SQLite database (growth-engine.db in the app-data folder): the
//! generated ideas and the tool permissions granted to each AI agent.

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
";

const IDEA_COLUMNS: &str = "id, title, short_notes, pain_addressed, target_keyword, pattern, tag, source, \
    channel_id, channel_name, scheduled_date, scheduled_time, offer_id, offer_name, suggested_draft, \
    image_prompt, status, provider, created_at, sent_at, remote_id";

/// Columns added after the first release; `migrate` adds any that are missing.
const ADDED_IDEA_COLUMNS: [(&str, &str); 2] = [("sent_at", "INTEGER"), ("remote_id", "INTEGER")];

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
                 (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)"
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
         image_prompt = ?11 WHERE id = ?1",
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
        ],
    )
    .map_err(describe)?;
    get_idea(conn, &edit.id)
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
            },
        )
        .unwrap();

        assert_eq!(edited.title, "Sharper hook");
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
}
