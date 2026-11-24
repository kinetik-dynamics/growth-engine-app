mod analytics;
mod api;
mod cli;
mod db;
mod image;
mod offers;
mod plan;
mod store;

use db::Idea;
use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use store::{Config, Session};
use tauri::{AppHandle, Emitter};

/// How long a weekly plan may take before the CLI is stopped.
const PLAN_TIMEOUT: Duration = Duration::from_secs(600);
const TEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Set by `cancel_plan` to stop the plan that is currently generating.
static CANCEL_PLAN: AtomicBool = AtomicBool::new(false);

/// One line of the live activity feed shown while a plan is generating.
#[derive(Clone, Serialize)]
struct PlanProgress {
    /// "stage" | "text" | "tool" | "denied" | "log"
    kind: &'static str,
    message: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum PlanOutcome {
    Created { ideas: Vec<Idea> },
    /// The agent was refused tools and returned no usable ideas; the user
    /// can allow them and run the plan again.
    PermissionNeeded { provider: String, tools: Vec<String> },
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or_default()
}

fn signed_in(app: &AppHandle) -> Result<Config, String> {
    store::load_config(app)?.ok_or_else(|| "You're signed out. Activate the app with a token first.".to_string())
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

fn report(app: &AppHandle, kind: &'static str, message: impl Into<String>) {
    let _ = app.emit("plan-progress", PlanProgress { kind, message: message.into() });
}

fn report_progress(app: &AppHandle, progress: cli::Progress) {
    match progress {
        cli::Progress::Text(text) => report(app, "text", text),
        cli::Progress::Tool(tool) => report(app, "tool", format!("Using tool: {tool}")),
        cli::Progress::Denied { tool, message } => {
            report(app, "denied", format!("Permission denied for \"{tool}\". {message}"))
        }
        cli::Progress::Log(line) => report(app, "log", line),
    }
}

#[tauri::command]
fn get_session(app: AppHandle) -> Result<Option<Session>, String> {
    Ok(store::load_config(&app)?.map(|config| config.session))
}

/// Verifies the token against the server and remembers it.
#[tauri::command]
async fn login(app: AppHandle, base_url: String, token: String) -> Result<Session, String> {
    let base_url = api::normalize_base_url(&base_url)?;
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("Paste your access token to continue.".into());
    }

    let me = api::get(&base_url, &token, "/api/me").await?;
    let session = Session {
        base_url,
        user_name: text(&me["user"]["name"]).unwrap_or_default(),
        user_email: text(&me["user"]["email"]).unwrap_or_default(),
        company_name: text(&me["company"]["name"]),
        website_url: text(&me["company"]["website_url"]),
    };
    store::save_config(&app, &Config { token, session: session.clone() })?;
    Ok(session)
}

#[tauri::command]
fn logout(app: AppHandle) -> Result<(), String> {
    store::clear_config(&app)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn list_offers(
    app: AppHandle,
    page: Option<u32>,
    search: Option<String>,
    icp_id: Option<u32>,
    pricing: Option<String>,
    source: Option<String>,
    sort: Option<String>,
) -> Result<Value, String> {
    let config = signed_in(&app)?;
    let mut query = vec![("page", page.unwrap_or(1).max(1).to_string())];
    if let Some(icp_id) = icp_id {
        query.push(("icp_id", icp_id.to_string()));
    }
    // The server validates these values, so they are passed through as-is.
    for (key, value) in [("search", search), ("pricing", pricing), ("source", source), ("sort", sort)] {
        if let Some(value) = value.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()) {
            query.push((key, value));
        }
    }
    // The server answers `{ offers, meta, stats, icp_options, company }`: one page, newest first by default.
    api::get_query(&config.session.base_url, &config.token, "/api/offers", &query).await
}

/// One of the main site's read-only pages, as the JSON its `/api/app/*` endpoint sends.
#[tauri::command]
async fn site_page(app: AppHandle, page: String) -> Result<Value, String> {
    const PAGES: [&str; 4] = ["overview", "icps", "proof", "publish"];
    if !PAGES.contains(&page.as_str()) {
        return Err(format!("Unknown page: {page}"));
    }
    let config = signed_in(&app)?;
    let mut result = api::get(&config.session.base_url, &config.token, &format!("/api/app/{page}")).await?;
    if page == "publish" {
        if let Ok(conn) = store::database(&app) {
            if let Some(posts) = result["posts"].as_array_mut() {
                for post in posts {
                    if let Some(post_id) = post["id"].as_str() {
                        if let Ok(Some((pid, url))) = db::get_post_platform_id(&conn, post_id) {
                            if post["platformPostId"].is_null() && post["platform_post_id"].is_null() {
                                post["platformPostId"] = Value::String(pid);
                            }
                            if post["postUrl"].is_null() && post["post_url"].is_null() {
                                if let Some(url) = url {
                                    post["postUrl"] = Value::String(url);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(result)
}

#[tauri::command]
fn offer_idea_counts(app: AppHandle) -> Result<Vec<db::OfferIdeaCount>, String> {
    db::offer_idea_counts(&store::database(&app)?)
}

#[tauri::command]
async fn detect_clis() -> Result<Vec<cli::CliStatus>, String> {
    tauri::async_runtime::spawn_blocking(cli::detect_all)
        .await
        .map_err(|e| e.to_string())
}

/// Runs a one-word prompt through the CLI to prove it is signed in and working.
#[tauri::command]
async fn test_cli(app: AppHandle, provider: String) -> Result<String, String> {
    let workspace = store::workspace_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let options = cli::RunOptions { cwd: &workspace, timeout: TEST_TIMEOUT, allowed_tools: &[], cancel: None };
        cli::prompt(&provider, "Reply with only the word OK and nothing else.", &options, &mut |_| {})
            .map(|result| result.text)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn list_permissions(app: AppHandle) -> Result<Vec<db::Permission>, String> {
    db::list_permissions(&store::database(&app)?)
}

#[tauri::command]
fn grant_permissions(app: AppHandle, provider: String, tools: Vec<String>) -> Result<(), String> {
    db::grant_permissions(&store::database(&app)?, &provider, &tools)
}

#[tauri::command]
fn revoke_permission(app: AppHandle, provider: String, tool: String) -> Result<(), String> {
    db::revoke_permission(&store::database(&app)?, &provider, &tool)
}

/* Idea skills */

const SKILL_NAME_MAX: usize = 80;

/// Trims the name and content of a skill being saved, and checks both.
fn checked_skill(name: &str, content: &str) -> Result<(String, String), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Give the skill a name.".into());
    }
    if name.chars().count() > SKILL_NAME_MAX {
        return Err(format!("Keep the skill name under {SKILL_NAME_MAX} characters."));
    }
    if content.trim().is_empty() {
        return Err("Write the skill's instructions first.".into());
    }
    Ok((name.to_string(), content.to_string()))
}

#[tauri::command]
fn list_skills(app: AppHandle) -> Result<Vec<db::Skill>, String> {
    db::list_skills(&store::database(&app)?)
}

#[tauri::command]
fn create_skill(app: AppHandle, name: String, content: String) -> Result<db::Skill, String> {
    let (name, content) = checked_skill(&name, &content)?;
    db::create_skill(&store::database(&app)?, &name, &content, now_ms())
}

#[tauri::command]
fn update_skill(app: AppHandle, id: String, name: String, content: String) -> Result<db::Skill, String> {
    let (name, content) = checked_skill(&name, &content)?;
    db::update_skill(&store::database(&app)?, &id, &name, &content, now_ms())
}

#[tauri::command]
fn delete_skill(app: AppHandle, id: String) -> Result<(), String> {
    db::delete_skill(&store::database(&app)?, &id)
}

#[tauri::command]
fn set_default_skill(app: AppHandle, id: String) -> Result<(), String> {
    db::set_default_skill(&store::database(&app)?, &id)
}

/// The built-in skill text, for starting a new skill from a template.
#[tauri::command]
fn skill_template() -> String {
    store::DEFAULT_SKILL.to_string()
}

#[tauri::command]
fn list_channel_skills(app: AppHandle) -> Result<Vec<db::ChannelSkill>, String> {
    db::channel_skills(&store::database(&app)?)
}

#[tauri::command]
fn set_channel_skill(app: AppHandle, channel_id: String, skill_id: Option<String>) -> Result<(), String> {
    db::set_channel_skill(&store::database(&app)?, &channel_id, skill_id.as_deref())
}

/* Image API */

const IMAGE_API_SETTING: &str = "image_api";

fn image_api(app: &AppHandle) -> Result<Option<image::ImageApi>, String> {
    let raw = db::get_setting(&store::database(app)?, IMAGE_API_SETTING)?;
    Ok(raw
        .and_then(|raw| serde_json::from_str::<image::ImageApi>(&raw).ok())
        .filter(|api| !api.api_key.trim().is_empty()))
}

/// What Settings shows about the image API. The key itself never leaves
/// the backend; only its last characters are shown.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImageApiStatus {
    provider: String,
    model: String,
    configured: bool,
    key_hint: Option<String>,
}

#[tauri::command]
fn get_image_api(app: AppHandle) -> Result<ImageApiStatus, String> {
    Ok(match image_api(&app)? {
        Some(api) => {
            let key: Vec<char> = api.api_key.trim().chars().collect();
            let tail: String = key[key.len().saturating_sub(4)..].iter().collect();
            ImageApiStatus { provider: api.provider, model: api.model, configured: true, key_hint: Some(format!("••••{tail}")) }
        }
        None => ImageApiStatus {
            provider: "gemini".into(),
            model: image::default_model("gemini").into(),
            configured: false,
            key_hint: None,
        },
    })
}

/// Saves the image API. A blank key keeps the saved one.
#[tauri::command]
fn save_image_api(app: AppHandle, provider: String, model: String, api_key: Option<String>) -> Result<ImageApiStatus, String> {
    if !image::is_provider(&provider) {
        return Err("Pick Google Gemini or OpenAI.".into());
    }
    let api_key = match api_key.map(|key| key.trim().to_string()).filter(|key| !key.is_empty()) {
        Some(key) => key,
        None => image_api(&app)?
            .map(|api| api.api_key)
            .ok_or("Paste the API key first.")?,
    };
    let model = match model.trim() {
        "" => image::default_model(&provider).to_string(),
        model => model.to_string(),
    };
    let saved = serde_json::to_string(&image::ImageApi { provider, model, api_key }).map_err(|e| e.to_string())?;
    db::set_setting(&store::database(&app)?, IMAGE_API_SETTING, Some(&saved))?;
    get_image_api(app)
}

#[tauri::command]
fn remove_image_api(app: AppHandle) -> Result<ImageApiStatus, String> {
    db::set_setting(&store::database(&app)?, IMAGE_API_SETTING, None)?;
    get_image_api(app)
}

/// Draws one test image so the user knows the key and model work.
#[tauri::command]
async fn test_image_api(app: AppHandle) -> Result<String, String> {
    let api = image_api(&app)?.ok_or("Save an API key first.")?;
    let picture = image::generate(&api, "A small potted plant on a clean desk, soft daylight.").await?;
    Ok(image::to_data_url(&picture.bytes, picture.extension))
}

/* Cover images of ideas */

/// Points the idea at a new image file and removes the one it replaced.
fn attach_image(app: &AppHandle, id: &str, picture: Option<&image::Picture>) -> Result<Idea, String> {
    let file = picture.map(|picture| store::save_image(app, id, picture, now_ms())).transpose()?;
    let (idea, previous) = db::set_idea_image(&store::database(app)?, id, file.as_deref())?;
    if let Some(previous) = previous.filter(|previous| Some(previous) != file.as_ref()) {
        store::delete_image(app, &previous);
    }
    Ok(idea)
}

/// The idea's cover image as a data URL, if it has one.
#[tauri::command]
fn idea_image(app: AppHandle, id: String) -> Result<Option<String>, String> {
    match db::get_idea(&store::database(&app)?, &id)?.image_file {
        Some(file) => store::read_image(&app, &file),
        None => Ok(None),
    }
}

/// Saves an image the user picked as the idea's cover image.
#[tauri::command]
fn set_idea_image(app: AppHandle, id: String, data_url: String) -> Result<Idea, String> {
    let picture = image::from_data_url(&data_url)?;
    attach_image(&app, &id, Some(&picture))
}

#[tauri::command]
fn remove_idea_image(app: AppHandle, id: String) -> Result<Idea, String> {
    attach_image(&app, &id, None)
}

/// Draws the idea's cover image from `prompt` (its saved brief when blank).
#[tauri::command]
async fn generate_idea_image(app: AppHandle, id: String, prompt: Option<String>) -> Result<Idea, String> {
    let api = image_api(&app)?.ok_or("Add an image API key in Settings > Image API first.")?;
    let idea = db::get_idea(&store::database(&app)?, &id)?;
    let prompt = prompt
        .filter(|prompt| !prompt.trim().is_empty())
        .or(idea.image_prompt)
        .filter(|prompt| !prompt.trim().is_empty())
        .ok_or("Write a cover image brief first.")?;
    let picture = image::generate(&api, &prompt).await?;
    attach_image(&app, &id, Some(&picture))
}

#[tauri::command]
fn list_ideas(app: AppHandle, tab: String, page: Option<u32>) -> Result<db::IdeaPage, String> {
    db::list_ideas_page(&store::database(&app)?, tab == "sent", page.unwrap_or(1), db::IDEAS_PER_PAGE)
}

#[tauri::command]
fn add_idea(app: AppHandle, title: String) -> Result<Idea, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("Type an idea first.".into());
    }
    let idea = Idea::manual(title, now_ms());
    db::insert_ideas(&mut store::database(&app)?, std::slice::from_ref(&idea))?;
    Ok(idea)
}

#[tauri::command]
fn delete_idea(app: AppHandle, id: String) -> Result<(), String> {
    db::delete_idea(&store::database(&app)?, &id)
}

/// Saves the edits made on an idea's page.
#[tauri::command]
fn update_idea(app: AppHandle, mut idea: db::IdeaEdit) -> Result<Idea, String> {
    idea.title = idea.title.trim().to_string();
    if idea.title.is_empty() {
        return Err("The idea needs a title.".into());
    }
    db::update_idea(&store::database(&app)?, &idea)
}

/// Sends the saved idea to the main site's Ideas page. Sending again
/// updates the copy there instead of adding a second one.
#[tauri::command]
async fn send_idea(app: AppHandle, id: String) -> Result<Idea, String> {
    let config = signed_in(&app)?;
    upload_idea(&app, &config, &id).await
}

/// Puts the local idea on the main site (or updates it there) and returns
/// it with its remote id.
async fn upload_idea(app: &AppHandle, config: &Config, id: &str) -> Result<Idea, String> {
    let idea = db::get_idea(&store::database(app)?, id)?;
    let blank = |value: &str| if value.trim().is_empty() { None } else { Some(value.to_string()) };
    // The app owns the cover image: sending without one removes it there too.
    let image = match &idea.image_file {
        Some(file) => store::read_image(app, file)?,
        None => None,
    };

    let body = serde_json::json!({
        "image": image,
        "external_id": idea.id,
        "offer_id": idea.offer_id,
        "title": idea.title,
        "short_notes": blank(&idea.short_notes),
        "pain_addressed": blank(&idea.pain_addressed),
        "target_keyword": blank(&idea.target_keyword),
        "pattern": blank(&idea.pattern),
        "tag": blank(&idea.tag),
        "target_channel_id": idea.channel_id,
        "target_channel_name": idea.channel_name,
        "target_platform": idea.channel_platform,
        "topic_title": idea.topic_title,
        "topic_description": idea.topic_description,
        "suggested_draft": blank(&idea.suggested_draft),
        "image_prompt": idea.image_prompt,
        "provider": idea.provider,
        "planned_date": idea.scheduled_date,
        "planned_time": idea.scheduled_time,
    });
    let response = api::post(&config.session.base_url, &config.token, "/api/ideas", &body).await?;

    db::mark_sent(&store::database(app)?, id, response["idea"]["id"].as_i64(), now_ms())
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

fn remote_id_of(idea: &Idea) -> Result<i64, String> {
    idea.remote_id
        .ok_or_else(|| "The main site didn't return an id for this idea. Send it again.".to_string())
}

/// The channels the main site can publish to.
#[tauri::command]
async fn list_channels(app: AppHandle) -> Result<Value, String> {
    let config = signed_in(&app)?;
    api::get(&config.session.base_url, &config.token, "/api/channels").await
}

/// The main site's copy of a sent idea, with its saved posts.
#[tauri::command]
async fn get_remote_idea(app: AppHandle, remote_id: i64) -> Result<Value, String> {
    let config = signed_in(&app)?;
    api::get(&config.session.base_url, &config.token, &format!("/api/ideas/{remote_id}")).await
}

#[tauri::command]
fn list_patterns(app: AppHandle) -> Result<Vec<String>, String> {
    db::list_patterns(&store::database(&app)?)
}

/// The main site's "Send" on its Write post page: the idea is sent (or
/// updated) first, then its post joins the publish queue on every picked
/// channel at its scheduled time.
#[tauri::command]
async fn publish_idea(app: AppHandle, id: String) -> Result<Idea, String> {
    let config = signed_in(&app)?;
    let idea = upload_idea(&app, &config, &id).await?;
    if idea.publish_channels.is_empty() {
        return Err("Pick at least one channel to publish to.".into());
    }
    if idea.suggested_draft.trim().is_empty() {
        return Err("Write the post before sending it.".into());
    }

    let body = serde_json::json!({
        "action": "send",
        "text": idea.suggested_draft,
        "pattern": if idea.pattern.is_empty() { None } else { Some(&idea.pattern) },
        "channels": idea.publish_channels.iter().map(|channel| serde_json::json!({
            "id": channel.id,
            "scheduled_date": channel.scheduled_date,
            "scheduled_time": channel.scheduled_time,
        })).collect::<Vec<_>>(),
    });
    let path = format!("/api/ideas/{}/posts", remote_id_of(&idea)?);
    api::post(&config.session.base_url, &config.token, &path, &body).await?;

    db::mark_published(&store::database(&app)?, &id, now_ms())
}

/// One idea picked on the bulk publish page, when it goes out and on which channels.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BulkIdea {
    id: String,
    scheduled_date: String,
    scheduled_time: String,
    channels: Vec<String>,
}

/// The main site's "Send all" on its bulk publish page: every idea is sent
/// (or updated) first, then their posts are queued. Each idea picks its own
/// channels, so one bulk request goes out per distinct set of channels.
#[tauri::command]
async fn publish_ideas(app: AppHandle, ideas: Vec<BulkIdea>) -> Result<Vec<Idea>, String> {
    if ideas.is_empty() {
        return Err("Pick at least one idea to publish.".into());
    }
    if ideas.iter().any(|picked| picked.channels.is_empty()) {
        return Err("Pick at least one channel for every idea.".into());
    }
    let config = signed_in(&app)?;

    // Sorted channel set → the ideas going out on exactly those channels.
    let mut groups: Vec<(Vec<String>, Vec<serde_json::Value>)> = Vec::new();
    for picked in &ideas {
        let idea = upload_idea(&app, &config, &picked.id).await?;
        let entry = serde_json::json!({
            "id": remote_id_of(&idea)?,
            "text": if idea.suggested_draft.trim().is_empty() { None } else { Some(&idea.suggested_draft) },
            "scheduled_date": picked.scheduled_date,
            "scheduled_time": picked.scheduled_time,
        });
        let mut channels = picked.channels.clone();
        channels.sort();
        channels.dedup();
        match groups.iter_mut().find(|(set, _)| *set == channels) {
            Some((_, entries)) => entries.push(entry),
            None => groups.push((channels, vec![entry])),
        }
    }

    for (channels, entries) in &groups {
        let body = serde_json::json!({ "action": "send", "channels": channels, "ideas": entries });
        api::post(&config.session.base_url, &config.token, "/api/ideas/posts/bulk", &body).await?;
    }

    let conn = store::database(&app)?;
    let published_at = now_ms();
    ideas.iter().map(|picked| db::mark_published(&conn, &picked.id, published_at)).collect()
}

/// Stops the plan that is currently generating, if any.
#[tauri::command]
fn cancel_plan() {
    CANCEL_PLAN.store(true, Ordering::Relaxed);
}

/// Asks the chosen AI CLI for one idea per requested post, based on the
/// focus offer and the saved skill, then stores them. Progress is emitted
/// as `plan-progress` events while it runs.
#[tauri::command]
async fn generate_plan(app: AppHandle, request: plan::PlanRequest) -> Result<PlanOutcome, String> {
    if request.slots.is_empty() {
        return Err("Add at least one post to plan.".into());
    }
    if request.offer_ids.is_empty() {
        return Err("Pick at least one focus offer.".into());
    }
    CANCEL_PLAN.store(false, Ordering::Relaxed);
    let config = signed_in(&app)?;
    let base_url = &config.session.base_url;
    let agent = cli::provider_name(&request.provider);

    report(&app, "stage", format!("Loading the offers and company profile from {base_url}"));
    // Each fetched on its own: the offers list is paginated, so a pick may be on any page.
    let mut offers = Vec::with_capacity(request.offer_ids.len());
    for offer_id in &request.offer_ids {
        let offer = api::get(base_url, &config.token, &format!("/api/offers/{offer_id}"))
            .await
            .map_err(|error| {
                if error.contains(" 404 ") {
                    "One of the picked offers no longer exists on the server. Go back and pick again.".to_string()
                } else {
                    error
                }
            })?["offer"]
            .clone();
        offers.push(offer);
    }
    let company = api::get(base_url, &config.token, "/api/me").await?["company"].clone();
    let offer_names: Vec<Option<String>> = offers.iter().map(|offer| text(&offer["name"])).collect();
    let slot_offers = plan::slot_offers(request.slots.len(), offers.len());

    let image_api = if request.with_images && request.generate_images {
        Some(image_api(&app)?.ok_or("Add an image API key in Settings > Image API, or pick \"Write image prompts only\".")?)
    } else {
        None
    };

    let (existing_titles, allowed_tools, skills, slot_skills) = {
        let conn = store::database(&app)?;
        let (skills, slot_skills) = plan_skills(&conn, &request)?;
        (db::recent_idea_titles(&conn, 40)?, db::allowed_tools(&conn, &request.provider)?, skills, slot_skills)
    };
    let skill_names: Vec<String> = skills.iter().map(|skill| skill.name.clone()).collect();
    let topic = request.topic();
    let prompt = plan::build_prompt(
        &skills,
        &slot_skills,
        &company,
        &offers,
        &slot_offers,
        &request.slots,
        topic.as_ref(),
        request.with_images,
        &existing_titles,
    );
    let (topic_title, topic_description) = match &topic {
        Some(topic) => (non_empty(topic.title), non_empty(topic.description)),
        None => (None, None),
    };
    report(
        &app,
        "stage",
        format!(
            "Built the brief: offers {}, skills {}, {} slots ({} characters)",
            offer_names.iter().map(|name| format!("\"{}\"", name.as_deref().unwrap_or("?"))).collect::<Vec<_>>().join(", "),
            skill_names.iter().map(|name| format!("\"{name}\"")).collect::<Vec<_>>().join(", "),
            request.slots.len(),
            prompt.chars().count()
        ),
    );
    if !allowed_tools.is_empty() {
        report(&app, "stage", format!("{agent} may use: {}", allowed_tools.join(", ")));
    }

    report(&app, "stage", format!("Started {agent}. Waiting for it to write {} ideas…", request.slots.len()));
    let run = run_agent(&app, &request.provider, prompt, allowed_tools).await?;

    report(&app, "stage", format!("{agent} finished. Reading the ideas"));
    let drafted = match plan::parse_ideas(&run.text) {
        Ok(drafted) => drafted,
        Err(_) if !run.denied.is_empty() => {
            return Ok(PlanOutcome::PermissionNeeded { provider: request.provider, tools: run.denied });
        }
        Err(error) => return Err(error),
    };

    let created_at = now_ms();
    let ideas: Vec<Idea> = drafted
        .into_iter()
        .zip(request.slots.iter())
        .enumerate()
        .map(|(index, (idea, slot))| Idea {
            id: format!("idea-{created_at}-{index}"),
            short_notes: idea.short_notes,
            pain_addressed: idea.pain,
            target_keyword: idea.keyword,
            pattern: idea.pattern,
            tag: idea.tag,
            source: "Agent plan".into(),
            channel_id: Some(slot.channel_id.clone()),
            channel_name: Some(slot.channel_name.clone()),
            channel_platform: Some(slot.platform.clone()),
            topic_title: topic_title.clone(),
            topic_description: topic_description.clone(),
            offer_id: slot_offers.get(index).and_then(|&i| request.offer_ids.get(i)).copied(),
            offer_name: slot_offers.get(index).and_then(|&i| offer_names.get(i)).cloned().flatten(),
            suggested_draft: idea.draft,
            image_prompt: if request.with_images { idea.image_prompt } else { None },
            provider: Some(request.provider.clone()),
            skill_name: slot_skills.get(index).and_then(|&i| skill_names.get(i)).cloned(),
            ..Idea::manual(idea.title.trim().to_string(), created_at)
        })
        .collect();

    db::insert_ideas(&mut store::database(&app)?, &ideas)?;
    report(&app, "stage", format!("Saved {} ideas", ideas.len()));

    let ideas = match image_api {
        Some(api) => draw_cover_images(&app, &api, ideas).await,
        None => ideas,
    };
    Ok(PlanOutcome::Created { ideas })
}

/// Runs `prompt` through the provider's CLI in the app folder, reporting its
/// progress as `plan-progress` events. `cancel_plan` stops it.
async fn run_agent(app: &AppHandle, provider: &str, prompt: String, allowed_tools: Vec<String>) -> Result<cli::RunResult, String> {
    let workspace = store::workspace_dir(app)?;
    let provider = provider.to_string();
    let progress_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let options = cli::RunOptions {
            cwd: &workspace,
            timeout: PLAN_TIMEOUT,
            allowed_tools: &allowed_tools,
            cancel: Some(&CANCEL_PLAN),
        };
        cli::prompt(&provider, &prompt, &options, &mut |progress| report_progress(&progress_app, progress))
    })
    .await
    .map_err(|e| e.to_string())?
}

/* Offers drafted by the AI agent */

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum OfferOutcome {
    Created { offers: Vec<db::OfferDraft> },
    /// The agent was refused tools and returned no usable offers.
    PermissionNeeded { provider: String, tools: Vec<String> },
}

/// How many pages of the site's offers are read to tell the agent what exists.
const MAX_OFFER_PAGES: u32 = 5;

/// The names of the offers already on the main site.
async fn site_offer_names(config: &Config) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for page in 1..=MAX_OFFER_PAGES {
        let query = [("page", page.to_string()), ("per_page", "50".to_string()), ("sort", "drafted".to_string())];
        let result = api::get_query(&config.session.base_url, &config.token, "/api/offers", &query).await?;
        names.extend(result["offers"].as_array().into_iter().flatten().filter_map(|offer| text(&offer["name"])));
        if result["meta"]["last_page"].as_u64().unwrap_or(1) <= page as u64 {
            break;
        }
    }
    Ok(names)
}

/// The `id`s of the items in `list`.
fn ids_of(list: &[Value]) -> Vec<i64> {
    list.iter().filter_map(|item| item["id"].as_i64()).collect()
}

#[tauri::command]
fn list_offer_drafts(app: AppHandle) -> Result<Vec<db::OfferDraft>, String> {
    db::pending_offer_drafts(&store::database(&app)?)
}

#[tauri::command]
fn update_offer_draft(app: AppHandle, mut draft: db::OfferDraftEdit) -> Result<db::OfferDraft, String> {
    draft.name = draft.name.trim().to_string();
    draft.promise = draft.promise.trim().to_string();
    if draft.name.is_empty() {
        return Err("The offer needs a name.".into());
    }
    if draft.promise.is_empty() {
        return Err("The offer needs a promise.".into());
    }
    draft.deliverables = draft.deliverables.iter().map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect();
    db::update_offer_draft(&store::database(&app)?, &draft, now_ms())
}

#[tauri::command]
fn delete_offer_draft(app: AppHandle, id: String) -> Result<(), String> {
    db::delete_offer_draft(&store::database(&app)?, &id)
}

/// Asks the chosen AI CLI for new offers, based on the company overview,
/// the ICPs and the case studies on the main site, and keeps them as local
/// drafts until they are synced. Progress is emitted as `plan-progress`.
#[tauri::command]
async fn generate_offers(app: AppHandle, request: offers::OfferRequest) -> Result<OfferOutcome, String> {
    if request.count == 0 || request.count > offers::MAX_OFFERS {
        return Err(format!("Ask for 1 to {} offers.", offers::MAX_OFFERS));
    }
    CANCEL_PLAN.store(false, Ordering::Relaxed);
    let config = signed_in(&app)?;
    let base_url = &config.session.base_url;
    let agent = cli::provider_name(&request.provider);

    report(&app, "stage", format!("Loading the company overview, ICPs and case studies from {base_url}"));
    let company = api::get(base_url, &config.token, "/api/me").await?["company"].clone();
    if company.is_null() {
        return Err("The main site has no company profile yet. Finish onboarding there first.".into());
    }
    let icps = api::get(base_url, &config.token, "/api/app/icps").await?["icps"].as_array().cloned().unwrap_or_default();
    let proof = api::get(base_url, &config.token, "/api/app/proof").await?["proof"].as_array().cloned().unwrap_or_default();
    let mut existing = site_offer_names(&config).await?;

    let (allowed_tools, drafted_names) = {
        let conn = store::database(&app)?;
        (db::allowed_tools(&conn, &request.provider)?, db::offer_draft_names(&conn)?)
    };
    for name in drafted_names {
        if !existing.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
            existing.push(name);
        }
    }

    let prompt = offers::build_prompt(
        &company,
        &offers::brief_icps(&icps),
        &offers::brief_case_studies(&proof),
        &existing,
        request.count,
        request.focus(),
    );
    report(
        &app,
        "stage",
        format!(
            "Built the brief: {} ICPs, {} case studies, {} existing offers ({} characters)",
            icps.len(),
            proof.len(),
            existing.len(),
            prompt.chars().count()
        ),
    );
    if !allowed_tools.is_empty() {
        report(&app, "stage", format!("{agent} may use: {}", allowed_tools.join(", ")));
    }

    report(&app, "stage", format!("Started {agent}. Waiting for it to draft {} offers…", request.count));
    let run = run_agent(&app, &request.provider, prompt, allowed_tools).await?;

    report(&app, "stage", format!("{agent} finished. Reading the offers"));
    let drafted = match offers::parse_offers(&run.text) {
        Ok(drafted) => drafted,
        Err(_) if !run.denied.is_empty() => {
            return Ok(OfferOutcome::PermissionNeeded { provider: request.provider, tools: run.denied });
        }
        Err(error) => return Err(error),
    };

    // Only ids the site sent may be linked; the agent can invent others.
    let (icp_ids, proof_ids) = (ids_of(&icps), ids_of(&proof));
    let created_at = now_ms();
    let drafts: Vec<db::OfferDraft> = drafted
        .into_iter()
        .take(request.count as usize)
        .enumerate()
        .map(|(index, offer)| db::OfferDraft {
            id: format!("offer-{created_at}-{index}"),
            name: offer.name.trim().to_string(),
            promise: offer.promise.trim().to_string(),
            description: offer.description.trim().to_string(),
            price_range: offer.price_range.trim().to_string(),
            timeline: offer.timeline.trim().to_string(),
            deliverables: offer.deliverables.iter().map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect(),
            icp_ids: offer.icp_ids.into_iter().filter(|id| icp_ids.contains(id)).collect(),
            case_study_ids: offer.case_study_ids.into_iter().filter(|id| proof_ids.contains(id)).collect(),
            rationale: offer.rationale.trim().to_string(),
            provider: Some(request.provider.clone()),
            created_at,
            updated_at: created_at,
            synced_at: None,
            remote_id: None,
        })
        .collect();

    db::insert_offer_drafts(&mut store::database(&app)?, &drafts)?;
    report(&app, "stage", format!("Saved {} offers as drafts", drafts.len()));
    Ok(OfferOutcome::Created { offers: drafts })
}

/// How many offers go to the main site in one request (its limit is 50).
const OFFERS_PER_SYNC: usize = 25;

/// Puts every offer draft not synced yet on the main site. The app's id
/// travels with each one, so a sync that is cut short can safely run again.
/// Returns how many were synced.
#[tauri::command]
async fn sync_offer_drafts(app: AppHandle) -> Result<usize, String> {
    let config = signed_in(&app)?;
    let drafts = db::pending_offer_drafts(&store::database(&app)?)?;
    if drafts.is_empty() {
        return Err("There are no new offers to sync.".into());
    }
    let blank = |value: &str| (!value.trim().is_empty()).then(|| value.trim().to_string());

    let mut synced = 0;
    for chunk in drafts.chunks(OFFERS_PER_SYNC) {
        let body = serde_json::json!({
            "offers": chunk.iter().map(|draft| serde_json::json!({
                "external_id": draft.id,
                "name": draft.name,
                "promise": draft.promise,
                "description": blank(&draft.description),
                "price_range": blank(&draft.price_range),
                "timeline": blank(&draft.timeline),
                "deliverables": draft.deliverables,
                "icp_ids": draft.icp_ids,
                "case_study_ids": draft.case_study_ids,
            })).collect::<Vec<_>>(),
        });
        let response = api::post(&config.session.base_url, &config.token, "/api/offers/sync", &body)
            .await
            .map_err(|error| {
                if error.contains(" 404 ") || error.contains(" 405 ") {
                    "The main site can't take offers from the app yet. Update it to the latest version, then sync again.".to_string()
                } else {
                    error
                }
            })?;

        let conn = store::database(&app)?;
        let synced_at = now_ms();
        for item in response["offers"].as_array().into_iter().flatten() {
            if let Some(id) = item["external_id"].as_str() {
                db::mark_offer_synced(&conn, id, item["id"].as_i64(), synced_at)?;
                synced += 1;
            }
        }
    }
    Ok(synced)
}

/// The distinct skills a plan uses, and for each slot the index of its
/// skill: the one picked for the slot's channel, else the default skill.
fn plan_skills(conn: &rusqlite::Connection, request: &plan::PlanRequest) -> Result<(Vec<plan::PlanSkill>, Vec<usize>), String> {
    let default = db::default_skill(conn)?;
    let mut picked: Vec<db::Skill> = Vec::new();
    let mut slot_skills = Vec::with_capacity(request.slots.len());
    for slot in &request.slots {
        let skill = match request.channel_skills.get(&slot.channel_id) {
            // A skill deleted since the page loaded falls back to the default.
            Some(id) => db::get_skill(conn, id).unwrap_or_else(|_| default.clone()),
            None => default.clone(),
        };
        let index = match picked.iter().position(|known| known.id == skill.id) {
            Some(index) => index,
            None => {
                picked.push(skill);
                picked.len() - 1
            }
        };
        slot_skills.push(index);
    }
    let skills = picked.into_iter().map(|skill| plan::PlanSkill { name: skill.name, content: skill.content }).collect();
    Ok((skills, slot_skills))
}

/// Draws a cover image for every new idea that has a brief. A failed
/// image is reported and skipped; the idea keeps its brief.
async fn draw_cover_images(app: &AppHandle, api: &image::ImageApi, ideas: Vec<Idea>) -> Vec<Idea> {
    let total = ideas.iter().filter(|idea| idea.image_prompt.is_some()).count();
    let mut drawn = Vec::with_capacity(ideas.len());
    let mut done = 0;
    for idea in ideas {
        let Some(prompt) = idea.image_prompt.clone().filter(|prompt| !prompt.trim().is_empty()) else {
            drawn.push(idea);
            continue;
        };
        if CANCEL_PLAN.load(Ordering::Relaxed) {
            report(app, "denied", "Stopped before all images were drawn. Generate the rest from each idea's page.");
            drawn.push(idea);
            continue;
        }
        done += 1;
        report(app, "stage", format!("Drawing cover image {done} of {total}: \"{}\"", idea.title));
        let result = match image::generate(api, &prompt).await {
            Ok(picture) => attach_image(app, &idea.id, Some(&picture)),
            Err(error) => Err(error),
        };
        match result {
            Ok(with_image) => drawn.push(with_image),
            Err(error) => {
                report(app, "denied", format!("No image for \"{}\": {error}", idea.title));
                drawn.push(idea);
            }
        }
    }
    drawn
}

#[tauri::command]
async fn fetch_post_analytics(
    app: AppHandle,
    post_id: String,
    channel_id: String,
    platform_post_id: Option<String>,
    post_url: Option<String>,
) -> Result<analytics::PostAnalytics, String> {
    analytics::fetch_post_analytics(app, post_id, channel_id, platform_post_id, post_url).await
}

#[tauri::command]
fn get_post_analytics(app: AppHandle, post_id: String) -> Result<Option<analytics::PostAnalytics>, String> {
    let conn = store::database(&app)?;
    db::get_post_analytics(&conn, &post_id)
}

#[tauri::command]
fn set_post_platform_id(app: AppHandle, post_id: String, platform_post_id: String, post_url: Option<String>) -> Result<(), String> {
    let conn = store::database(&app)?;
    db::set_post_platform_id(&conn, &post_id, &platform_post_id, post_url.as_deref())
}

#[tauri::command]
fn get_channel_tokens(app: AppHandle) -> Result<std::collections::HashMap<String, String>, String> {
    let conn = store::database(&app)?;
    let mut tokens = std::collections::HashMap::new();
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings WHERE key LIKE 'channel_token:%'")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        if let Some(channel_id) = row.0.strip_prefix("channel_token:") {
            tokens.insert(channel_id.to_string(), row.1);
        }
    }
    Ok(tokens)
}

