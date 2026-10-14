//! Opt-in local automation for the actual webview and native backend.
//! Compile with `agent-control`; set DW4_AGENT_TOKEN to start the server.
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tauri::{Manager, State};

type PendingReply = Option<(u64, mpsc::Sender<Value>)>;

#[derive(Default)]
pub struct AgentState {
    pending: Arc<Mutex<PendingReply>>,
    dialogs: Mutex<std::collections::VecDeque<(String, Option<String>)>>,
}

/// Request the same native close event as the window title-bar button.
#[tauri::command]
pub fn agent_close(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or("main window missing")?
        .close()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn agent_queue_dialog(
    kind: String,
    path: Option<String>,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    if kind != "open" && kind != "save" {
        return Err("dialog kind must be open or save".into());
    }
    state
        .dialogs
        .lock()
        .map_err(|e| e.to_string())?
        .push_back((kind, path));
    Ok(())
}

#[tauri::command]
pub fn agent_pick_path(
    kind: String,
    state: State<'_, AgentState>,
) -> Result<Option<String>, String> {
    let mut dialogs = state.dialogs.lock().map_err(|e| e.to_string())?;
    let Some((expected, _)) = dialogs.front() else {
        return Err("no agent dialog response queued".into());
    };
    if expected != &kind {
        return Err(format!("expected {expected} dialog, got {kind}"));
    }
    Ok(dialogs.pop_front().unwrap().1)
}

#[tauri::command]
pub fn agent_reply(id: u64, result: Value, state: State<'_, AgentState>) {
    let mut pending = state.pending.lock().unwrap();
    if pending
        .as_ref()
        .is_some_and(|(expected, _)| *expected == id)
        && let Some((_, sender)) = pending.take()
    {
        let _ = sender.send(result);
    }
}

fn request(stream: &mut TcpStream, token: &str) -> Result<String, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    let end = loop {
        let mut chunk = [0; 4096];
        let n = stream.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("incomplete request".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
        if bytes.len() > 1_048_576 {
            return Err("request too large".into());
        }
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break end + 4;
        }
    };
    let headers = std::str::from_utf8(&bytes[..end]).map_err(|e| e.to_string())?;
    if !headers.starts_with("POST /eval HTTP/1.1\r\n") {
        return Err("use POST /eval".into());
    }
    let mut length = None;
    let mut authorized = false;
    for line in headers.lines().skip(1) {
        if let Some((key, value)) = line.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse::<usize>().ok();
            }
            if key.eq_ignore_ascii_case("authorization") {
                authorized = value.trim() == format!("Bearer {token}");
            }
        }
    }
    if !authorized {
        return Err("unauthorized".into());
    }
    let length = length.ok_or("content-length required")?;
    if length > 1_000_000 {
        return Err("request too large".into());
    }
    while bytes.len() < end + length {
        let mut chunk = [0; 4096];
        let n = stream.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("incomplete body".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    let body: Value =
        serde_json::from_slice(&bytes[end..end + length]).map_err(|e| e.to_string())?;
    body["script"]
        .as_str()
        .map(str::to_owned)
        .ok_or("script required".into())
}

pub fn start(app: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let Ok(token) = std::env::var("DW4_AGENT_TOKEN") else {
        return Ok(());
    };
    if token.len() < 24 {
        return Err("DW4_AGENT_TOKEN must have at least 24 characters".into());
    }
    let port = std::env::var("DW4_AGENT_PORT")
        .unwrap_or_else(|_| "17365".into())
        .parse::<u16>()?;
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let pending = app.state::<AgentState>().pending.clone();
    std::thread::spawn(move || {
        for (index, mut stream) in listener.incoming().flatten().enumerate() {
            let id = index as u64 + 1;
            let result = request(&mut stream, &token).and_then(|script| {
                let (sender, receiver) = mpsc::channel();
                *pending.lock().unwrap() = Some((id, sender));
                let script = serde_json::to_string(&script).unwrap();
                let js = format!(
                    r#"(async () => {{
                    let result;
                    try {{ result = {{ok: true, value: (await (0,eval)({script})) ?? null}}; }}
                    catch (error) {{ result = {{ok: false, error: String(error)}}; }}
                    await window.__TAURI_INTERNALS__.invoke('agent_reply', {{id: {id}, result}});
                }})()"#
                );
                app.get_webview_window("main")
                    .ok_or("main window missing")?
                    .eval(&js)
                    .map_err(|e| e.to_string())?;
                receiver
                    .recv_timeout(Duration::from_secs(10))
                    .map_err(|e| e.to_string())
            });
            *pending.lock().unwrap() = None;
            let body = match result {
                Ok(value) => value,
                Err(error) => json!({"ok": false, "error": error}),
            }
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(packet: Vec<u8>) -> Result<String, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(address).unwrap();
            socket.write_all(&packet).unwrap();
            socket.shutdown(std::net::Shutdown::Write).unwrap();
        });
        let result = request(&mut listener.accept().unwrap().0, "test-only-bearer");
        client.join().unwrap();
        result
    }

    fn packet(body: &str, authorization: &str) -> Vec<u8> {
        format!("POST /eval HTTP/1.1\r\nAuthorization: {authorization}\r\nContent-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
    }

    #[test]
    fn authorized_scripts_preserve_unicode_and_literal_code() {
        let body = r#"{"script":"'α' + String(2)"}"#;
        assert_eq!(
            parse(packet(body, "Bearer test-only-bearer")).unwrap(),
            "'α' + String(2)"
        );
    }

    #[test]
    fn missing_or_wrong_bearer_is_rejected_before_evaluation() {
        for authorization in ["", "Bearer wrong", "Basic test-only-bearer"] {
            assert_eq!(
                parse(packet(r#"{"script":"1"}"#, authorization)).unwrap_err(),
                "unauthorized"
            );
        }
    }

    #[test]
    fn oversized_and_incomplete_requests_are_rejected() {
        let oversized = b"POST /eval HTTP/1.1\r\nAuthorization: Bearer test-only-bearer\r\nContent-Length: 1000001\r\n\r\n";
        assert_eq!(parse(oversized.to_vec()).unwrap_err(), "request too large");
        let truncated = b"POST /eval HTTP/1.1\r\nAuthorization: Bearer test-only-bearer\r\nContent-Length: 20\r\n\r\n{}";
        assert_eq!(parse(truncated.to_vec()).unwrap_err(), "incomplete body");
    }

    #[test]
    fn arbitrary_paths_and_non_script_json_are_rejected() {
        let other = b"POST /other HTTP/1.1\r\n\r\n";
        assert_eq!(parse(other.to_vec()).unwrap_err(), "use POST /eval");
        assert_eq!(
            parse(packet(r#"{"script":42}"#, "Bearer test-only-bearer")).unwrap_err(),
            "script required"
        );
    }
}
