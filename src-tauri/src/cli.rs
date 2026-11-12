//! Detects AI agent CLIs installed on this machine and runs one-shot prompts
//! through them. The app has no API keys of its own: whichever CLI the user
//! has already signed in to does the writing.
//!
//! Claude Code and Antigravity are run in their streaming JSON modes so the
//! UI can show what the agent is doing and which tool permissions it was
//! denied (headless mode cannot ask the user, so requests are auto-denied).

use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    pub bin: &'static str,
}

pub const PROVIDERS: [Provider; 4] = [
    Provider { id: "claude", name: "Claude Code", bin: "claude" },
    Provider { id: "antigravity", name: "Antigravity", bin: "agy" },
    Provider { id: "gemini", name: "Gemini CLI", bin: "gemini" },
    Provider { id: "codex", name: "Codex CLI", bin: "codex" },
];

/// Windows caps a whole command line at 32,767 characters; providers that
/// only accept the prompt as an argument must stay under it.
const MAX_ARG_PROMPT_CHARS: usize = 30_000;

pub const STOPPED: &str = "Generation stopped.";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub id: String,
    pub name: String,
    pub command: String,
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

/// Something worth showing the user while the agent works.
#[derive(Debug, PartialEq)]
pub enum Progress {
    /// A piece of the agent's answer as it is written.
    Text(String),
    /// The agent started using a tool.
    Tool(String),
    /// The agent asked for a tool permission and was refused.
    Denied { tool: String, message: String },
    /// A line the CLI printed that is not part of the answer.
    Log(String),
}

pub struct RunOptions<'a> {
    pub cwd: &'a Path,
    pub timeout: Duration,
    /// Tools the user has allowed this agent to use without asking.
    pub allowed_tools: &'a [String],
    pub cancel: Option<&'a AtomicBool>,
}

#[derive(Debug)]
pub struct RunResult {
    pub text: String,
    /// Tools the agent was refused during this run.
    pub denied: Vec<String>,
}

/// What has been gathered from the CLI's output stream so far.
#[derive(Default)]
struct StreamState {
    streamed: String,
    result: Option<String>,
    denied: Vec<String>,
    failed: Option<String>,
}

impl StreamState {
    fn deny(&mut self, tool: &str) {
        if !tool.is_empty() && !self.denied.iter().any(|denied| denied == tool) {
            self.denied.push(tool.to_string());
        }
    }

    fn text(&self) -> String {
        match &self.result {
            Some(result) if !result.trim().is_empty() => result.trim().to_string(),
            _ => self.streamed.trim().to_string(),
        }
    }
}

pub fn provider_name(id: &str) -> &'static str {
    PROVIDERS.iter().find(|provider| provider.id == id).map_or("The AI agent", |provider| provider.name)
}

/// Looks for `bin` in the given directories, trying the executable
/// extensions Windows shims use (`.exe`, then npm's `.cmd`/`.bat`).
pub fn find_in_dirs(bin: &str, dirs: impl Iterator<Item = PathBuf>) -> Option<PathBuf> {
    let extensions: &[&str] = if cfg!(windows) { &["exe", "cmd", "bat"] } else { &[""] };
    for dir in dirs {
        for extension in extensions {
            let candidate = if extension.is_empty() {
                dir.join(bin)
            } else {
                dir.join(format!("{bin}.{extension}"))
            };
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn find_on_path(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    find_in_dirs(bin, std::env::split_paths(&path))
}

/// Stops the CLI and anything it started (npm shims run node as a child).
fn kill_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
    let _ = child.kill();
    let _ = child.wait();
}

struct Exit {
    stderr: String,
    success: bool,
}

/// Runs the program, handing each stdout line to `on_line` as it arrives.
fn run(
    program: &Path,
    args: &[&str],
    stdin: Option<&str>,
    cwd: Option<&Path>,
    timeout: Duration,
    cancel: Option<&AtomicBool>,
    on_line: &mut dyn FnMut(&str),
) -> Result<Exit, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Don't flash a console window for each call.
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|e| format!("Couldn't start {}: {e}", program.display()))?;

    // Pipes are drained on their own threads so a chatty child can't
    // deadlock against us while we wait for it to exit.
    if let (Some(input), Some(mut pipe)) = (stdin, child.stdin.take()) {
        let input = input.to_owned();
        thread::spawn(move || {
            let _ = pipe.write_all(input.as_bytes());
        });
    }
    let stdout_pipe = child.stdout.take().ok_or("Couldn't capture CLI output.")?;
    let (sender, receiver) = mpsc::channel::<String>();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout_pipe);
        let mut buffer = Vec::new();
        while matches!(reader.read_until(b'\n', &mut buffer), Ok(read) if read > 0) {
            let line = String::from_utf8_lossy(&buffer).trim_end_matches(['\r', '\n']).to_string();
            buffer.clear();
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let mut stderr_pipe = child.stderr.take().ok_or("Couldn't capture CLI output.")?;
    let stderr_reader = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut buffer);
        buffer
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => on_line(&line),
            Err(RecvTimeoutError::Timeout) => {}
            // stdout is closed: the process is finishing.
            Err(RecvTimeoutError::Disconnected) => match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => thread::sleep(Duration::from_millis(50)),
                Err(e) => return Err(format!("Lost track of the CLI process: {e}")),
            },
        }
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            kill_tree(&mut child);
            return Err(STOPPED.into());
        }
        if Instant::now() >= deadline {
            kill_tree(&mut child);
            return Err(format!(
                "The CLI didn't finish within {} minutes, so it was stopped.",
                timeout.as_secs().div_ceil(60)
            ));
        }
    };

    Ok(Exit {
        stderr: String::from_utf8_lossy(&stderr_reader.join().unwrap_or_default()).into_owned(),
        success: status.success(),
    })
}

