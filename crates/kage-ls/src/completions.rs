use serde::Deserialize;
use std::collections::BTreeMap;

/// Completion item kinds (LSP `CompletionItemKind`).
pub const KIND_FUNCTION: u8 = 3;
pub const KIND_SNIPPET: u8 = 15;
pub const KIND_TYPE: u8 = 7;
pub const KIND_KEYWORD: u8 = 14;
pub const KIND_PROPERTY: u8 = 10;

#[derive(Debug, Clone)]
pub struct Entry {
    pub label: String,
    pub insert: String,
    pub is_snippet: bool,
    pub kind: u8,
    pub detail: String,
    pub documentation: String,
}

impl Entry {
    fn plain(label: &str, kind: u8, detail: &str, documentation: &str) -> Self {
        Self {
            label: label.to_string(),
            insert: label.to_string(),
            is_snippet: false,
            kind,
            detail: detail.to_string(),
            documentation: documentation.to_string(),
        }
    }

    fn snippet(label: &str, insert: &str, detail: &str, documentation: &str) -> Self {
        Self {
            label: label.to_string(),
            insert: insert.to_string(),
            is_snippet: true,
            kind: KIND_SNIPPET,
            detail: detail.to_string(),
            documentation: documentation.to_string(),
        }
    }

    pub fn to_completion_item(&self) -> serde_json::Value {
        serde_json::json!({
            "label": self.label,
            "kind": self.kind,
            "detail": self.detail,
            "documentation": { "kind": "markdown", "value": self.documentation },
            "insertText": self.insert,
            "insertTextFormat": if self.is_snippet { 2 } else { 1 },
        })
    }
}

// --- VS Code snippet file parsing (ported from ebitengine-kage-vscode) ---

#[derive(Deserialize)]
#[serde(transparent)]
struct VsSnippetsFile(BTreeMap<String, VsSnippet>);

#[derive(Deserialize)]
struct VsSnippet {
    #[serde(default)]
    prefix: ListOrDirect,
    body: ListOrDirect,
    #[serde(default)]
    description: ListOrDirect,
}

#[derive(Deserialize, Default)]
#[serde(untagged)]
enum ListOrDirect {
    Single(String),
    List(Vec<String>),
    #[default]
    None,
}

impl ListOrDirect {
    fn lines(&self) -> Vec<&str> {
        match self {
            ListOrDirect::Single(s) => vec![s.as_str()],
            ListOrDirect::List(l) => l.iter().map(|s| s.as_str()).collect(),
            ListOrDirect::None => vec![],
        }
    }

    fn joined(&self, sep: &str) -> String {
        self.lines().join(sep)
    }
}

fn description_to_markdown(description: &ListOrDirect, name: &str) -> (String, String) {
    let lines = description.lines();
    let mut usage = String::new();
    let mut doc_lines: Vec<String> = Vec::new();

    for line in lines {
        if let Some(url) = line.strip_prefix("Documentation: ") {
            doc_lines.push(format!("\n[{name}]({url})"));
        } else if line.starts_with("Usage ") {
            usage = line.trim_start_matches("Usage ").to_string();
            doc_lines.push(format!("`{usage}`"));
        } else if !line.trim().is_empty() {
            doc_lines.push(line.to_string());
        }
    }

    (usage, doc_lines.join("\n\n"))
}

