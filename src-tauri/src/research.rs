//! Builds the lead-research prompt and reads back what the AI found.
//!
//! The queue holds two kinds of item: leads (people, from LinkedIn or Google
//! Maps) and prospects (companies, from the onboarding scan). The agent looks
//! up whatever each one is missing on the web. The user's own instructions
//! live in Settings; the app adds the leads and the reply format itself, so
//! an edited prompt can never break parsing.

use serde::Deserialize;
use serde_json::{json, Map, Value};

/// The built-in instructions, shown in Settings until the user edits them.
pub const DEFAULT_PROMPT: &str = include_str!("../default-research-prompt.md");

/// How many leads one agent run researches.
pub const BATCH_SIZE: usize = 5;

const MAX_ABOUT_CHARS: usize = 500;

/// True for null, blank text and empty lists.
fn is_blank(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

fn is_lead(item: &Value) -> bool {
    item["type"] == "lead"
}

/// The fields this queue item has no value for yet.
pub fn missing_fields(item: &Value) -> Vec<&'static str> {
    let wanted: &[&'static str] = if is_lead(item) {
        &["email", "phone", "website", "location", "category", "rating", "company_name", "company_details"]
    } else {
        &["email", "phone", "location", "category", "company_details", "contacts"]
    };
    wanted.iter().copied().filter(|field| is_blank(&item[*field])).collect()
}

/// One queue item picked by hand for research.
#[derive(Deserialize, Clone, Debug)]
pub struct Target {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: i64,
}

