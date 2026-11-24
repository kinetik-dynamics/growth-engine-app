//! Post analytics: fetching metrics (impressions, views, likes, comments)
//! from official social media channel APIs (LinkedIn, Facebook, X) or the main site,
//! and persisting platform post IDs and cached metrics in the local database.

use crate::{api, db, store};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tauri::AppHandle;

const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricValue {
    pub value: Option<i64>,
    pub formatted: String,
    pub available: bool,
    pub unavailable_reason: Option<String>,
}

impl MetricValue {
    pub fn available(val: i64) -> Self {
        Self {
            value: Some(val),
            formatted: format_count(val),
            available: true,
            unavailable_reason: None,
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            value: None,
            formatted: "—".to_string(),
            available: false,
            unavailable_reason: Some(reason.into()),
        }
    }
}

pub fn format_count(val: i64) -> String {
    if val >= 1_000_000 {
        format!("{:.1}M", val as f64 / 1_000_000.0)
    } else if val >= 10_000 {
        format!("{:.1}k", val as f64 / 1_000.0)
    } else {
        let s = val.to_string();
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        for (i, c) in chars.into_iter().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                result.push(',');
            }
            result.push(c);
        }
        result
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostComment {
    pub id: String,
    pub author: String,
    pub author_avatar: Option<String>,
    pub text: String,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostAnalytics {
    pub post_id: String,
    pub platform_post_id: Option<String>,
    pub post_url: Option<String>,
    pub channel_id: String,
    pub platform: String,
    pub impressions: MetricValue,
    pub views: MetricValue,
    pub likes: MetricValue,
    pub comments: MetricValue,
    pub latest_comments: Vec<PostComment>,
    pub last_updated: u64,
    pub error: Option<String>,
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Helper to extract clean platform post ID or URN from a raw input string or URL.
pub fn parse_platform_post_id(platform: &str, input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    let url = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        Some(trimmed.to_string())
    } else {
        None
    };

    match platform {
        "LinkedIn" => {
            // Check for urn:li:activity:..., urn:li:share:..., urn:li:ugcPost:...
            for prefix in &["urn:li:activity:", "urn:li:share:", "urn:li:ugcPost:"] {
                if let Some(idx) = trimmed.find(prefix) {
                    let rest = &trimmed[idx..];
                    let end = rest.find(['/', '?', '&', '#', ' ', '"', '\'']).unwrap_or(rest.len());
                    return (rest[..end].to_string(), url);
                }
            }
            // Check for -activity-123456789- pattern in LinkedIn URLs
            if let Some(idx) = trimmed.find("-activity-") {
                let rest = &trimmed[idx + "-activity-".len()..];
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if !digits.is_empty() {
                    return (format!("urn:li:activity:{digits}"), url);
                }
            }
            // If all digits, default to urn:li:share:ID
            if trimmed.chars().all(|c| c.is_ascii_digit()) && !trimmed.is_empty() {
                return (format!("urn:li:share:{trimmed}"), url);
            }
            (trimmed.to_string(), url)
        }
        "Facebook" => {
            if let Some(ref u) = url {
                // e.g. facebook.com/permalink.php?story_fbid=123&id=456
                if u.contains("story_fbid=") {
                    if let Some(fbid) = u.split("story_fbid=").nth(1) {
                        let id = fbid.split('&').next().unwrap_or("").trim();
                        if !id.is_empty() {
                            return (id.to_string(), url);
                        }
                    }
                }
                // e.g. facebook.com/page/posts/123456789
                if u.contains("/posts/") {
                    if let Some(part) = u.split("/posts/").nth(1) {
                        let id = part.split(['/', '?', '&']).next().unwrap_or("").trim();
                        if !id.is_empty() {
                            return (id.to_string(), url);
                        }
                    }
                }
            }
            (trimmed.to_string(), url)
        }
        "X" => {
            if let Some(ref u) = url {
                // e.g. twitter.com/user/status/1234567890
                if u.contains("/status/") {
                    if let Some(part) = u.split("/status/").nth(1) {
                        let id: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
                        if !id.is_empty() {
                            return (id, url);
                        }
                    }
                }
            }
            (trimmed.to_string(), url)
        }
        _ => (trimmed.to_string(), url),
    }
}

pub fn platform_of(channel_id: &str) -> &'static str {
    if channel_id.starts_with("linkedin") {
        "LinkedIn"
    } else if channel_id.starts_with("facebook") {
        "Facebook"
    } else if channel_id.starts_with("x_") || channel_id == "x" {
        "X"
    } else if channel_id.starts_with("website") {
        "Website"
    } else {
        "Social"
    }
}