fn entries_from_snippets_json() -> Vec<Entry> {
    let raw = include_str!("../data/snippets.json");
    let file: VsSnippetsFile = serde_json::from_str(raw).expect("snippets.json is valid JSON");

    let mut entries = Vec::new();
    for (name, snippet) in file.0 {
        // Snippets commonly list "kage" as an extra trigger; the first
        // meaningful prefix is the one shown in editors.
        let prefix = snippet
            .prefix
            .lines()
            .into_iter()
            .find(|p| !p.is_empty() && *p != "kage");
        let Some(prefix) = prefix else { continue };

        // Swizzle templates already start with a dot and are listed separately.
        if prefix.starts_with('.') {
            continue;
        }

        let insert = snippet.body.joined("\n");
        let (usage, documentation) =
            description_to_markdown(&snippet.description, &format!("ebitengine: {name}"));

        if name == "main" {
            entries.push(Entry::snippet(
                prefix,
                &insert,
                "Fragment shader boilerplate",
                &format!(
                    "The basis for the shader.\n\n- `pos`: destination pixel position, xy is in range 0..N\n- `tex`: source texture texel position, xy is in range 0..1\n- `col`: supplemental color information given from vertices, rgba is in range 0..1\n\nReturns the current position color.\n{documentation}"
                ),
            ));
            continue;
        }

        let (is_template, kind) = match name.as_str() {
            "imageColorNAtPixel" | "imageColorNAtUnit" => (true, KIND_SNIPPET),
            _ => (false, KIND_FUNCTION),
        };

        if is_template {
            entries.push(Entry::snippet(
                prefix,
                &insert,
                &usage,
                &documentation,
            ));
        } else {
            let detail = if usage.is_empty() { prefix.to_string() } else { usage };
            entries.push(Entry {
                label: prefix.to_string(),
                insert,
                is_snippet: true,
                kind,
                detail,
                documentation,
            });
        }
    }
    entries
}

// --- statically known entries ---

const TYPES: &[(&str, &str)] = &[
    ("bool", "Boolean value (true or false)"),
    ("int", "Signed integer value"),
    ("float", "Floating point number"),
    ("vec2", "Two dimensional float vector (x, y)"),
    ("vec3", "Three dimensional float vector (x, y, z)"),
    ("vec4", "Four dimensional float vector (x, y, z, w)"),
    ("ivec2", "Two dimensional int vector (x, y)"),
    ("ivec3", "Three dimensional int vector (x, y, z)"),
    ("ivec4", "Four dimensional int vector (x, y, z, w)"),
    ("mat2", "2x2 matrix"),
    ("mat3", "3x3 matrix"),
    ("mat4", "4x4 matrix"),
];

const KEYWORDS: &[(&str, &str)] = &[
    ("package", "Package clause, e.g. `package main`"),
    ("var", "Variable declaration. Global vars are uniforms when uploaded from Go"),
    ("const", "Constant declaration"),
    ("func", "Function declaration"),
    ("if", "Conditional statement"),
    ("else", "Alternative branch of an if statement"),
    ("for", "Loop statement (the only loop in Kage)"),
    ("return", "Return statement"),
    ("true", "Boolean true constant"),
    ("false", "Boolean false constant"),
    ("nil", "Nil constant"),
];

/// Built-in Kage functions that are not covered by the snippets file.
const EXTRA_FUNCTIONS: &[(&str, &str, &str)] = &[
    (
        "imageSrc0At",
        "imageSrc0At(pos vec2) vec4",
        "Returns the color value as vec4 at the given position pos in texels of source image 0.",
    ),
    (
        "imageSrc1At",
        "imageSrc1At(pos vec2) vec4",
        "Returns the color value as vec4 at the given position pos in texels of source image 1.",
    ),
    (
        "imageSrc2At",
        "imageSrc2At(pos vec2) vec4",
        "Returns the color value as vec4 at the given position pos in texels of source image 2.",
    ),
    (
        "imageSrc3At",
        "imageSrc3At(pos vec2) vec4",
        "Returns the color value as vec4 at the given position pos in texels of source image 3.",
    ),
    (
        "imageSrc0UnsafeAt",
        "imageSrc0UnsafeAt(pos vec2) vec4",
        "Returns the color value at pos in texels of source image 0, but without boundary checks.",
    ),
    (
        "imageSrc1UnsafeAt",
        "imageSrc1UnsafeAt(pos vec2) vec4",
        "Returns the color value at pos in texels of source image 1, but without boundary checks.",
    ),
    (
        "imageSrc2UnsafeAt",
        "imageSrc2UnsafeAt(pos vec2) vec4",
        "Returns the color value at pos in texels of source image 2, but without boundary checks.",
    ),
    (
        "imageSrc3UnsafeAt",
        "imageSrc3UnsafeAt(pos vec2) vec4",
        "Returns the color value at pos in texels of source image 3, but without boundary checks.",
    ),
    (
        "texelFetch",
        "texelFetch(tex vec2, pos int) vec4",
        "Returns the texel value of the image number `pos` at the position `tex`.",
    ),
];