/// A queue item with the fields it is missing.
pub type Work = (Value, Vec<&'static str>);

/// The items worth researching. Items picked by hand are always researched
/// (if they have gaps). Otherwise items researched before are skipped unless
/// `include_researched`, and `limit` caps how many are taken.
pub fn pick_work(items: Vec<Value>, only: &[Target], include_researched: bool, limit: Option<usize>) -> Vec<Work> {
    let mut work: Vec<Work> = items
        .into_iter()
        .filter(|item| {
            if !only.is_empty() {
                return only.iter().any(|target| item["id"].as_i64() == Some(target.id) && item["type"] == target.kind.as_str());
            }
            include_researched || is_blank(&item["researched_at"])
        })
        .filter_map(|item| {
            let missing = missing_fields(&item);
            (!missing.is_empty()).then_some((item, missing))
        })
        .collect();
    if only.is_empty() {
        if let Some(limit) = limit {
            work.truncate(limit);
        }
    }
    work
}

/// An ISO-8601 UTC timestamp for epoch milliseconds, so the UI can read it
/// like the timestamps the main site sends.
pub fn iso_utc(ms: u64) -> String {
    let secs = ms / 1000;
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// Shows unsynced findings on the queue items they belong to, the way the
/// main site will once they are synced: only blank fields are filled.
pub fn overlay(items: &mut [Value], pending: &[crate::db::PendingResearch]) {
    for item in items {
        let Some(found) = pending.iter().find(|found| item["id"].as_i64() == Some(found.id) && item["type"] == found.kind.as_str()) else {
            continue;
        };
        if let Some(fields) = found.payload.as_object() {
            for (field, value) in fields.iter().filter(|(field, _)| !matches!(field.as_str(), "type" | "id")) {
                if is_blank(&item[field.as_str()]) {
                    item[field.as_str()] = value.clone();
                }
            }
        }
        item["researched_at"] = json!(iso_utc(found.researched_at));
        item["unsynced"] = json!(true);
    }
}

/// What the prompt tells the agent about one item.
pub fn brief(item: &Value, missing: &[&str]) -> Value {
    let mut known = Map::new();
    known.insert("type".into(), item["type"].clone());
    known.insert("id".into(), item["id"].clone());
    let name = if is_lead(item) { &item["contact_name"] } else { &item["company_name"] };
    known.insert("name".into(), name.clone());
    for field in [
        "company_name", "contact_title", "headline", "email", "phone", "website", "company_domain", "location", "category",
        "linkedin_url", "maps_url", "description",
    ] {
        if !is_blank(&item[field]) && !known.contains_key(field) {
            known.insert(field.into(), item[field].clone());
        }
    }
    if let Some(about) = item["about"].as_str().filter(|about| !about.trim().is_empty()) {
        known.insert("about".into(), json!(about.trim().chars().take(MAX_ABOUT_CHARS).collect::<String>()));
    }
    if let Some(icp) = item["icp"]["name"].as_str() {
        known.insert("icp".into(), json!(icp));
    }
    known.insert("missing".into(), json!(missing));
    Value::Object(known)
}

pub fn build_prompt(instructions: &str, batch: &[Value]) -> String {
    format!(
        "{}\n\n# Leads to research\n\n{}\n\n# Output\n\nReply with ONLY a JSON array, one object per lead above, in the same order. \
         No markdown fences, no commentary. Each object has:\n\
         - \"type\" and \"id\": copied exactly from the lead\n\
         - \"email\", \"phone\", \"website\", \"location\", \"category\", \"company_name\": text, or \"\" when not found\n\
         - \"rating\": a number from 0 to 5, or null\n\
         - \"reviews_count\": a whole number, or null\n\
         - \"company_details\": the company profile as plain text, or \"\"\n\
         - \"contacts\": an array of {{\"name\", \"role\", \"email\", \"phone\", \"linkedin_url\"}} for the people you found, or []\n\
         Leave a field empty rather than guess.\n",
        instructions.trim(),
        serde_json::to_string_pretty(batch).unwrap_or_default(),
    )
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
pub struct Contact {
    pub name: String,
    pub role: String,
    pub email: String,
    pub phone: String,
    pub linkedin_url: String,
}

/// One item as the AI reports it.
#[derive(Deserialize, Default, Debug)]
#[serde(default)]
pub struct Finding {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: i64,
    pub email: String,
    pub phone: String,
    pub website: String,
    pub location: String,
    pub category: String,
    pub company_name: String,
    pub company_details: String,
    pub rating: Value,
    pub reviews_count: Value,
    pub contacts: Vec<Contact>,
}

pub fn parse_findings(output: &str) -> Result<Vec<Finding>, String> {
    crate::plan::parse_array(output, |finding: &Finding| finding.id > 0 && (finding.kind == "lead" || finding.kind == "prospect"), "research results")
}

/// A number the AI may have sent as text.
fn number(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
}

/// The body sent to the main site: the item's id plus only the fields that
/// were missing and that the agent found a value for.
pub fn payload(finding: &Finding, missing: &[&str]) -> Value {
    let mut body = Map::new();
    body.insert("type".into(), json!(finding.kind));
    body.insert("id".into(), json!(finding.id));
    let wants = |field: &str| missing.contains(&field);

    for (field, value) in [
        ("email", &finding.email),
        ("phone", &finding.phone),
        ("website", &finding.website),
        ("location", &finding.location),
        ("category", &finding.category),
        ("company_name", &finding.company_name),
        ("company_details", &finding.company_details),
    ] {
        if wants(field) && !value.trim().is_empty() {
            body.insert(field.into(), json!(value.trim()));
        }
    }
    if wants("rating") {
        if let Some(rating) = number(&finding.rating).filter(|rating| (0.0..=5.0).contains(rating)) {
            body.insert("rating".into(), json!(rating));
            if let Some(reviews) = number(&finding.reviews_count).filter(|reviews| *reviews >= 0.0) {
                body.insert("reviews_count".into(), json!(reviews as i64));
            }
        }
    }
    if wants("contacts") {
        let contacts: Vec<Value> = finding
            .contacts
            .iter()
            .filter(|contact| !contact.name.trim().is_empty())
            .map(|contact| {
                json!({
                    "name": contact.name.trim(),
                    "role": contact.role.trim(),
                    "email": contact.email.trim(),
                    "phone": contact.phone.trim(),
                    "linkedin_url": contact.linkedin_url.trim(),
                })
            })
            .collect();
        if !contacts.is_empty() {
            body.insert("contacts".into(), json!(contacts));
        }
    }
    Value::Object(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leads_and_companies_are_missing_different_fields() {
        let lead = json!({ "type": "lead", "id": 1, "email": "a@b.co", "phone": " ", "website": null, "location": "NYC", "category": "Jeweler", "rating": 4.8, "company_name": "Acme", "company_details": null });
        assert_eq!(missing_fields(&lead), ["phone", "website", "company_details"]);

        let company = json!({ "type": "prospect", "id": 2, "email": null, "phone": null, "location": "SF", "category": null, "company_details": "x", "contacts": [] });
        assert_eq!(missing_fields(&company), ["email", "phone", "category", "contacts"]);
    }

    #[test]
    fn timestamps_are_iso_utc() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_795_573_021_516), "2026-11-25T02:17:01Z");
    }

    #[test]
    fn unsynced_findings_fill_blanks_only_and_mark_the_item() {
        let mut items = vec![json!({ "type": "lead", "id": 7, "email": "keep@x.co", "phone": null }), json!({ "type": "prospect", "id": 7 })];
        let pending = [crate::db::PendingResearch {
            kind: "lead".into(),
            id: 7,
            payload: json!({ "type": "lead", "id": 7, "email": "new@x.co", "phone": "+1 555" }),
            researched_at: 0,
        }];
        overlay(&mut items, &pending);
        assert_eq!((items[0]["email"].as_str(), items[0]["phone"].as_str()), (Some("keep@x.co"), Some("+1 555")));
        assert_eq!(items[0]["unsynced"], true);
        assert_eq!(items[0]["researched_at"], "1970-01-01T00:00:00Z");
        assert!(items[1].get("unsynced").is_none(), "same id, other type");
    }

    #[test]
    fn work_skips_researched_items_and_honours_the_limit() {
        let items = vec![
            json!({ "type": "lead", "id": 1, "email": "a@b.co", "phone": "1", "website": "x", "location": "x", "category": "x", "rating": 5, "company_name": "x", "company_details": "x" }),
            json!({ "type": "lead", "id": 2, "researched_at": "2026-11-25T00:00:00Z" }),
            json!({ "type": "prospect", "id": 3 }),
            json!({ "type": "prospect", "id": 4 }),
        ];
        let ids = |work: Vec<Work>| work.iter().map(|(item, _)| item["id"].as_i64().unwrap()).collect::<Vec<_>>();

        assert_eq!(ids(pick_work(items.clone(), &[], false, None)), [3, 4], "complete and researched items are skipped");
        assert_eq!(ids(pick_work(items.clone(), &[], true, None)), [2, 3, 4]);
        assert_eq!(ids(pick_work(items.clone(), &[], false, Some(1))), [3]);

        let picked = [Target { kind: "lead".into(), id: 2 }];
        assert_eq!(ids(pick_work(items, &picked, false, Some(0))), [2], "a hand-picked item ignores both the history and the limit");
    }

    #[test]
    fn the_prompt_carries_the_instructions_the_leads_and_the_format() {
        let item = json!({ "type": "prospect", "id": 9, "company_name": "Acme", "company_domain": "acme.test", "about": "", "icp": { "id": 1, "name": "Founders" } });
        let prompt = build_prompt("  MY RULES  ", &[brief(&item, &["email"])]);
        assert!(prompt.starts_with("MY RULES\n"));
        assert!(prompt.contains("\"name\": \"Acme\""));
        assert!(prompt.contains("\"icp\": \"Founders\""));
        assert!(prompt.contains("\"missing\": [") && prompt.contains("\"email\""));
        assert!(prompt.contains("ONLY a JSON array"));
        assert!(!prompt.contains("\"about\""), "blank fields are left out");
    }

    #[test]
    fn only_missing_fields_with_values_are_sent_back() {
        let output = "```json\n[{\"type\":\"prospect\",\"id\":5,\"email\":\"ceo@acme.test\",\"phone\":\"\",\"category\":\"Software\",\"company_details\":\"Builds things.\",\
                      \"rating\":\"4.5\",\"contacts\":[{\"name\":\"Jo\",\"role\":\"CEO\"},{\"name\":\" \"}]}]\n```";
        let findings = parse_findings(output).unwrap();
        let body = payload(&findings[0], &["email", "phone", "contacts", "company_details"]);
        assert_eq!(body["email"], "ceo@acme.test");
        assert!(body.get("phone").is_none(), "blank values are dropped");
        assert!(body.get("category").is_none(), "fields that weren't missing are dropped");
        assert!(body.get("rating").is_none());
        assert_eq!(body["contacts"].as_array().unwrap().len(), 1);
        assert_eq!(body["contacts"][0]["role"], "CEO");
    }

    #[test]
    fn ratings_may_arrive_as_text_but_must_be_in_range() {
        let finding = Finding { kind: "lead".into(), id: 1, rating: json!("4.5"), reviews_count: json!(12), ..Default::default() };
        let body = payload(&finding, &["rating"]);
        assert_eq!((body["rating"].as_f64(), body["reviews_count"].as_i64()), (Some(4.5), Some(12)));
        let wild = Finding { kind: "lead".into(), id: 1, rating: json!(45), ..Default::default() };
        assert!(payload(&wild, &["rating"]).get("rating").is_none());
    }

    #[test]
    fn rejects_output_without_results() {
        assert!(parse_findings("I could not find anything.").unwrap_err().contains("research results"));
    }
}
