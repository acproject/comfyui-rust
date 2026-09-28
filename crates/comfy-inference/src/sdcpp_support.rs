//! Knowledge of which model families
//! [stable-diffusion.cpp](https://github.com/leejet/stable-diffusion.cpp) supports.
//!
//! A built-in snapshot (keywords + metadata, dated 2026-09-28) is always
//! available. At runtime the upstream README can be scanned (see
//! [`SdCppRegistry::merge_upstream_readme`]); the result is merged with the
//! built-in table and cached on disk as JSON. Matching against local model
//! filenames is keyword based with token boundaries, preferring the most
//! specific (longest) keyword.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

/// Upstream README used for the "scan website" refresh.
pub const UPSTREAM_README: &str =
    "https://raw.githubusercontent.com/leejet/stable-diffusion.cpp/master/README.md";

const CACHE_FILENAME: &str = "sdcpp_supported.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdCppModelKind {
    Image,
    ImageEdit,
    Video,
    Other,
}

/// One supported model family entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdCppModelEntry {
    /// Stable slug, usually the docs filename without extension.
    pub id: String,
    /// Human readable name.
    pub name: String,
    pub kind: SdCppModelKind,
    /// Docs path on upstream, e.g. `docs/qwen_image_2.1.md`.
    pub doc: Option<String>,
    /// Lowercase keywords; matching normalizes punctuation to spaces.
    pub keywords: Vec<String>,
    /// Announced date (YYYY-MM-DD) when known.
    pub added: Option<String>,
    /// True for entries shipped in the binary snapshot.
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CacheFile {
    /// UNIX seconds when the upstream scan happened.
    fetched_at_unix: u64,
    source: String,
    entries: Vec<SdCppModelEntry>,
}

/// Immutable view of the registry, safe to hand to API handlers.
#[derive(Debug, Clone)]
pub struct RegistrySnapshot {
    pub entries: Vec<SdCppModelEntry>,
    /// "builtin" | "cache"
    pub source: String,
    pub fetched_at_unix: u64,
    pub upstream: String,
}

struct SdCppRegistry {
    entries: Vec<SdCppModelEntry>,
    source: String,
    fetched_at_unix: u64,
    cache_path: Option<PathBuf>,
}

fn registry() -> &'static RwLock<SdCppRegistry> {
    static REGISTRY: OnceLock<RwLock<SdCppRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(SdCppRegistry {
        entries: builtin_entries(),
        source: "builtin".to_string(),
        fetched_at_unix: 0,
        cache_path: None,
    }))
}

/// Initialise / re-initialise the registry from an on-disk cache if present.
/// `cache_dir` is the application config directory.
pub fn init(cache_dir: Option<&Path>) {
    let mut reg = registry().write().expect("sdcpp registry poisoned");
    reg.cache_path = cache_dir.map(|d| d.join(CACHE_FILENAME));
    if let Some(path) = reg.cache_path.as_ref() {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(cache) = serde_json::from_slice::<CacheFile>(&bytes) {
                reg.entries = merge_entries(builtin_entries(), cache.entries);
                reg.source = "cache".to_string();
                reg.fetched_at_unix = cache.fetched_at_unix;
                tracing::info!(
                    "stable-diffusion.cpp support list loaded from cache ({} families, fetched_at={})",
                    reg.entries.len(),
                    cache.fetched_at_unix
                );
                return;
            }
        }
    }
    reg.entries = builtin_entries();
    reg.source = "builtin".to_string();
    reg.fetched_at_unix = 0;
}

/// Current snapshot of the registry.
pub fn snapshot() -> RegistrySnapshot {
    let reg = registry().read().expect("sdcpp registry poisoned");
    RegistrySnapshot {
        entries: reg.entries.clone(),
        source: reg.source.clone(),
        fetched_at_unix: reg.fetched_at_unix,
        upstream: UPSTREAM_README.to_string(),
    }
}