/// Vector component swizzle members.
const SWIZZLE_MEMBERS: &[&str] = &[
    "x", "y", "z", "w", "r", "g", "b", "a", "s", "t", "p", "q", "xy", "xyz", "xyzw", "rg", "rgb",
    "rgba",
];

pub struct CompletionData {
    entries: Vec<Entry>,
}

impl CompletionData {
    pub fn new() -> Self {
        let mut entries = entries_from_snippets_json();

        // The generic `imageSrcNAt` / `imageSrcNUnsafeAt` templates are
        // replaced by the concrete imageSrc0At..3 entries below.
        entries.retain(|entry| entry.label != "imageSrcNAt" && entry.label != "imageSrcNUnsafeAt");

        let existing: std::collections::HashSet<String> =
            entries.iter().map(|entry| entry.label.clone()).collect();

        for (name, doc) in TYPES {
            if existing.contains(*name) {
                continue;
            }
            entries.push(Entry::plain(name, KIND_TYPE, &format!("{name} type"), doc));
        }
        for (name, doc) in KEYWORDS {
            entries.push(Entry::plain(name, KIND_KEYWORD, name, doc));
        }
        for (name, usage, doc) in EXTRA_FUNCTIONS {
            if existing.contains(*name) {
                continue;
            }
            entries.push(Entry::plain(
                name,
                KIND_FUNCTION,
                usage,
                &format!("`{usage}`\n\n{doc}\n\n[Ebitengine shader docs](https://ebitengine.org/en/documents/shader.html)"),
            ));
        }

        entries.sort_by(|a, b| a.label.cmp(&b.label));
        Self { entries }
    }

    /// Returns builtin completions matching `prefix`.
    pub fn completions(&self, prefix: &str) -> Vec<&Entry> {
        let lowered = prefix.to_ascii_lowercase();
        let mut scored: Vec<(u8, &Entry)> = self
            .entries
            .iter()
            .filter_map(|entry| score_match(&entry.label.to_ascii_lowercase(), &lowered).map(|s| (s, entry)))
            .collect();
        scored.sort_by_key(|(score, entry)| (*score, entry.label.clone()));
        scored.into_iter().map(|(_, entry)| entry).collect()
    }

    /// Member completions offered after a dot (vector swizzling).
    pub fn swizzle_completions(&self) -> Vec<Entry> {
        SWIZZLE_MEMBERS
            .iter()
            .map(|member| Entry::plain(
                member,
                KIND_PROPERTY,
                "vector component",
                "Reads or writes one or more vector components (swizzling).",
            ))
            .collect()
    }

    /// Hover documentation for a word, if any.
    pub fn hover(&self, word: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|entry| entry.label == word)
    }
}

/// Scores how well `candidate` matches `prefix` (both lowercased).
/// Lower is better. Returns None when there is no match.
fn score_match(candidate: &str, prefix: &str) -> Option<u8> {
    if prefix.is_empty() {
        return Some(0);
    }
    if candidate.starts_with(prefix) {
        return Some(0);
    }
    let mut rest = candidate;
    let mut gap = 0u8;
    for c in prefix.chars() {
        loop {
            match rest.chars().next() {
                Some(next) if next == c => {
                    rest = &rest[next.len_utf8()..];
                    break;
                }
                Some(_) => {
                    rest = &rest[1..];
                    gap += 1;
                    if gap > 30 {
                        return None;
                    }
                }
                None => return None,
            }
        }
    }
    Some(gap.min(100))
}
