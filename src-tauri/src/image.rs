//! Cover images: generating them with the image API the user set up in
//! Settings, and the data-URL form the UI and the main site use.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

/// The largest image the app accepts, matching the main site's limit.
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

const GENERATE_TIMEOUT: Duration = Duration::from_secs(180);

/// Which service draws the images, with its key. Saved in the local database.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageApi {
    /// "gemini" or "openai".
    pub provider: String,
    pub model: String,
    pub api_key: String,
}

pub fn default_model(provider: &str) -> &'static str {
    match provider {
        "openai" => "gpt-image-1",
        _ => "gemini-2.5-flash-image",
    }
}

pub fn is_provider(provider: &str) -> bool {
    matches!(provider, "gemini" | "openai")
}

/// An image's bytes and its file extension.
#[derive(Debug)]
pub struct Picture {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
}

fn extension_for(mime: &str) -> Option<&'static str> {
    match mime {
        "image/png" => Some("png"),
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        _ => None,
    }
}

fn mime_for(extension: &str) -> &'static str {
    match extension {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/png",
    }
}

/// Reads a "data:image/png;base64,…" URL picked in the UI.
pub fn from_data_url(data_url: &str) -> Result<Picture, String> {
    let rest = data_url.strip_prefix("data:").ok_or("That file isn't an image.")?;
    let (mime, data) = rest.split_once(";base64,").ok_or("That file isn't an image.")?;
    let extension = extension_for(mime).ok_or("Pick a PNG, JPG, WebP or GIF image.")?;
    let bytes = STANDARD.decode(data.trim()).map_err(|_| "The image couldn't be read.".to_string())?;
    check_size(&bytes)?;
    Ok(Picture { bytes, extension })
}

pub fn to_data_url(bytes: &[u8], extension: &str) -> String {
    format!("data:{};base64,{}", mime_for(extension), STANDARD.encode(bytes))
}

fn check_size(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        return Err("The image is empty.".into());
    }
    if bytes.len() > MAX_BYTES {
        return Err("The image is larger than 8 MB. Pick a smaller one.".into());
    }
    Ok(())
}

/// Asks the configured service for one cover image drawn from `prompt`.
pub async fn generate(api: &ImageApi, prompt: &str) -> Result<Picture, String> {
    let client = reqwest::Client::builder()
        .timeout(GENERATE_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let prompt = format!(
        "{}\n\nA landscape social media cover image. No text, words or logos in the image.",
        prompt.trim()
    );

    let picture = match api.provider.as_str() {
        "openai" => openai(&client, api, &prompt).await?,
        _ => gemini(&client, api, &prompt).await?,
    };
    check_size(&picture.bytes)?;
    Ok(picture)
}

async fn send(request: reqwest::RequestBuilder, service: &str) -> Result<Value, String> {
    let response = request
        .send()
        .await
        .map_err(|e| format!("Couldn't reach {service}. ({e})"))?;
    let status = response.status();
    let body = response.json::<Value>().await.unwrap_or_default();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(format!("{service} rejected the API key. Check it in Settings > Image API."));
    }
    if !status.is_success() {
        let message = body["error"]["message"].as_str().unwrap_or("no details");
        return Err(format!("{service} couldn't make the image ({status}): {message}"));
    }
    Ok(body)
}

async fn gemini(client: &reqwest::Client, api: &ImageApi, prompt: &str) -> Result<Picture, String> {
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", api.model);
    let body = json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": { "responseModalities": ["TEXT", "IMAGE"] },
    });
    let reply = send(client.post(url).header("x-goog-api-key", api.api_key.trim()).json(&body), "Google Gemini").await?;
    gemini_picture(&reply)
}

fn gemini_picture(reply: &Value) -> Result<Picture, String> {
    let parts = reply["candidates"][0]["content"]["parts"].as_array().cloned().unwrap_or_default();
    for part in &parts {
        let inline = if part["inlineData"].is_object() { &part["inlineData"] } else { &part["inline_data"] };
        if let Some(data) = inline["data"].as_str() {
            let mime = inline["mimeType"].as_str().or(inline["mime_type"].as_str()).unwrap_or("image/png");
            let bytes = STANDARD.decode(data).map_err(|_| "Google Gemini sent an unreadable image.".to_string())?;
            return Ok(Picture { bytes, extension: extension_for(mime).unwrap_or("png") });
        }
    }
    let said = parts.iter().filter_map(|part| part["text"].as_str()).collect::<Vec<_>>().join(" ");
    Err(if said.trim().is_empty() {
        "Google Gemini didn't return an image. Check that the model can create images.".into()
    } else {
        format!("Google Gemini didn't return an image. It said: {}", said.chars().take(300).collect::<String>())
    })
}

async fn openai(client: &reqwest::Client, api: &ImageApi, prompt: &str) -> Result<Picture, String> {
    let mut body = json!({ "model": api.model, "prompt": prompt, "n": 1 });
    if api.model.starts_with("dall-e") {
        body["response_format"] = json!("b64_json");
        body["size"] = json!(if api.model == "dall-e-3" { "1792x1024" } else { "1024x1024" });
    } else {
        body["size"] = json!("1536x1024");
    }
    let reply = send(
        client.post("https://api.openai.com/v1/images/generations").bearer_auth(api.api_key.trim()).json(&body),
        "OpenAI",
    )
    .await?;
    let data = reply["data"][0]["b64_json"].as_str().ok_or("OpenAI didn't return an image.")?;
    let bytes = STANDARD.decode(data).map_err(|_| "OpenAI sent an unreadable image.".to_string())?;
    Ok(Picture { bytes, extension: "png" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_urls_round_trip_and_bad_ones_are_refused() {
        let url = to_data_url(&[1, 2, 3], "jpg");
        assert!(url.starts_with("data:image/jpeg;base64,"));

        let picture = from_data_url(&url).unwrap();
        assert_eq!((picture.bytes, picture.extension), (vec![1, 2, 3], "jpg"));

        assert!(from_data_url("data:text/plain;base64,aGk=").unwrap_err().contains("PNG"));
        assert!(from_data_url("hello").is_err());
        assert!(from_data_url("data:image/png;base64,").unwrap_err().contains("empty"));
    }

    #[test]
    fn the_image_is_taken_from_a_gemini_reply_or_its_words_are_reported() {
        let reply = json!({ "candidates": [{ "content": { "parts": [
            { "text": "Here you go" },
            { "inlineData": { "mimeType": "image/png", "data": STANDARD.encode([9, 9]) } }
        ] } }] });
        let picture = gemini_picture(&reply).unwrap();
        assert_eq!((picture.bytes, picture.extension), (vec![9, 9], "png"));

        let refusal = json!({ "candidates": [{ "content": { "parts": [{ "text": "I can't draw that" }] } }] });
        assert!(gemini_picture(&refusal).unwrap_err().contains("I can't draw that"));
    }
}