#[tauri::command]
fn save_channel_token(app: AppHandle, channel_id: String, token: String) -> Result<(), String> {
    let conn = store::database(&app)?;
    let val = if token.trim().is_empty() { None } else { Some(token.trim()) };
    db::set_setting(&conn, &format!("channel_token:{channel_id}"), val)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            if let Err(error) = store::import_legacy_ideas(app.handle()) {
                eprintln!("{error}");
            }
            if let Err(error) = store::seed_skills(app.handle(), now_ms()) {
                eprintln!("{error}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_session,
            login,
            logout,
            list_offers,
            site_page,
            offer_idea_counts,
            detect_clis,
            test_cli,
            list_permissions,
            grant_permissions,
            revoke_permission,
            list_skills,
            create_skill,
            update_skill,
            delete_skill,
            set_default_skill,
            skill_template,
            list_channel_skills,
            set_channel_skill,
            get_image_api,
            save_image_api,
            remove_image_api,
            test_image_api,
            idea_image,
            set_idea_image,
            remove_idea_image,
            generate_idea_image,
            list_ideas,
            add_idea,
            delete_idea,
            update_idea,
            send_idea,
            list_channels,
            get_remote_idea,
            list_patterns,
            publish_idea,
            publish_ideas,
            cancel_plan,
            generate_plan,
            list_offer_drafts,
            update_offer_draft,
            delete_offer_draft,
            generate_offers,
            sync_offer_drafts,
            fetch_post_analytics,
            get_post_analytics,
            set_post_platform_id,
            get_channel_tokens,
            save_channel_token
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