fn tail(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let count = trimmed.chars().count();
    trimmed.chars().skip(count.saturating_sub(max_chars)).collect()
}

fn version_of(path: &Path) -> Option<String> {
    let mut first_line = None;
    let exit = run(path, &["--version"], None, None, Duration::from_secs(10), None, &mut |line| {
        if first_line.is_none() && !line.trim().is_empty() {
            first_line = Some(line.trim().to_string());
        }
    })
    .ok()?;
    first_line.filter(|line| exit.success && line.chars().count() <= 60)
}

/// Reports every known provider and whether its CLI is installed.
pub fn detect_all() -> Vec<CliStatus> {
    let handles: Vec<_> = PROVIDERS
        .iter()
        .map(|provider| {
            thread::spawn(move || {
                let path = find_on_path(provider.bin);
                CliStatus {
                    id: provider.id.into(),
                    name: provider.name.into(),
                    command: provider.bin.into(),
                    installed: path.is_some(),
                    version: path.as_deref().and_then(version_of),
                    path: path.map(|p| p.display().to_string()),
                }
            })
        })
        .collect();

    handles.into_iter().filter_map(|handle| handle.join().ok()).collect()
}

/// Tool names end up on a command line, so only plain identifiers pass.
fn safe_tools(tools: &[String]) -> Vec<&str> {
    tools
        .iter()
        .map(String::as_str)
        .filter(|tool| !tool.is_empty() && tool.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .collect()
}

/// Interprets one line of Claude Code's `--output-format stream-json`.
fn parse_claude_line(event: &Value, state: &mut StreamState) -> Vec<Progress> {
    match event["type"].as_str() {
        Some("stream_event") => {
            let delta = &event["event"]["delta"];
            if delta["type"] == "text_delta" {
                let text = delta["text"].as_str().unwrap_or_default().to_string();
                state.streamed.push_str(&text);
                return vec![Progress::Text(text)];
            }
            vec![]
        }
        Some("assistant") => event["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|block| block["type"] == "tool_use")
            .filter_map(|block| block["name"].as_str())
            .map(|name| Progress::Tool(name.to_string()))
            .collect(),
        Some("system") if event["subtype"] == "permission_denied" => {
            let tool = event["tool_name"].as_str().unwrap_or_default().to_string();
            state.deny(&tool);
            vec![Progress::Denied { tool, message: event["message"].as_str().unwrap_or_default().to_string() }]
        }
        Some("result") => {
            let result = event["result"].as_str().unwrap_or_default().to_string();
            if event["is_error"] == true {
                state.failed = Some(result);
            } else {
                state.result = Some(result);
            }
            vec![]
        }
        _ => vec![],
    }
}

/// Interprets one line of Antigravity's `--output-format stream-json`.
fn parse_antigravity_line(event: &Value, state: &mut StreamState) -> Vec<Progress> {
    match event["event"].as_str() {
        Some("step_update") => {
            let step = &event["step_update"];
            match step["step_type"].as_str() {
                Some("agent_response") => match step["text_delta"].as_str() {
                    Some(text) if !text.is_empty() => {
                        state.streamed.push_str(text);
                        vec![Progress::Text(text.to_string())]
                    }
                    _ => vec![],
                },
                Some("tool") if step["state"] == "ACTIVE" => {
                    vec![Progress::Tool(step["tool_name"].as_str().unwrap_or("a tool").to_string())]
                }
                _ => vec![],
            }
        }
        Some("result") => {
            let result = &event["result"];
            let mut progress = Vec::new();
            for action in result["denied_actions"].as_array().into_iter().flatten() {
                let tool = action["action"].as_str().unwrap_or_default().to_string();
                state.deny(&tool);
                progress.push(Progress::Denied {
                    message: format!("Headless mode can't ask for the \"{tool}\" permission, so it was denied."),
                    tool,
                });
            }
            let response = result["response"].as_str().unwrap_or_default().to_string();
            if result["status"] == "SUCCESS" {
                state.result = Some(response);
            } else {
                state.failed = Some(format!("{} {response}", result["status"].as_str().unwrap_or("FAILED")));
            }
            progress
        }
        _ => vec![],
    }
}

fn parse_line(provider_id: &str, line: &str, state: &mut StreamState) -> Vec<Progress> {
    let streams_json = matches!(provider_id, "claude" | "antigravity");
    if !streams_json {
        // Plain-text CLIs: every line is part of the answer.
        state.streamed.push_str(line);
        state.streamed.push('\n');
        return vec![Progress::Text(format!("{line}\n"))];
    }
    if line.trim().is_empty() {
        return vec![];
    }
    match serde_json::from_str::<Value>(line) {
        Ok(event) if provider_id == "claude" => parse_claude_line(&event, state),
        Ok(event) => parse_antigravity_line(&event, state),
        Err(_) => vec![Progress::Log(line.to_string())],
    }
}

/// Sends one prompt through the provider's CLI in headless mode, reporting
/// progress as it happens, and returns the final answer.
pub fn prompt(
    provider_id: &str,
    prompt: &str,
    options: &RunOptions,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<RunResult, String> {
    let provider = PROVIDERS
        .iter()
        .find(|provider| provider.id == provider_id)
        .ok_or_else(|| format!("Unknown AI agent \"{provider_id}\"."))?;
    let path = find_on_path(provider.bin).ok_or_else(|| {
        format!("{} isn't installed (no `{}` on PATH). Check Settings > AI agents.", provider.name, provider.bin)
    })?;

    let allowed = safe_tools(options.allowed_tools);
    let claude_allowed = allowed.join(",");
    let antigravity_settings = serde_json::json!({
        "permissions": { "allow": allowed.iter().map(|tool| format!("{tool}(*)")).collect::<Vec<_>>() }
    })
    .to_string();

    let mut args: Vec<&str> = Vec::new();
    let mut stdin = Some(prompt);
    match provider.id {
        "claude" => {
            args.extend(["-p", "--output-format", "stream-json", "--verbose", "--include-partial-messages"]);
            if !allowed.is_empty() {
                args.extend(["--allowedTools", &claude_allowed]);
            }
        }
        // agy has no stdin mode: the prompt must be the value of -p.
        "antigravity" => {
            if prompt.chars().count() > MAX_ARG_PROMPT_CHARS {
                return Err(format!(
                    "This prompt is too long for {} (it only accepts up to {MAX_ARG_PROMPT_CHARS} characters). Shorten the idea skill or plan fewer posts.",
                    provider.name
                ));
            }
            stdin = None;
            args.extend(["-p", prompt, "--output-format", "stream-json"]);
            if !allowed.is_empty() {
                args.extend(["--settings", &antigravity_settings]);
            }
        }
        "gemini" => args.extend(["--skip-trust", "-p", "Follow the instructions given on stdin."]),
        "codex" => args.extend(["exec", "--skip-git-repo-check", "-"]),
        _ => unreachable!("every provider in PROVIDERS has an invocation"),
    }

    let mut state = StreamState::default();
    let exit = run(&path, &args, stdin, Some(options.cwd), options.timeout, options.cancel, &mut |line| {
        for progress in parse_line(provider.id, line, &mut state) {
            on_progress(progress);
        }
    })?;

    let text = state.text();
    // A refused permission is reported to the caller, who can offer to allow it.
    if !state.denied.is_empty() {
        return Ok(RunResult { text, denied: state.denied });
    }
    if let Some(failure) = state.failed {
        return Err(format!("{} reported an error:\n{}", provider.name, tail(&failure, 600)));
    }
    if !exit.success {
        let detail = if exit.stderr.trim().is_empty() { &text } else { &exit.stderr };
        return Err(format!("{} reported an error:\n{}", provider.name, tail(detail, 600)));
    }
    if text.is_empty() {
        return Err(format!("{} finished without printing anything.", provider.name));
    }
    Ok(RunResult { text, denied: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn feed(provider: &str, lines: &[&str]) -> (StreamState, Vec<Progress>) {
        let mut state = StreamState::default();
        let progress = lines.iter().flat_map(|line| parse_line(provider, line, &mut state)).collect();
        (state, progress)
    }

    #[test]
    fn finds_a_cli_by_its_platform_executable_name() {
        let dir = std::env::temp_dir().join(format!("ge-cli-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = if cfg!(windows) { "fakecli.cmd" } else { "fakecli" };
        fs::write(dir.join(file), "").unwrap();

        let found = find_in_dirs("fakecli", [dir.clone()].into_iter());
        let missing = find_in_dirs("othercli", [dir.clone()].into_iter());
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(found, Some(dir.join(file)));
        assert_eq!(missing, None);
    }

    #[test]
    fn tail_keeps_the_end_without_splitting_characters() {
        assert_eq!(tail("  ভুল error  ", 5), "error");
        assert_eq!(tail("short", 600), "short");
    }

    #[test]
    fn claude_stream_yields_text_as_it_is_written_and_the_final_result() {
        let (state, progress) = feed(
            "claude",
            &[
                r#"{"type":"system","subtype":"init","cwd":"C:\\x"}"#,
                r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"[{\"title\""}}}"#,
                r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":":\"A\"}]"}}}"#,
                r#"{"type":"result","subtype":"success","is_error":false,"result":"[{\"title\":\"A\"}]"}"#,
            ],
        );

        assert_eq!(progress, [Progress::Text("[{\"title\"".into()), Progress::Text(":\"A\"}]".into())]);
        assert_eq!(state.text(), r#"[{"title":"A"}]"#);
        assert!(state.denied.is_empty());
    }

    #[test]
    fn claude_stream_reports_tool_use_and_each_denied_tool_once() {
        let denied = r#"{"type":"system","subtype":"permission_denied","tool_name":"Write","message":"Claude requested permissions to write"}"#;
        let (state, progress) = feed(
            "claude",
            &[
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Creating"},{"type":"tool_use","name":"Write","input":{}}]}}"#,
                denied,
                denied,
            ],
        );

        assert_eq!(progress[0], Progress::Tool("Write".into()));
        assert_eq!(
            progress[1],
            Progress::Denied { tool: "Write".into(), message: "Claude requested permissions to write".into() }
        );
        assert_eq!(state.denied, ["Write"]);
    }

    #[test]
    fn antigravity_stream_yields_text_tools_and_denied_actions() {
        let (state, progress) = feed(
            "antigravity",
            &[
                r#"{"event":"step_update","step_update":{"state":"ACTIVE","step_type":"agent_response","text_delta":"Hello"}}"#,
                r#"{"event":"step_update","step_update":{"state":"ACTIVE","step_type":"tool","tool_name":"write_file"}}"#,
                r#"{"event":"step_update","step_update":{"state":"ERROR","step_type":"tool","tool_name":"write_file"}}"#,
                "jetski: no output produced",
                r#"{"event":"result","result":{"status":"SUCCESS","response":"","denied_actions":[{"action":"write_file","display_name":"WriteFile"}]}}"#,
            ],
        );

        assert_eq!(progress[0], Progress::Text("Hello".into()));
        assert_eq!(progress[1], Progress::Tool("write_file".into()));
        assert_eq!(progress[2], Progress::Log("jetski: no output produced".into()));
        assert!(matches!(&progress[3], Progress::Denied { tool, .. } if tool == "write_file"));
        assert_eq!(state.denied, ["write_file"]);
        assert_eq!(state.text(), "Hello");
    }

    #[test]
    fn plain_text_clis_treat_every_line_as_the_answer() {
        let (state, progress) = feed("codex", &["[", "]"]);

        assert_eq!(progress, [Progress::Text("[\n".into()), Progress::Text("]\n".into())]);
        assert_eq!(state.text(), "[\n]");
    }

    #[test]
    fn only_plain_identifiers_are_passed_on_as_allowed_tools() {
        let tools = ["write_file".to_string(), "Write".to_string(), "x\" --evil".to_string(), String::new()];

        assert_eq!(safe_tools(&tools), ["write_file", "Write"]);
    }
}