/// Match a local model path or file name against the supported families.
pub fn match_identifier(identifier: &str) -> Option<SdCppModelEntry> {
    let reg = registry().read().expect("sdcpp registry poisoned");
    match_in(identifier, &reg.entries)
}

/// Normalize text: lowercase, punctuation/underscores to spaces, collapsed.
fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn token_boundary_contains(haystack: &str, needle: &str) -> bool {
    let hay = haystack.as_bytes();
    let mut start = 0usize;
    while let Some(pos) = haystack[start..].find(needle) {
        let abs = start + pos;
        let before_ok = abs == 0 || hay.get(abs - 1).copied() == Some(b' ');
        let after = abs + needle.len();
        // A letter keyword may be directly followed by a version digit,
        // e.g. "ideogram" in "ideogram4", "krea" in "krea2".
        let after_ok = after == hay.len()
            || hay.get(after).copied() == Some(b' ')
            || (needle
                .as_bytes()
                .last()
                .map(|c| c.is_ascii_alphabetic())
                .unwrap_or(false)
                && hay.get(after).copied().map(|c| c.is_ascii_digit()).unwrap_or(false));
        if before_ok && after_ok {
            return true;
        }
        start = abs + needle.len().max(1);
    }
    false
}

fn match_in(identifier: &str, entries: &[SdCppModelEntry]) -> Option<SdCppModelEntry> {
    let name = normalize(identifier);
    if name.is_empty() {
        return None;
    }
    let mut best: Option<(usize, &SdCppModelEntry)> = None;
    for entry in entries {
        for kw in &entry.keywords {
            let kw_n = normalize(kw);
            if kw_n.len() >= 3 && token_boundary_contains(&name, &kw_n) {
                if best.map(|(len, _)| kw_n.len() > len).unwrap_or(true) {
                    best = Some((kw_n.len(), entry));
                }
            }
        }
    }
    best.map(|(_, e)| e.clone())
}

/// Derive matcher keywords from a docs slug and display name.
fn derive_keywords(slug: &str, name: &str) -> Vec<String> {
    let mut kws: Vec<String> = Vec::new();
    let slug_n = normalize(slug);
    if slug_n.len() >= 3 {
        kws.push(slug_n.clone());
    }
    // Name variants: "Qwen Image 2.1", "SD3/SD3.5" ...
    for part in name.split(['/', ',']) {
        let p = part.trim();
        if p.len() >= 3 {
            kws.push(normalize(p));
        }
    }
    kws.sort();
    kws.dedup();
    kws.retain(|k| k.len() >= 3);
    kws
}

/// Merge builtin entries with freshly parsed upstream entries.
/// Builtin keyword tables win (curated), upstream metadata augments.
fn merge_entries(
    builtin: Vec<SdCppModelEntry>,
    upstream: Vec<SdCppModelEntry>,
) -> Vec<SdCppModelEntry> {
    let mut merged: Vec<SdCppModelEntry> = builtin;
    for up in upstream {
        let key = up.id.clone();
        if let Some(existing) = merged.iter_mut().find(|e| e.id == key) {
            existing.name = up.name.clone();
            existing.kind = up.kind;
            existing.doc = up.doc.or_else(|| existing.doc.clone());
            if up.added.is_some() {
                existing.added = up.added;
            }
        } else {
            merged.push(up);
        }
    }
    merged
}

