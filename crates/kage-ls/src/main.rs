use std::collections::HashMap;
use std::io::{Read, Write};

mod completions;

use completions::CompletionData;

fn main() {
    let data = CompletionData::new();
    let mut docs: HashMap<String, String> = HashMap::new();
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();

    loop {
        let Some(message) = read_message(&mut reader) else {
            break;
        };
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&message) else {
            continue;
        };

        let id = message.get("id").cloned();
        let method = message.get("method").and_then(|v| v.as_str()).map(String::from);

        let Some(method) = method else {
            // Response from the client; nothing to do.
            continue;
        };

        let params = message.get("params").cloned().unwrap_or(serde_json::Value::Null);

        match (method.as_str(), id) {
            ("initialize", Some(id)) => {
                let result = serde_json::json!({
                    "capabilities": {
                        "textDocumentSync": { "openClose": true, "change": 1 },
                        "completionProvider": {
                            "triggerCharacters": ["."],
                            "resolveProvider": false,
                        },
                        "hoverProvider": true,
                    },
                    "serverInfo": { "name": "kage-ls", "version": env!("CARGO_PKG_VERSION") },
                });
                send_response(&mut writer, &id, Some(result));
            }
            ("shutdown", Some(id)) => {
                send_response(&mut writer, &id, Some(serde_json::Value::Null));
            }
            ("shutdown", None) => {}
            (_, Some(id)) if is_request(&method) => {
                let result = handle_request(&method, &params, &mut docs, &data);
                send_response(&mut writer, &id, result);
            }
            (_, Some(id)) => {
                // Unknown request.
                send_error(&mut writer, &id, -32601, format!("method not found: {method}"));
            }
            (_, None) => {
                // Notification (initialized, didOpen, didChange, exit, $/..., ...).
                handle_notification(&method, &params, &mut docs);
                if method == "exit" {
                    break;
                }
            }
        }
    }
}

fn is_request(method: &str) -> bool {
    matches!(method, "textDocument/completion" | "textDocument/hover")
}

fn handle_request(
    method: &str,
    params: &serde_json::Value,
    docs: &mut HashMap<String, String>,
    data: &CompletionData,
) -> Option<serde_json::Value> {
    match method {
        "textDocument/completion" => Some(completion(params, docs, data)),
        "textDocument/hover" => hover(params, docs, data),
        _ => None,
    }
}

fn handle_notification(method: &str, params: &serde_json::Value, docs: &mut HashMap<String, String>) {
    match method {
        "textDocument/didOpen" => {
            if let Some(doc) = params
                .get("textDocument")
                .and_then(|d| Some((d.get("uri")?.as_str()?.to_string(), d.get("text")?.as_str()?.to_string())))
            {
                docs.insert(doc.0, doc.1);
            }
        }
        "textDocument/didChange" => {
            let uri = params
                .get("textDocument")
                .and_then(|d| d.get("uri"))
                .and_then(|u| u.as_str())
                .map(String::from);
            let text = params
                .get("contentChanges")
                .and_then(|changes| changes.as_array())
                .and_then(|changes| changes.last())
                .and_then(|change| change.get("text"))
                .and_then(|t| t.as_str())
                .map(String::from);
            if let (Some(uri), Some(text)) = (uri, text) {
                docs.insert(uri, text);
            }
        }
        "textDocument/didClose" => {
            if let Some(uri) = params
                .get("textDocument")
                .and_then(|d| d.get("uri"))
                .and_then(|u| u.as_str())
                .map(String::from)
            {
                docs.remove(&uri);
            }
        }
        _ => {}
    }
}

