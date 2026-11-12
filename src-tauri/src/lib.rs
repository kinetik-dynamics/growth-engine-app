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
async fn list_offers(app: AppHandle) -> Result<Value, String> {
    let config = signed_in(&app)?;
    let body = api::get(&config.session.base_url, &config.token, "/api/offers").await?;
    Ok(body["offers"].clone())
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
fn list_ideas(app: AppHandle) -> Result<Vec<Idea>, String> {
    db::list_ideas(&store::database(&app)?)
}

#[tauri::command]
fn add_idea(app: AppHandle, title: String) -> Result<Vec<Idea>, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("Type an idea first.".into());
    }
    let mut conn = store::database(&app)?;
    db::insert_ideas(&mut conn, &[Idea::manual(title, now_ms())])?;
    db::list_ideas(&conn)
}

#[tauri::command]
fn delete_idea(app: AppHandle, id: String) -> Result<Vec<Idea>, String> {
    let conn = store::database(&app)?;
    db::delete_idea(&conn, &id)?;
    db::list_ideas(&conn)
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
    let idea = db::get_idea(&store::database(&app)?, &id)?;

    let body = serde_json::json!({
        "external_id": idea.id,
        "offer_id": idea.offer_id,
        "title": idea.title,
        "short_notes": idea.short_notes,
        "pain_addressed": idea.pain_addressed,
        "target_keyword": idea.target_keyword,
        "pattern": idea.pattern,
        "tag": idea.tag,
        "target_channel_id": idea.channel_id,
        "suggested_draft": idea.suggested_draft,
        "image_prompt": idea.image_prompt,
    });
    let response = api::post(&config.session.base_url, &config.token, "/api/ideas", &body).await?;

    db::mark_sent(&store::database(&app)?, &id, response["idea"]["id"].as_i64(), now_ms())
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
    let offers = api::get(base_url, &config.token, "/api/offers").await?;
    let offer = offers["offers"]
        .as_array()
        .and_then(|offers| offers.iter().find(|offer| offer["id"].as_i64() == Some(request.offer_id)))
        .cloned()
        .ok_or("That offer no longer exists on the server. Go back and pick another.")?;
    let company = api::get(base_url, &config.token, "/api/me").await?["company"].clone();
    let offer_name = text(&offer["name"]);

    let skill = store::load_skill(&app)?;
    let (existing_titles, allowed_tools) = {
        let conn = store::database(&app)?;
        let titles: Vec<String> = db::list_ideas(&conn)?.into_iter().take(40).map(|idea| idea.title).collect();
        (titles, db::allowed_tools(&conn, &request.provider)?)
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
            cancel_plan,
            generate_plan
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