/// Parse the upstream README markdown into supported-family entries.
pub fn parse_upstream_readme(markdown: &str) -> Vec<SdCppModelEntry> {
    #[derive(Clone, Copy)]
    enum Section {
        None,
        Image,
        Edit,
        Video,
    }

    let mut section = Section::None;
    let mut news: Vec<(String, String)> = Vec::new();

    for raw_line in markdown.lines() {
        let line = raw_line.trim();
        let lower = line.to_lowercase();

        // News lines: `* **2026/09/20** ... **Day-0 support for Qwen-Image-2.1**`
        if lower.starts_with("* **20") {
            if let Some(date) = line
                .split("**")
                .nth(1)
                .map(|d| d.trim().replace('/', "-"))
            {
                // Last bolded segment usually names the model.
                if let Some(model) = line.split("**").filter(|s| !s.trim().is_empty()).last() {
                    let model = model
                        .trim()
                        .trim_start_matches("Day-0 support for")
                        .trim_start_matches("Day-1 support for")
                        .trim_start_matches("support for")
                        .trim();
                    if model.len() >= 3 {
                        news.push((date, model.to_string()));
                    }
                }
            }
        }

        if lower.contains("image edit models") {
            section = Section::Edit;
            continue;
        }
        if lower.contains("image models") {
            section = Section::Image;
            continue;
        }
        if lower.contains("video models") {
            section = Section::Video;
            continue;
        }
        // Any other top-level bullet section ends tracking.
        if line.starts_with("- ") && lower.contains("models") == false && !line.starts_with("- [") {
            section = Section::None;
            continue;
        }
    }

    let kind_of = |s: Section| match s {
        Section::Image => SdCppModelKind::Image,
        Section::Edit => SdCppModelKind::ImageEdit,
        Section::Video => SdCppModelKind::Video,
        Section::None => SdCppModelKind::Other,
    };

    let mut entries: Vec<SdCppModelEntry> = Vec::new();
    section = Section::None;
    let mut section_indent = 0usize;

    for raw_line in markdown.lines() {
        if raw_line.trim().is_empty() {
            continue;
        }
        let indent = raw_line.chars().take_while(|c| *c == ' ').count();
        let line = raw_line.trim();
        let lower = line.to_lowercase();

        // Subsection headings: "- Image Models", "- Image Edit Models", ...
        if line.starts_with('-') && lower.contains("models") && line.contains("](./") == false {
            if lower.contains("image edit models") {
                section = Section::Edit;
                section_indent = indent;
                continue;
            }
            if lower.contains("image models") {
                section = Section::Image;
                section_indent = indent;
                continue;
            }
            if lower.contains("video models") {
                section = Section::Video;
                section_indent = indent;
                continue;
            }
            // Any other bullet heading ends the tracked section.
            section = Section::None;
            continue;
        }

        if matches!(section, Section::None) {
            continue;
        }
        // A sibling/outer bullet (e.g. "- [PhotoMaker] support.") closes the
        // nested model list section.
        if indent <= section_indent {
            section = Section::None;
            continue;
        }
        // Entry line example: `- [Qwen Image 2.1](./docs/qwen_image_2.1.md)`
        let Some(rest) = line.strip_prefix("- [") else {
            continue;
        };
        let Some(close) = rest.find("](") else {
            continue;
        };
        let name = rest[..close].replace("`", "").trim().to_string();
        let link_rest = &rest[close + 2..];
        let link_end = link_rest.find(')').unwrap_or(link_rest.len());
        let doc_raw = link_rest[..link_end].trim();
        let slug = doc_raw
            .trim_start_matches("./")
            .trim_start_matches("docs/")
            .trim_end_matches(".md");

        if name.is_empty() || slug.is_empty() {
            continue;
        }

        // One docs page can be referenced by several links (e.g. docs/sd.md
        // covers both "SD1.x, SD2.x, SD-Turbo" and "SDXL, SDXL-Turbo"):
        // merge their display names instead of overwriting.
        if let Some(existing) = entries.iter_mut().find(|e| e.id == slug) {
            if !existing.name.contains(&name) {
                existing.name = format!("{} / {}", existing.name, name);
            }
            continue;
        }

        let mut keywords = derive_keywords(slug, &name);
        // The family name itself is always a keyword.
        keywords.push(normalize(&name));
        keywords.sort();
        keywords.dedup();

        entries.push(SdCppModelEntry {
            id: slug.to_string(),
            name: name.clone(),
            kind: kind_of(section),
            doc: Some(doc_raw.to_string()),
            keywords,
            added: find_news_date(&news, slug, &name),
            builtin: false,
        });
    }

    entries
}