/// Resolves the access token for the given channel by checking:
/// 1. Local SQLite settings: `channel_token:{channel_id}` or `token:{platform}`
/// 2. Remote Growth Engine server: `/api/channels`
/// 3. Environment variable fallback (e.g. LINKEDIN_ACCESS_TOKEN, FACEBOOK_PAGE_ACCESS_TOKEN)
pub async fn resolve_channel_token(app: &AppHandle, channel_id: &str) -> Option<String> {
    // 1. SQLite settings
    if let Ok(conn) = store::database(app) {
        if let Ok(Some(tok)) = db::get_setting(&conn, &format!("channel_token:{channel_id}")) {
            if !tok.trim().is_empty() {
                return Some(tok.trim().to_string());
            }
        }
        let plat = platform_of(channel_id);
        if let Ok(Some(tok)) = db::get_setting(&conn, &format!("token:{plat}")) {
            if !tok.trim().is_empty() {
                return Some(tok.trim().to_string());
            }
        }
    }

    // 2. Growth Engine server /api/channels
    if let Ok(Some(config)) = store::load_config(app) {
        if let Ok(val) = api::get(&config.session.base_url, &config.token, "/api/channels").await {
            let list = val["channels"].as_array().or_else(|| val.as_array());
            if let Some(channels) = list {
                for ch in channels {
                    let id_match = ch["id"].as_str() == Some(channel_id);
                    let plat_match = ch["platform"].as_str() == Some(platform_of(channel_id));
                    if id_match || plat_match {
                        if let Some(tok) = ch["accessToken"]
                            .as_str()
                            .or_else(|| ch["access_token"].as_str())
                            .or_else(|| ch["token"].as_str())
                            .or_else(|| ch["credentials"]["accessToken"].as_str())
                            .or_else(|| ch["credentials"]["access_token"].as_str())
                        {
                            if !tok.trim().is_empty() {
                                return Some(tok.trim().to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Environment variables
    let env_name = match platform_of(channel_id) {
        "LinkedIn" => "LINKEDIN_ACCESS_TOKEN",
        "Facebook" => "FACEBOOK_PAGE_ACCESS_TOKEN",
        "X" => "X_BEARER_TOKEN",
        _ => "",
    };
    if !env_name.is_empty() {
        if let Ok(val) = std::env::var(env_name) {
            if !val.trim().is_empty() {
                return Some(val.trim().to_string());
            }
        }
    }

    None
}

/// Official LinkedIn API fetcher:
/// - Social Actions API: GET /rest/socialActions/{shareUrn} -> Likes and Comments
/// - Social Actions Comments: GET /rest/socialActions/{shareUrn}/comments -> Latest comments
/// - Organizational Entity Share Statistics (Company page): GET /rest/organizationalEntityShareStatistics
/// - Member Creator Post Analytics (Personal profile): GET /rest/memberCreatorPostAnalytics
async fn fetch_linkedin(
    client: &reqwest::Client,
    token: &str,
    target_urn: &str,
    is_company: bool,
    org_urn: Option<&str>,
) -> Result<(MetricValue, MetricValue, MetricValue, MetricValue, Vec<PostComment>), String> {
    let encoded_urn = target_urn.replace(':', "%3A").replace('/', "%2F");

    // 1. Social actions (likes & comments count)
    let social_url = format!("https://api.linkedin.com/rest/socialActions/{encoded_urn}");
    let social_res = client
        .get(&social_url)
        .bearer_auth(token)
        .header("LinkedIn-Version", "202401")
        .header("X-Restli-Protocol-Version", "2.0.0")
        .header("Accept", "application/json")
        .send()
        .await;

    let mut likes = MetricValue::unavailable("Likes metric unavailable");
    let mut comments = MetricValue::unavailable("Comments metric unavailable");

    if let Ok(res) = social_res {
        if res.status().is_success() {
            if let Ok(body) = res.json::<Value>().await {
                if let Some(l) = body["likesSummary"]["totalLikes"]
                    .as_i64()
                    .or_else(|| body["likesSummary"]["aggregatedTotalLikes"].as_i64())
                {
                    likes = MetricValue::available(l);
                }
                if let Some(c) = body["commentsSummary"]["totalComments"]
                    .as_i64()
                    .or_else(|| body["commentsSummary"]["aggregatedTotalComments"].as_i64())
                {
                    comments = MetricValue::available(c);
                }
            }
        } else if res.status().as_u16() == 403 || res.status().as_u16() == 401 {
            likes = MetricValue::unavailable("Token expired or missing r_liteprofile/w_member_social scope");
            comments = MetricValue::unavailable("Token expired or missing r_liteprofile/w_member_social scope");
        }
    }

    // 2. Latest comments (up to 5)
    let mut latest_comments = Vec::new();
    let comments_url = format!("https://api.linkedin.com/rest/socialActions/{encoded_urn}/comments?count=5");
    if let Ok(res) = client
        .get(&comments_url)
        .bearer_auth(token)
        .header("LinkedIn-Version", "202401")
        .header("X-Restli-Protocol-Version", "2.0.0")
        .header("Accept", "application/json")
        .send()
        .await
    {
        if res.status().is_success() {
            if let Ok(body) = res.json::<Value>().await {
                if let Some(elements) = body["elements"].as_array() {
                    for elem in elements {
                        let id = elem["$URN"].as_str().unwrap_or("").to_string();
                        let author = elem["created"]["actor"].as_str().unwrap_or("LinkedIn Member").to_string();
                        let text = elem["message"]["text"].as_str().unwrap_or("").to_string();
                        let time = elem["created"]["time"].as_i64().map(|ms| ms.to_string());
                        if !text.is_empty() {
                            latest_comments.push(PostComment {
                                id: if id.is_empty() { format!("comment-{}", latest_comments.len()) } else { id },
                                author,
                                author_avatar: None,
                                text,
                                created_at: time,
                            });
                        }
                    }
                }
            }
        }
    }

    // 3. Impressions and Views
    let (impressions, views) = if is_company {
        let org = org_urn.unwrap_or("urn:li:organization:self");
        let stats_url = format!(
            "https://api.linkedin.com/rest/organizationalEntityShareStatistics?q=organizationalEntity&organizationalEntity={org}&shares=List({encoded_urn})"
        );
        let stats_res = client
            .get(&stats_url)
            .bearer_auth(token)
            .header("LinkedIn-Version", "202401")
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("Accept", "application/json")
            .send()
            .await;

        let mut imp = MetricValue::unavailable("Requires r_organization_social scope or Organization Administrator");
        let mut viw = MetricValue::unavailable("Requires r_organization_social scope or Organization Administrator");

        if let Ok(res) = stats_res {
            if res.status().is_success() {
                if let Ok(body) = res.json::<Value>().await {
                    if let Some(first) = body["elements"].as_array().and_then(|arr| arr.first()) {
                        let stats = &first["totalShareStatistics"];
                        if let Some(count) = stats["impressionCount"].as_i64() {
                            imp = MetricValue::available(count);
                        }
                        if let Some(unique) = stats["uniqueImpressionsCount"].as_i64() {
                            viw = MetricValue::available(unique);
                        }
                    }
                }
            }
        }
        (imp, viw)
    } else {
        // Personal profile: try memberCreatorPostAnalytics
        let creator_url = format!("https://api.linkedin.com/rest/memberCreatorPostAnalytics?q=entity&entity={encoded_urn}");
        let creator_res = client
            .get(&creator_url)
            .bearer_auth(token)
            .header("LinkedIn-Version", "202401")
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("Accept", "application/json")
            .send()
            .await;

        let mut imp = MetricValue::unavailable("Personal profiles require LinkedIn Member Creator Analytics approval");
        let mut viw = MetricValue::unavailable("Personal profiles require LinkedIn Member Creator Analytics approval");

        if let Ok(res) = creator_res {
            if res.status().is_success() {
                if let Ok(body) = res.json::<Value>().await {
                    if let Some(first) = body["elements"].as_array().and_then(|arr| arr.first()) {
                        if let Some(count) = first["impressionCount"].as_i64() {
                            imp = MetricValue::available(count);
                        }
                        if let Some(unique) = first["uniqueImpressionCount"].as_i64() {
                            viw = MetricValue::available(unique);
                        }
                    }
                }
            }
        }
        (imp, viw)
    };

    Ok((impressions, views, likes, comments, latest_comments))
}

/// Official Facebook Graph API fetcher:
/// - Likes and Comments: GET https://graph.facebook.com/v21.0/{post_id}?fields=reactions.summary(total_count),comments.summary(total_count)
/// - Post Insights: GET https://graph.facebook.com/v21.0/{post_id}/insights?metric=post_impressions,post_impressions_unique,post_video_views
async fn fetch_facebook(
    client: &reqwest::Client,
    token: &str,
    post_id: &str,
) -> Result<(MetricValue, MetricValue, MetricValue, MetricValue, Vec<PostComment>), String> {
    // 1. Reactions & comments
    let base_url = format!(
        "https://graph.facebook.com/v21.0/{post_id}?fields=reactions.summary(total_count),comments.summary(total_count)"
    );
    let post_res = client
        .get(&base_url)
        .bearer_auth(token)
        .header("Accept", "application/json")
        .send()
        .await;

    let mut likes = MetricValue::unavailable("Likes unavailable: check token permissions");
    let mut comments = MetricValue::unavailable("Comments unavailable: check token permissions");
    let mut latest_comments = Vec::new();

    if let Ok(res) = post_res {
        if res.status().is_success() {
            if let Ok(body) = res.json::<Value>().await {
                if let Some(l) = body["reactions"]["summary"]["total_count"].as_i64() {
                    likes = MetricValue::available(l);
                }
                if let Some(c) = body["comments"]["summary"]["total_count"].as_i64() {
                    comments = MetricValue::available(c);
                }
                if let Some(arr) = body["comments"]["data"].as_array() {
                    for item in arr.iter().take(5) {
                        let id = item["id"].as_str().unwrap_or("").to_string();
                        let author = item["from"]["name"].as_str().unwrap_or("Facebook User").to_string();
                        let text = item["message"].as_str().unwrap_or("").to_string();
                        let created_at = item["created_time"].as_str().map(|s| s.to_string());
                        if !text.is_empty() {
                            latest_comments.push(PostComment { id, author, author_avatar: None, text, created_at });
                        }
                    }
                }
            }
        } else if res.status().as_u16() == 403 || res.status().as_u16() == 401 {
            likes = MetricValue::unavailable("Token lacks pages_read_engagement or pages_show_list scope");
            comments = MetricValue::unavailable("Token lacks pages_read_engagement or pages_show_list scope");
        }
    }

    // 2. Insights (post_impressions, post_impressions_unique)
    let insights_url = format!(
        "https://graph.facebook.com/v21.0/{post_id}/insights?metric=post_impressions,post_impressions_unique,post_video_views"
    );
    let insights_res = client
        .get(&insights_url)
        .bearer_auth(token)
        .header("Accept", "application/json")
        .send()
        .await;

    let mut impressions = MetricValue::unavailable("Requires page read_insights scope");
    let mut views = MetricValue::unavailable("Requires page read_insights scope");

    if let Ok(res) = insights_res {
        if res.status().is_success() {
            if let Ok(body) = res.json::<Value>().await {
                if let Some(data) = body["data"].as_array() {
                    for metric in data {
                        let name = metric["name"].as_str().unwrap_or("");
                        let val = metric["values"]
                            .as_array()
                            .and_then(|v| v.first())
                            .and_then(|v| v["value"].as_i64());
                        if let Some(val) = val {
                            if name == "post_impressions" {
                                impressions = MetricValue::available(val);
                            } else if name == "post_impressions_unique" {
                                views = MetricValue::available(val);
                            } else if name == "post_video_views" && !views.available {
                                views = MetricValue::available(val);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok((impressions, views, likes, comments, latest_comments))
}

/// Official X API v2 fetcher:
/// - GET https://api.twitter.com/2/tweets/{tweet_id}?tweet.fields=public_metrics,non_public_metrics
async fn fetch_x(
    client: &reqwest::Client,
    token: &str,
    tweet_id: &str,
) -> Result<(MetricValue, MetricValue, MetricValue, MetricValue, Vec<PostComment>), String> {
    let url = format!("https://api.twitter.com/2/tweets/{tweet_id}?tweet.fields=public_metrics,non_public_metrics");
    let res = client
        .get(&url)
        .bearer_auth(token)
        .header("Accept", "application/json")
        .send()
        .await;

    let mut impressions = MetricValue::unavailable("X impressions require tweet.read and analytics scope");
    let mut views = MetricValue::unavailable("X views require tweet.read and analytics scope");
    let mut likes = MetricValue::unavailable("X likes require tweet.read scope");
    let mut comments = MetricValue::unavailable("X comments require tweet.read scope");

    if let Ok(res) = res {
        if res.status().is_success() {
            if let Ok(body) = res.json::<Value>().await {
                let metrics = &body["data"]["public_metrics"];
                if let Some(imp) = metrics["impression_count"].as_i64() {
                    impressions = MetricValue::available(imp);
                    views = MetricValue::available(imp);
                }
                if let Some(l) = metrics["like_count"].as_i64() {
                    likes = MetricValue::available(l);
                }
                if let Some(r) = metrics["reply_count"].as_i64() {
                    comments = MetricValue::available(r);
                }
            }
        }
    }

    Ok((impressions, views, likes, comments, Vec::new()))
}

/// Fallback to Growth Engine web app API: GET /api/posts/{post_id}/analytics
async fn fetch_main_site_analytics(
    app: &AppHandle,
    post_id: &str,
) -> Option<(MetricValue, MetricValue, MetricValue, MetricValue, Vec<PostComment>)> {
    let config = store::load_config(app).ok()??;
    let path = format!("/api/posts/{post_id}/analytics");
    let res = api::get(&config.session.base_url, &config.token, &path).await.ok()?;

    let data = if res["analytics"].is_object() { &res["analytics"] } else { &res };
    let impressions = data["impressions"]
        .as_i64()
        .map(MetricValue::available)
        .unwrap_or_else(|| MetricValue::unavailable("Impressions unavailable on server"));
    let views = data["views"]
        .as_i64()
        .map(MetricValue::available)
        .unwrap_or_else(|| MetricValue::unavailable("Views unavailable on server"));
    let likes = data["likes"]
        .as_i64()
        .map(MetricValue::available)
        .unwrap_or_else(|| MetricValue::unavailable("Likes unavailable on server"));
    let comments = data["commentsCount"]
        .as_i64()
        .or_else(|| data["comments_count"].as_i64())
        .or_else(|| data["comments"].as_i64())
        .map(MetricValue::available)
        .unwrap_or_else(|| MetricValue::unavailable("Comments count unavailable on server"));

    let mut latest_comments = Vec::new();
    if let Some(arr) = data["latestComments"].as_array().or_else(|| data["comments_list"].as_array()) {
        for c in arr {
            latest_comments.push(PostComment {
                id: c["id"].as_str().unwrap_or("").to_string(),
                author: c["author"].as_str().unwrap_or("User").to_string(),
                author_avatar: c["authorAvatar"].as_str().map(|s| s.to_string()),
                text: c["text"].as_str().unwrap_or("").to_string(),
                created_at: c["createdAt"].as_str().map(|s| s.to_string()),
            });
        }
    }

    Some((impressions, views, likes, comments, latest_comments))
}

pub async fn fetch_post_analytics(
    app: AppHandle,
    post_id: String,
    channel_id: String,
    platform_post_id: Option<String>,
    post_url: Option<String>,
) -> Result<PostAnalytics, String> {
    let conn = store::database(&app)?;
    let platform = platform_of(&channel_id).to_string();

    // 1. Resolve platform post ID
    let mut resolved_platform_id = platform_post_id.filter(|s| !s.trim().is_empty());
    let mut resolved_url = post_url.filter(|s| !s.trim().is_empty());

    if resolved_platform_id.is_none() {
        if let Ok(Some((stored_id, stored_url))) = db::get_post_platform_id(&conn, &post_id) {
            resolved_platform_id = Some(stored_id);
            if resolved_url.is_none() {
                resolved_url = stored_url;
            }
        }
    }

    // 2. Website channel has no social analytics
    if platform == "Website" {
        let analytics = PostAnalytics {
            post_id: post_id.clone(),
            platform_post_id: resolved_platform_id,
            post_url: resolved_url,
            channel_id: channel_id.clone(),
            platform: platform.clone(),
            impressions: MetricValue::unavailable("Analytics not supported for Website posts"),
            views: MetricValue::unavailable("Analytics not supported for Website posts"),
            likes: MetricValue::unavailable("Analytics not supported for Website posts"),
            comments: MetricValue::unavailable("Analytics not supported for Website posts"),
            latest_comments: Vec::new(),
            last_updated: crate::now_ms(),
            error: None,
        };
        let _ = db::save_post_analytics(&conn, &analytics);
        return Ok(analytics);
    }

    // 3. Try main site analytics first
    if let Some((imp, viw, lik, com, latest)) = fetch_main_site_analytics(&app, &post_id).await {
        let analytics = PostAnalytics {
            post_id: post_id.clone(),
            platform_post_id: resolved_platform_id,
            post_url: resolved_url,
            channel_id: channel_id.clone(),
            platform: platform.clone(),
            impressions: imp,
            views: viw,
            likes: lik,
            comments: com,
            latest_comments: latest,
            last_updated: crate::now_ms(),
            error: None,
        };
        let _ = db::save_post_analytics(&conn, &analytics);
        return Ok(analytics);
    }

    // 4. If no platform post ID is available, return informative placeholder
    let Some(raw_id) = resolved_platform_id.clone() else {
        let analytics = PostAnalytics {
            post_id: post_id.clone(),
            platform_post_id: None,
            post_url: resolved_url,
            channel_id: channel_id.clone(),
            platform: platform.clone(),
            impressions: MetricValue::unavailable("No platform post ID found. Click to enter the post ID or URL."),
            views: MetricValue::unavailable("No platform post ID found. Click to enter the post ID or URL."),
            likes: MetricValue::unavailable("No platform post ID found. Click to enter the post ID or URL."),
            comments: MetricValue::unavailable("No platform post ID found. Click to enter the post ID or URL."),
            latest_comments: Vec::new(),
            last_updated: crate::now_ms(),
            error: Some("No platform post ID found for this post. Enter the post ID or URL to fetch metrics.".into()),
        };
        let _ = db::save_post_analytics(&conn, &analytics);
        return Ok(analytics);
    };

    let (clean_id, detected_url) = parse_platform_post_id(&platform, &raw_id);
    if resolved_url.is_none() {
        resolved_url = detected_url;
    }

    // 5. Resolve channel access token
    let token = resolve_channel_token(&app, &channel_id).await;
    let Some(token) = token else {
        let analytics = PostAnalytics {
            post_id: post_id.clone(),
            platform_post_id: Some(clean_id),
            post_url: resolved_url,
            channel_id: channel_id.clone(),
            platform: platform.clone(),
            impressions: MetricValue::unavailable(format!("Connect {platform} account in Settings to fetch impressions")),
            views: MetricValue::unavailable(format!("Connect {platform} account in Settings to fetch views")),
            likes: MetricValue::unavailable(format!("Connect {platform} account in Settings to fetch likes")),
            comments: MetricValue::unavailable(format!("Connect {platform} account in Settings to fetch comments")),
            latest_comments: Vec::new(),
            last_updated: crate::now_ms(),
            error: Some(format!("No {platform} access token found. Connect the channel or set its token in Settings.")),
        };
        let _ = db::save_post_analytics(&conn, &analytics);
        return Ok(analytics);
    };

    // 6. Fetch from official platform API
    let client = http_client();
    let result = match platform.as_str() {
        "LinkedIn" => {
            let is_company = channel_id.contains("company") || channel_id.contains("org");
            fetch_linkedin(&client, &token, &clean_id, is_company, None).await
        }
        "Facebook" => fetch_facebook(&client, &token, &clean_id).await,
        "X" => fetch_x(&client, &token, &clean_id).await,
        _ => Err(format!("Unsupported platform {platform}")),
    };

    match result {
        Ok((impressions, views, likes, comments, latest_comments)) => {
            let analytics = PostAnalytics {
                post_id,
                platform_post_id: Some(clean_id),
                post_url: resolved_url,
                channel_id,
                platform,
                impressions,
                views,
                likes,
                comments,
                latest_comments,
                last_updated: crate::now_ms(),
                error: None,
            };
            let _ = db::save_post_analytics(&conn, &analytics);
            Ok(analytics)
        }
        Err(err) => {
            let analytics = PostAnalytics {
                post_id,
                platform_post_id: Some(clean_id),
                post_url: resolved_url,
                channel_id,
                platform,
                impressions: MetricValue::unavailable("Fetch failed"),
                views: MetricValue::unavailable("Fetch failed"),
                likes: MetricValue::unavailable("Fetch failed"),
                comments: MetricValue::unavailable("Fetch failed"),
                latest_comments: Vec::new(),
                last_updated: crate::now_ms(),
                error: Some(err),
            };
            let _ = db::save_post_analytics(&conn, &analytics);
            Ok(analytics)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_count_handles_various_ranges() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1000), "1,000");
        assert_eq!(format_count(9999), "9,999");
        assert_eq!(format_count(10000), "10.0k");
        assert_eq!(format_count(54200), "54.2k");
        assert_eq!(format_count(1500000), "1.5M");
    }

    #[test]
    fn parse_platform_post_id_extracts_correct_identifiers() {
        // LinkedIn URLs and URNs
        let (id, url) = parse_platform_post_id("LinkedIn", "urn:li:share:7123456789012345678");
        assert_eq!(id, "urn:li:share:7123456789012345678");
        assert_eq!(url, None);

        let (id, url) = parse_platform_post_id(
            "LinkedIn",
            "https://www.linkedin.com/feed/update/urn:li:activity:7123456789012345678/?actorCompanyId=123",
        );
        assert_eq!(id, "urn:li:activity:7123456789012345678");
        assert!(url.is_some());

        let (id, _) = parse_platform_post_id("LinkedIn", "7123456789012345678");
        assert_eq!(id, "urn:li:share:7123456789012345678");

        // Facebook URLs and IDs
        let (id, _) = parse_platform_post_id("Facebook", "https://www.facebook.com/permalink.php?story_fbid=9876543210&id=10001234");
        assert_eq!(id, "9876543210");

        let (id, _) = parse_platform_post_id("Facebook", "10001234_9876543210");
        assert_eq!(id, "10001234_9876543210");

        // X URLs and IDs
        let (id, _) = parse_platform_post_id("X", "https://x.com/user/status/1758291029384719283");
        assert_eq!(id, "1758291029384719283");
    }

    #[test]
    fn metric_value_constructors() {
        let avail = MetricValue::available(42);
        assert!(avail.available);
        assert_eq!(avail.value, Some(42));
        assert_eq!(avail.formatted, "42");
        assert_eq!(avail.unavailable_reason, None);

        let unavail = MetricValue::unavailable("Missing scope");
        assert!(!unavail.available);
        assert_eq!(unavail.value, None);
        assert_eq!(unavail.formatted, "—");
        assert_eq!(unavail.unavailable_reason.as_deref(), Some("Missing scope"));
    }
}
