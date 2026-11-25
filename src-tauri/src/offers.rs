//! Builds the offer-generation prompt and parses the offers the AI sends back.
//!
//! An offer is not a service: it is the specific outcome the company takes to
//! one audience ("Automation of construction estimates", not "AI automation").
//! The agent drafts them from the company overview, its ICPs and its case
//! studies, all read from the main site.

use serde::Deserialize;
use serde_json::{json, Map, Value};

/// The most offers one run may ask for.
pub const MAX_OFFERS: u32 = 10;
/// Caps on what goes into the prompt, so it stays under the CLI's limits.
const MAX_ICPS: usize = 30;
const MAX_CASE_STUDIES: usize = 40;
const MAX_FIELD_CHARS: usize = 600;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferRequest {
    pub provider: String,
    pub count: u32,
    /// Optional steer, e.g. "offers for construction firms".
    #[serde(default)]
    pub focus: Option<String>,
    /// ICP ids to base the offers on; empty means every ICP.
    #[serde(default)]
    pub icp_ids: Vec<i64>,
    /// Case study ids to base the offers on; empty means every one.
    #[serde(default)]
    pub proof_ids: Vec<i64>,
    /// The offer skill whose instructions the agent follows.
    #[serde(default)]
    pub skill_id: Option<String>,
}

/// Keeps the items whose `id` is in `ids`; an empty `ids` keeps them all.
pub fn only_ids(items: Vec<Value>, ids: &[i64]) -> Vec<Value> {
    if ids.is_empty() {
        return items;
    }
    items.into_iter().filter(|item| item["id"].as_i64().is_some_and(|id| ids.contains(&id))).collect()
}

impl OfferRequest {
    pub fn focus(&self) -> Option<&str> {
        self.focus.as_deref().map(str::trim).filter(|focus| !focus.is_empty())
    }
}

/// One offer as the AI returns it.
#[derive(Deserialize, Default, Debug)]
#[serde(default)]
pub struct DraftedOffer {
    pub name: String,
    pub promise: String,
    pub description: String,
    pub price_range: String,
    pub timeline: String,
    pub deliverables: Vec<String>,
    pub icp_ids: Vec<i64>,
    pub case_study_ids: Vec<i64>,
    pub rationale: String,
}

/// Keeps only the listed fields of `item` that have a value, each cut to a
/// readable length.
fn pick(item: &Value, fields: &[&str]) -> Value {
    let mut picked = Map::new();
    for field in fields {
        let value = match &item[*field] {
            Value::String(text) if text.trim().is_empty() => continue,
            Value::String(text) => Value::String(text.trim().chars().take(MAX_FIELD_CHARS).collect()),
            Value::Null => continue,
            Value::Array(items) if items.is_empty() => continue,
            other => other.clone(),
        };
        picked.insert((*field).to_string(), value);
    }
    Value::Object(picked)
}

/// The ICPs as the prompt shows them.
pub fn brief_icps(icps: &[Value]) -> Vec<Value> {
    icps.iter()
        .take(MAX_ICPS)
        .map(|icp| pick(icp, &["id", "name", "who_they_are", "roles_to_contact", "pain", "buying_triggers", "criteria"]))
        .collect()
}

/// The case studies as the prompt shows them. Clients that may not be named
/// stay anonymous.
pub fn brief_case_studies(proof: &[Value]) -> Vec<Value> {
    proof
        .iter()
        .take(MAX_CASE_STUDIES)
        .map(|study| {
            let mut picked = pick(study, &["id", "title", "description", "client_type", "industry", "result"]);
            if study["can_name_client"].as_bool() == Some(true)
                && let Some(client) = study["client_name"].as_str().filter(|name| !name.trim().is_empty())
            {
                picked["client_name"] = json!(client);
            }
            if let Some(offer) = study["offer"]["name"].as_str() {
                picked["already_proves_offer"] = json!(offer);
            }
            picked
        })
        .collect()
}