fn find_news_date(news: &[(String, String)], slug: &str, name: &str) -> Option<String> {
    let target = normalize(&format!("{slug} {name}"));
    let target_tokens: Vec<&str> = target.split_whitespace().collect();
    let mut best: Option<(usize, String)> = None;
    for (date, phrase) in news {
        let phrase_n = normalize(phrase);
        // Keep short numeric version tokens ("2", "3.5" -> "3","5") so that
        // e.g. "FLUX.2-dev" cannot match the FLUX.1 family; drop 4-digit
        // numbers (years / "2509" style suffixes).
        let tokens: Vec<&str> = phrase_n
            .split_whitespace()
            .filter(|t| {
                t.len() >= 3
                    || (t.len() <= 3 && t.bytes().all(|b| b.is_ascii_digit()))
            })
            .filter(|t| !(t.len() == 4 && t.bytes().all(|b| b.is_ascii_digit())))
            .collect();
        if tokens.is_empty() {
            continue;
        }
        // Every announcement token must be present in the family descriptor.
        let hits = tokens
            .iter()
            .filter(|t| target_tokens.contains(t))
            .count();
        if hits == tokens.len() {
            // Most specific announcement wins; ties keep the earliest date
            // (the family's first support announcement).
            let better = match &best {
                None => true,
                Some((len, existing_date)) => {
                    tokens.len() > *len || (tokens.len() == *len && date < existing_date)
                }
            };
            if better {
                best = Some((tokens.len(), date.clone()));
            }
        }
    }
    best.map(|(_, d)| d)
}

/// Merge freshly scanned upstream entries into the global registry and
/// persist the raw upstream subset to the cache file.
pub fn merge_upstream_readme(markdown: &str, fetched_at_unix: u64) -> RegistrySnapshot {
    let upstream = parse_upstream_readme(markdown);
    let mut reg = registry().write().expect("sdcpp registry poisoned");
    reg.entries = merge_entries(builtin_entries(), upstream.clone());
    reg.source = "github".to_string();
    reg.fetched_at_unix = fetched_at_unix;

    if let Some(path) = reg.cache_path.as_ref() {
        let cache = CacheFile {
            fetched_at_unix,
            source: "github".to_string(),
            entries: upstream,
        };
        if let Ok(json) = serde_json::to_vec_pretty(&cache) {
            if std::fs::write(path, json).is_err() {
                tracing::warn!("failed to write sdcpp support cache to {}", path.display());
            }
        }
    }

    RegistrySnapshot {
        entries: reg.entries.clone(),
        source: reg.source.clone(),
        fetched_at_unix,
        upstream: UPSTREAM_README.to_string(),
    }
}