fn completion(
    params: &serde_json::Value,
    docs: &HashMap<String, String>,
    data: &CompletionData,
) -> serde_json::Value {
    let (text, position) = match document_context(params, docs) {
        Some(context) => context,
        None => {
            let items: Vec<_> = data.completions("").iter().map(|e| e.to_completion_item()).collect();
            return serde_json::json!({ "isIncomplete": false, "items": items });
        }
    };

    let (line, column) = position;
    let line_text = text.lines().nth(line).unwrap_or("");
    let before = utf16_prefix(line_text, column);

    // Word prefix being typed, e.g. "imageS" in "imageS".
    let word: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    // Member access trigger, e.g. "col." in "col.".
    let after_word = &before[..before.len() - word.len()];
    let is_member = after_word.ends_with('.');

    let mut items: Vec<serde_json::Value> = Vec::new();

    if is_member {
        items.extend(data.swizzle_completions().iter().map(|e| e.to_completion_item()));
    } else {
        items.extend(data.completions(&word).iter().map(|e| e.to_completion_item()));
        items.extend(word_completions(&text, &word));
    }

    serde_json::json!({ "isIncomplete": false, "items": items })
}

fn hover(
    params: &serde_json::Value,
    docs: &HashMap<String, String>,
    data: &CompletionData,
) -> Option<serde_json::Value> {
    let (text, position) = document_context(params, docs)?;
    let (line, column) = position;
    let line_text = text.lines().nth(line).unwrap_or("");
    let before = utf16_prefix(line_text, column);
    let word: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    data.hover(&word).map(|entry| {
        serde_json::json!({
            "contents": { "kind": "markdown", "value": format!("{}\n\n{}", entry.detail, entry.documentation) },
        })
    })
}

fn document_context(
    params: &serde_json::Value,
    docs: &HashMap<String, String>,
) -> Option<(String, (usize, usize))> {
    let uri = params
        .get("textDocument")
        .and_then(|d| d.get("uri"))
        .and_then(|u| u.as_str())?;
    let text = docs.get(uri).cloned()?;
    let position = params.get("position")?;
    let line = position.get("line")?.as_u64()? as usize;
    let character = position.get("character")?.as_u64()? as usize;
    Some((text, (line, character)))
}

/// Returns the part of `line` up to the given UTF-16 offset.
fn utf16_prefix(line: &str, utf16_offset: usize) -> String {
    let mut utf16_len = 0usize;
    for (index, c) in line.char_indices() {
        if utf16_len >= utf16_offset {
            return line[..index].to_string();
        }
        utf16_len += c.len_utf16();
    }
    line.to_string()
}

/// Plain word-based completions for identifiers used in the document.
fn word_completions(text: &str, prefix: &str) -> Vec<serde_json::Value> {
    let lowered = prefix.to_ascii_lowercase();
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            word.push(c);
        } else {
            if word.len() > 2 {
                words.push(std::mem::take(&mut word));
            }
            word.clear();
        }
    }
    if word.len() > 2 {
        words.push(word);
    }
    words.sort();
    words.dedup();

    words
        .into_iter()
        .filter(|candidate| {
            !candidate.eq_ignore_ascii_case(prefix)
                && candidate.to_ascii_lowercase().starts_with(&lowered)
        })
        .take(50)
        .map(|label| {
            serde_json::json!({
                "label": label,
                "kind": 1,
                "detail": "document word",
            })
        })
        .collect()
}

// --- LSP protocol plumbing ---

fn read_message(reader: &mut impl Read) -> Option<String> {
    let mut content_length: Option<usize> = None;
    let mut header = Vec::with_capacity(64);

    loop {
        let mut byte = [0u8; 1];
        if reader.read_exact(&mut byte).is_err() {
            return None;
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n") {
            let line = String::from_utf8_lossy(&header[..header.len() - 2]);
            if let Some(value) = line.strip_prefix("Content-Length: ") {
                content_length = value.trim().parse().ok();
            }
            header.clear();
            // Empty line terminates the header block.
            let mut next = [0u8; 2];
            // Peek: if the next two bytes are \r\n the headers are done.
            if reader.read_exact(&mut next).is_err() {
                return None;
            }
            if &next == b"\r\n" {
                break;
            }
            header.extend_from_slice(&next);
        }
    }

    let content_length = content_length?;
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

fn send_message(writer: &mut impl Write, body: &str) {
    let _ = write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body);
    let _ = writer.flush();
}

fn send_response(writer: &mut impl Write, id: &serde_json::Value, result: Option<serde_json::Value>) {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    });
    send_message(writer, &body.to_string());
}

fn send_error(writer: &mut impl Write, id: &serde_json::Value, code: i64, message: String) {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    });
    send_message(writer, &body.to_string());
}
