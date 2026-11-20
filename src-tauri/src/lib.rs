mod api;
mod cli;
mod db;
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
    api::get(&config.session.base_url, &config.token, &format!("/api/app/{page}")).await
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

#[tauri::command]
fn get_skill(app: AppHandle) -> Result<String, String> {
    store::load_skill(&app)
}

#[tauri::command]
fn save_skill(app: AppHandle, content: String) -> Result<(), String> {
    if content.trim().is_empty() {
        return Err("The skill can't be empty. Use \"Reset to default\" to restore the original.".into());
    }
    store::save_skill(&app, &content)
}

#[tauri::command]
fn reset_skill(app: AppHandle) -> Result<String, String> {
    store::reset_skill(&app)
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

    let body = serde_json::json!({
        "external_id": idea.id,
        "offer_id": idea.offer_id,
        "title": idea.title,
        "short_notes": blank(&idea.short_notes),
        "pain_addressed": blank(&idea.pain_addressed),
        "target_keyword": blank(&idea.target_keyword),
        "pattern": blank(&idea.pattern),
        "tag": blank(&idea.tag),
        "target_channel_id": idea.channel_id,
        "suggested_draft": blank(&idea.suggested_draft),
        "image_prompt": idea.image_prompt,
        "provider": idea.provider,
        "planned_date": idea.scheduled_date,
        "planned_time": idea.scheduled_time,
    });
    let response = api::post(&config.session.base_url, &config.token, "/api/ideas", &body).await?;

    db::mark_sent(&store::database(app)?, id, response["idea"]["id"].as_i64(), now_ms())
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

/// One idea picked on the bulk publish page and when it goes out.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BulkIdea {
    id: String,
    scheduled_date: String,
    scheduled_time: String,
}

/// The main site's "Send all" on its bulk publish page: every idea is sent
/// (or updated) first, then all their posts are queued together.
#[tauri::command]
async fn publish_ideas(app: AppHandle, ideas: Vec<BulkIdea>, channels: Vec<String>) -> Result<Vec<Idea>, String> {
    if ideas.is_empty() {
        return Err("Pick at least one idea to publish.".into());
    }
    if channels.is_empty() {
        return Err("Pick at least one channel to publish to.".into());
    }
    let config = signed_in(&app)?;

    let mut entries = Vec::with_capacity(ideas.len());
    for picked in &ideas {
        let idea = upload_idea(&app, &config, &picked.id).await?;
        entries.push(serde_json::json!({
            "id": remote_id_of(&idea)?,
            "text": if idea.suggested_draft.trim().is_empty() { None } else { Some(&idea.suggested_draft) },
            "scheduled_date": picked.scheduled_date,
            "scheduled_time": picked.scheduled_time,
        }));
    }

    let body = serde_json::json!({ "action": "send", "channels": channels, "ideas": entries });
    api::post(&config.session.base_url, &config.token, "/api/ideas/posts/bulk", &body).await?;

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
    CANCEL_PLAN.store(false, Ordering::Relaxed);
    let config = signed_in(&app)?;
    let base_url = &config.session.base_url;
    let agent = cli::provider_name(&request.provider);

    report(&app, "stage", format!("Loading the offer and company profile from {base_url}"));
    // Fetched on its own: the offers list is paginated, so the pick may be on any page.
    let offer = api::get(base_url, &config.token, &format!("/api/offers/{}", request.offer_id))
        .await
        .map_err(|error| {
            if error.contains(" 404 ") {
                "That offer no longer exists on the server. Go back and pick another.".to_string()
            } else {
                error
            }
        })?["offer"]
        .clone();
    let company = api::get(base_url, &config.token, "/api/me").await?["company"].clone();
    let offer_name = text(&offer["name"]);

    let skill = store::load_skill(&app)?;
    let (existing_titles, allowed_tools) = {
        let conn = store::database(&app)?;
        (db::recent_idea_titles(&conn, 40)?, db::allowed_tools(&conn, &request.provider)?)
    };
    let prompt = plan::build_prompt(&skill, &company, &offer, &request.slots, request.with_images, &existing_titles);
    report(
        &app,
        "stage",
        format!(
            "Built the brief: offer \"{}\", your idea skill, {} slots ({} characters)",
            offer_name.as_deref().unwrap_or("?"),
            request.slots.len(),
            prompt.chars().count()
        ),
    );
    if !allowed_tools.is_empty() {
        report(&app, "stage", format!("{agent} may use: {}", allowed_tools.join(", ")));
    }

    report(&app, "stage", format!("Started {agent}. Waiting for it to write {} ideas…", request.slots.len()));
    let workspace = store::workspace_dir(&app)?;
    let provider = request.provider.clone();
    let progress_app = app.clone();
    let run = tauri::async_runtime::spawn_blocking(move || {
        let options = cli::RunOptions {
            cwd: &workspace,
            timeout: PLAN_TIMEOUT,
            allowed_tools: &allowed_tools,
            cancel: Some(&CANCEL_PLAN),
        };
        cli::prompt(&provider, &prompt, &options, &mut |progress| report_progress(&progress_app, progress))
    })
    .await
    .map_err(|e| e.to_string())??;

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
            offer_id: Some(request.offer_id),
            offer_name: offer_name.clone(),
            suggested_draft: idea.draft,
            image_prompt: if request.with_images { idea.image_prompt } else { None },
            provider: Some(request.provider.clone()),
            ..Idea::manual(idea.title.trim().to_string(), created_at)
        })
        .collect();

    db::insert_ideas(&mut store::database(&app)?, &ideas)?;
    report(&app, "stage", format!("Saved {} ideas", ideas.len()));
    Ok(PlanOutcome::Created { ideas })
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
            get_skill,
            save_skill,
            reset_skill,
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
            generate_plan
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