pub fn build_prompt(
    company: &Value,
    icps: &[Value],
    case_studies: &[Value],
    existing_offers: &[String],
    count: u32,
    focus: Option<&str>,
    skill: Option<(&str, &str)>,
) -> String {
    let mut prompt = String::from(
        "You are the offer strategist for a B2B studio. Draft new offers the company can take to its \
         audience. Work only from the material in this message: do not use tools, read files, or ask questions.\n\n\
         # What an offer is\n\n\
         An offer is not a service. A service is what the company can do (\"AI Automation\"); an offer is the \
         specific, outcome-led package it approaches one audience with (\"Automation of construction estimates\"). \
         A good offer:\n\
         - names a concrete outcome for a specific ICP, in their words\n\
         - is backed by the company's real capabilities and, where possible, a case study\n\
         - is narrow enough that the buyer instantly knows it is for them\n\
         - has a clear promise, a short list of deliverables and a realistic timeline\n",
    );

    if let Some((name, content)) = skill {
        prompt.push_str(&format!(
            "\n# Offer skill: {name}\n\nFollow these instructions when you draft the offers. They override the general guidance above where they differ.\n\n{}\n",
            content.trim()
        ));
    }

    prompt.push_str("\n# Company overview\n\n");
    prompt.push_str(&serde_json::to_string_pretty(company).unwrap_or_default());

    prompt.push_str("\n\n# Ideal client profiles (ICPs)\n\n");
    if icps.is_empty() {
        prompt.push_str("None yet. Infer likely audiences from the company overview and case studies.");
    } else {
        prompt.push_str(&serde_json::to_string_pretty(icps).unwrap_or_default());
    }

    prompt.push_str("\n\n# Case studies (proof)\n\n");
    if case_studies.is_empty() {
        prompt.push_str("None yet. Keep promises grounded in the company overview.");
    } else {
        prompt.push_str(&serde_json::to_string_pretty(case_studies).unwrap_or_default());
    }

    if let Some(focus) = focus {
        prompt.push_str("\n\n# Focus for this run (every offer must fit it)\n\n");
        prompt.push_str(focus);
    }

    if !existing_offers.is_empty() {
        prompt.push_str("\n\n# Offers that already exist (do not repeat or lightly reword these)\n\n");
        for name in existing_offers {
            prompt.push_str(&format!("- {name}\n"));
        }
    }

    prompt.push_str(&format!(
        "\n\n# Output\n\nReply with ONLY a JSON array of exactly {count} objects. No markdown fences, no commentary. \
         Each offer must take a different angle (a different ICP, outcome or problem). Each object has:\n\
         - \"name\": the offer's name, outcome-led, under 80 characters\n\
         - \"promise\": one or two sentences on the result the buyer gets\n\
         - \"description\": a short paragraph on what is done and how\n\
         - \"price_range\": a price range if the material suggests one, else \"\"\n\
         - \"timeline\": how long it takes, e.g. \"4-6 weeks\", else \"\"\n\
         - \"deliverables\": an array of 3-5 short strings\n\
         - \"icp_ids\": an array of the ids of the ICPs above this offer is for (may be empty)\n\
         - \"case_study_ids\": an array of the ids of the case studies above that prove it (may be empty)\n\
         - \"rationale\": one sentence on why this offer fits this company and audience\n"
    ));

    prompt
}

/// Pulls the JSON array of offers out of the CLI output.
pub fn parse_offers(output: &str) -> Result<Vec<DraftedOffer>, String> {
    crate::plan::parse_array(
        output,
        |offer: &DraftedOffer| !offer.name.trim().is_empty() && !offer.promise.trim().is_empty(),
        "offers",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_offers_and_drops_ones_without_a_name_or_promise() {
        let output = "Sure:\n```json\n[{\"name\":\"Same-day estimates\",\"promise\":\"Quotes out today\",\"icp_ids\":[3],\
                      \"deliverables\":[\"Workflow\"]},{\"name\":\"No promise\"}]\n```";
        let offers = parse_offers(output).unwrap();
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].name, "Same-day estimates");
        assert_eq!(offers[0].icp_ids, [3]);
        assert_eq!(offers[0].deliverables, ["Workflow"]);
        assert!(offers[0].case_study_ids.is_empty());
    }

    #[test]
    fn rejects_output_without_offers() {
        let error = parse_offers("I need more information.").unwrap_err();
        assert!(error.contains("didn't return offers"));
        assert!(error.contains("I need more information."));
    }

    #[test]
    fn prompt_carries_the_material_focus_and_existing_offers() {
        let icps = brief_icps(&[json!({ "id": 7, "name": "Construction firms", "pain": "  ", "offers": [] })]);
        let prompt = build_prompt(
            &json!({ "name": "Kinetik" }),
            &icps,
            &[],
            &["AI Automation Sprint".into()],
            3,
            Some("Construction"),
            Some(("Sharp", "BE SHARP")),
        );
        assert!(prompt.contains("# Offer skill: Sharp\n\nFollow these instructions"));
        assert!(prompt.contains("BE SHARP"));

        assert!(prompt.contains("\"name\": \"Construction firms\""));
        assert!(!prompt.contains("\"pain\""), "blank fields are left out");
        assert!(prompt.contains("# Focus for this run (every offer must fit it)\n\nConstruction"));
        assert!(prompt.contains("- AI Automation Sprint\n"));
        assert!(prompt.contains("exactly 3 objects"));
        assert!(prompt.contains("None yet. Keep promises grounded"));
    }

    #[test]
    fn only_ids_filters_and_empty_keeps_all() {
        let items = vec![json!({ "id": 1 }), json!({ "id": 2 })];
        assert_eq!(only_ids(items.clone(), &[]).len(), 2);
        assert_eq!(only_ids(items, &[2]), vec![json!({ "id": 2 })]);
    }

    #[test]
    fn anonymous_clients_stay_anonymous() {
        let studies = brief_case_studies(&[
            json!({ "id": 1, "title": "A", "client_name": "Acme", "can_name_client": false }),
            json!({ "id": 2, "title": "B", "client_name": "Globex", "can_name_client": true, "offer": { "id": 4, "name": "Sprint" } }),
        ]);
        assert!(studies[0].get("client_name").is_none());
        assert_eq!(studies[1]["client_name"], "Globex");
        assert_eq!(studies[1]["already_proves_offer"], "Sprint");
    }
}