/// Built-in snapshot of the upstream supported-model list (2026-09-28).
/// Keywords are curated for local filename matching.
fn builtin_entries() -> Vec<SdCppModelEntry> {
    use SdCppModelKind::*;
    #[rustfmt::skip]
    let raw: &[(&str, &str, SdCppModelKind, Option<&str>, &str, &[&str])] = &[
        // id, name, kind, added, doc, keywords
        ("sd",            "SD1.x / SD2.x / SD-Turbo", Image, Some("2024-01-01"), "docs/sd.md",
            &["sd15", "sd14", "sd1 5", "sd1.5", "v1-5-pruned", "v1 5", "sd21", "sd2 1", "sd-turbo", "sd turbo"]),
        ("distilled_sd",  "Distilled SD1.x/SDXL",     Image, None, "docs/distilled_sd.md",
            &["distilled sd", "distilled-sd"]),
        ("sdxl",          "SDXL / SDXL-Turbo",        Image, None, "docs/sd.md",
            &["sdxl"]),
        ("sd3",           "SD3 / SD3.5",              Image, None, "docs/sd3.md",
            &["sd3", "sd3 5", "sd35"]),
        ("flux",          "FLUX.1 dev / schnell",     Image, None, "docs/flux.md",
            &["flux1", "flux 1", "flux-dev", "flux dev", "flux-schnell", "flux schnell", "flux"]),
        ("flux2",         "FLUX.2 dev / klein",       Image, Some("2025-11-30"), "docs/flux2.md",
            &["flux2", "flux 2", "flux.2"]),
        ("lens",          "Lens",                     Image, Some("2026-05-27"), "docs/lens.md",
            &["lens"]),
        ("chroma",        "Chroma",                   Image, None, "docs/chroma.md",
            &["chroma"]),
        ("chroma_radiance","Chroma1-Radiance",        Image, None, "docs/chroma_radiance.md",
            &["chroma1", "chroma radiance", "radiance"]),
        ("qwen_image",    "Qwen Image",               Image, Some("2025-10-12"), "docs/qwen_image.md",
            &["qwen image", "qwen-image", "qwenimage"]),
        ("qwen_image_2.1","Qwen Image 2.1",           Image, Some("2026-09-20"), "docs/qwen_image_2.1.md",
            &["qwen image 2 1", "qwen-image-2.1", "qwen_image_2_1", "qwen image 2"]),
        ("pid",           "PiD",                      Image, Some("2026-05-31"), "docs/pid.md",
            &["pid"]),
        ("longcat_image", "LongCat Image",            Image, None, "docs/longcat_image.md",
            &["longcat"]),
        ("z_image",       "Z-Image",                  Image, Some("2025-12-01"), "docs/z_image.md",
            &["z image", "z-image", "zimage"]),
        ("minit2i",       "MiniT2I",                  Image, None, "docs/minit2i.md",
            &["minit2i", "mini t2i"]),
        ("sensenova_u1",  "SenseNova U1.5",           Image, None, "docs/sensenova_u1.md",
            &["sensenova"]),
        ("ovis_image",    "Ovis-Image",               Image, None, "docs/ovis_image.md",
            &["ovis image", "ovis-image", "ovisimage"]),
        ("anima",         "Anima",                    Image, None, "docs/anima.md",
            &["anima"]),
        ("ernie_image",   "ERNIE-Image",              Image, None, "docs/ernie_image.md",
            &["ernie"]),
        ("boogu_image",   "Boogu Image",              Image, None, "docs/boogu_image.md",
            &["boogu"]),
        ("krea2",         "Krea2",                    Image, Some("2026-06-25"), "docs/krea2.md",
            &["krea"]),
        ("mage_flow",     "Mage-Flow",                Image, None, "docs/mage_flow.md",
            &["mage flow", "mage-flow", "mageflow"]),
        ("sefi_image",    "SeFi-Image",               Image, None, "docs/sefi_image.md",
            &["sefi"]),
        ("hidream_o1_image","HiDream-O1-Image",       Image, None, "docs/hidream_o1_image.md",
            &["hidream"]),
        ("ideogram4",     "Ideogram4",                Image, Some("2026-06-04"), "docs/ideogram4.md",
            &["ideogram4", "ideogram"]),
        ("llada_image",   "LLaDA-Image",              Image, None, "docs/llada_image.md",
            &["llada"]),
        ("kontext",       "FLUX.1-Kontext-dev",       ImageEdit, None, "docs/kontext.md",
            &["kontext"]),
        ("qwen_image_edit","Qwen Image Edit",         ImageEdit, Some("2025-10-13"), "docs/qwen_image_edit.md",
            &["qwen image edit", "qwen-image-edit"]),
        ("longcat_image_edit","LongCat Image Edit",   ImageEdit, None, "docs/longcat_image.md",
            &["longcat edit"]),
        ("boogu_image_edit","Boogu Image Edit",       ImageEdit, None, "docs/boogu_image.md",
            &["boogu edit"]),
        ("wan",           "Wan2.1 / Wan2.2",          Video, Some("2025-09-06"), "docs/wan.md",
            &["wan2 1", "wan2 2", "wan2.1", "wan2.2", "wan21", "wan22", "wan-2", "wan 2"]),
        ("minimax_h3",    "MiniMax-H3",               Video, Some("2026-08-04"), "docs/minimax_h3.md",
            &["minimax", "h3"]),
        ("ltx2",          "LTX-2.3 / LTX-2.5",        Video, Some("2026-05-17"), "docs/ltx2.md",
            &["ltx"]),
        ("hunyuan_video", "HunyuanVideo 1.5",         Video, None, "docs/hunyuan_video.md",
            &["hunyuan"]),
        ("lingbot_video", "LingBot-Video",            Video, None, "docs/lingbot_video.md",
            &["lingbot"]),
    ];

    raw.iter()
        .map(|(id, name, kind, added, doc, kws)| SdCppModelEntry {
            id: id.to_string(),
            name: name.to_string(),
            kind: *kind,
            doc: Some(doc.to_string()),
            keywords: kws.iter().map(|s| s.to_string()).collect(),
            added: added.map(|s| s.to_string()),
            builtin: true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_families() {
        let entries = builtin_entries();
        assert_eq!(
            match_in("sd3.5-large-fp16-q8_0.gguf", &entries).unwrap().id,
            "sd3"
        );
        assert_eq!(
            match_in("flux2-dev-Q8_0.gguf", &entries).unwrap().id,
            "flux2"
        );
        assert_eq!(
            match_in("anima-preview2.safetensors", &entries).unwrap().id,
            "anima"
        );
        assert_eq!(
            match_in("Qwen-Image-2.1/transformer/x.safetensors", &entries)
                .unwrap()
                .id,
            "qwen_image_2.1"
        );
        assert_eq!(
            match_in("wan2.1-t2v-14b-F16.gguf", &entries).unwrap().id,
            "wan"
        );
        assert!(match_in("random_file.safetensors", &entries).is_none());
    }

    #[test]
    fn parses_upstream_readme_sections() {
        let md = r#"
## Features
* **2026/09/20** 🚀 stable-diffusion.cpp adds **Day-0 support for Qwen-Image-2.1**
* **2025/11/30** 🚀 stable-diffusion.cpp now supports **FLUX.2-dev**
* **2025/10/12** 🚀 stable-diffusion.cpp now supports **Qwen-Image**
- Supported models
  - Image Models
    - [SD1.x, SD2.x, SD-Turbo](./docs/sd.md)
    - [SDXL, SDXL-Turbo](./docs/sd.md)
    - [SD3/SD3.5](./docs/sd3.md)
    - [Qwen Image 2.1](./docs/qwen_image_2.1.md)
  - [PhotoMaker](./docs/photo_maker.md) support.
  - Video Models
    - [Wan2.1/Wan2.2](./docs/wan.md)
- Supported weight formats
"#;
        let entries = parse_upstream_readme(md);
        let sd = entries.iter().find(|e| e.id == "sd").expect("sd entry");
        assert!(sd.name.contains("SD1.x"));
        assert!(sd.name.contains("SDXL"));
        let sd3 = entries.iter().find(|e| e.id == "sd3").expect("sd3 entry");
        assert_eq!(sd3.name, "SD3/SD3.5");
        let q21 = entries.iter().find(|e| e.id == "qwen_image_2.1").expect("q21");
        assert_eq!(q21.added.as_deref(), Some("2026-09-20"));
        let wan = entries.iter().find(|e| e.id == "wan").expect("wan");
        assert_eq!(wan.kind, SdCppModelKind::Video);
        assert!(entries.iter().all(|e| e.id != "photo_maker"), "PhotoMaker must not be parsed as video");
    }
}
