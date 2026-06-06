//! Smart prompt syntax parser for PromptRelay.
//!
//! Supports two syntax modes:
//! - **Inline**: segments separated by `|` with optional `[n-m]` weight tags
//! - **Block**: segments preceded by header lines like `Scene 1:` or `Shot 2-4:`

use regex::Regex;
use std::sync::LazyLock;

static INLINE_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([\d\.]+)(?:[:\-]([\d\.]+))?\]").unwrap());

static DIGIT_RANGE_TAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([\d]+(?:\.\d+)?)\s*[-\u{2013}]\s*([\d]+(?:\.\d+)?)\s*$").unwrap());

/// A parsed segment with text and weight.
#[derive(Debug, Clone)]
pub struct Segment {
    pub text: String,
    pub weight: f64,
}

/// Try to parse a string as an integer.
fn try_parse_num(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Ok(val) = s.parse::<f64>() {
        return Some(val as i64);
    }
    None
}

/// Parse a block segment header line.
///
/// Returns `Some((start, end_or_None))` if the line is a valid header.
/// Valid format: one or more prefix words, followed by a number,
/// optional range, then colon at end of line.
///
/// Examples:
/// - `Scene 1:` -> (1, None)
/// - `My Scene 3:` -> (3, None)
/// - `Shot 2-4:` -> (2, 4)
fn parse_header(line: &str) -> Option<(i64, Option<i64>)> {
    let line = line.trim();
    if !line.ends_with(':') {
        return None;
    }
    let body = line[..line.len() - 1].trim_end();
    let tokens: Vec<&str> = body.split_whitespace().collect();
    // Need at least 2 tokens: at least one prefix word + one number token
    if tokens.len() < 2 {
        return None;
    }

    // Try digit range at end: "Scene 2-4:"
    if let Some(m) = DIGIT_RANGE_TAIL_RE.find(body) {
        let prefix = &body[..m.start()];
        if !prefix.trim().is_empty() {
            let caps = DIGIT_RANGE_TAIL_RE.captures(body).unwrap();
            let start = try_parse_num(caps.get(1).unwrap().as_str());
            let end = try_parse_num(caps.get(2).unwrap().as_str());
            if let (Some(s), Some(e)) = (start, end) {
                return Some((s, Some(e)));
            }
        }
    }

    // Try 1..N tail tokens as number (longest candidate first)
    let max_num_tokens = std::cmp::min(4, tokens.len() - 1);
    for n in (1..=max_num_tokens).rev() {
        let candidate = tokens[tokens.len() - n..].join(" ");
        if let Some(val) = try_parse_num(&candidate) {
            return Some((val, None));
        }
    }

    None
}

/// Extract first `[n]` or `[n-m]` weight tag from text.
/// Returns (clean_text, weight_or_None). Tag is stripped from text.
fn extract_inline_tag(text: &str) -> (String, Option<f64>) {
    if let Some(caps) = INLINE_TAG_RE.captures(text) {
        let val1: f64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0.0);
        let val2 = caps.get(2).map(|m| m.as_str().parse::<f64>().unwrap_or(0.0));
        let weight = match val2 {
            Some(v2) => v2 - val1,
            None => val1,
        };
        let clean = INLINE_TAG_RE.replace_all(text, "").trim().to_string();
        (clean, Some(weight))
    } else {
        (text.trim().to_string(), None)
    }
}

/// Parse pipe-separated inline syntax with optional `[n-m]` weight tags.
///
/// Syntax examples:
/// - `one | two | three` -> equal weights
/// - `one [0-50] | two [50-150] | three [150]` -> proportional weights
fn parse_inline_syntax(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    for part in text.split('|') {
        let (clean, weight) = extract_inline_tag(part);
        if !clean.is_empty() {
            segments.push(Segment {
                text: clean,
                weight: weight.unwrap_or(1.0),
            });
        }
    }
    segments
}

/// Parse block header syntax where each segment is preceded by a header line.
///
/// Header format: any words followed by a number (or word-number) and a colon
/// on its own line. Optional `[n-m]` inline tag in body overrides header weight.
fn parse_block_syntax(text: &str) -> Vec<Segment> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut raw_segments: Vec<(Option<(i64, Option<i64>)>, String)> = Vec::new();
    let mut current_header: Option<(i64, Option<i64>)> = None;
    let mut current_body = String::new();

    for line in &lines {
        if let Some(h) = parse_header(line) {
            if !current_body.is_empty() || current_header.is_some() {
                raw_segments.push((current_header, current_body.clone()));
            }
            current_header = Some(h);
            current_body.clear();
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if !current_body.is_empty() || current_header.is_some() {
        raw_segments.push((current_header, current_body.clone()));
    }

    let mut segments = Vec::new();
    for (header, body) in &raw_segments {
        let (clean, inline_weight) = extract_inline_tag(body);
        if clean.is_empty() {
            continue;
        }
        let weight = if let Some(iw) = inline_weight {
            iw
        } else if let Some((start, end)) = header {
            match end {
                Some(e) => (*e - *start) as f64,
                None => 1.0,
            }
        } else {
            1.0
        };
        segments.push(Segment {
            text: clean,
            weight,
        });
    }
    segments
}

/// Parse smart_prompt text into a list of segments with text and weight.
///
/// Detects syntax automatically:
///
/// **Inline** (newline-agnostic):
/// Segments separated by `|` with optional `[n-m]` proportional weight tags.
/// - `man walks | man runs | man jumps`
/// - `man walks [0-50] | man runs [50-150] | man jumps [150-200]`
///
/// **Block** (newline-specific):
/// Segments preceded by a header line: any words + number + colon on its own line.
/// - `Scene 1:\nman walks\nScene 2:\nman runs`
/// - `My Shot 1-3:\nman walks\nMy Shot 3-7:\nman runs`
pub fn parse_smart_prompt(text: &str) -> Vec<Segment> {
    let has_blocks = text.lines().any(|line| parse_header(line).is_some());
    if has_blocks {
        parse_block_syntax(text)
    } else {
        parse_inline_syntax(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_equal_weights() {
        let segments = parse_smart_prompt("one | two | three");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].text, "one");
        assert_eq!(segments[0].weight, 1.0);
        assert_eq!(segments[1].text, "two");
        assert_eq!(segments[2].text, "three");
    }

    #[test]
    fn test_inline_proportional_weights() {
        let segments = parse_smart_prompt("one [0-50] | two [50-150] | three [150]");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].weight, 50.0);
        assert_eq!(segments[1].weight, 100.0);
        assert_eq!(segments[2].weight, 150.0);
    }

    #[test]
    fn test_block_syntax() {
        let text = "Scene 1:\nman walks\nScene 2:\nman runs";
        let segments = parse_smart_prompt(text);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "man walks");
        assert_eq!(segments[0].weight, 1.0);
        assert_eq!(segments[1].text, "man runs");
        assert_eq!(segments[1].weight, 1.0);
    }

    #[test]
    fn test_block_range_header() {
        let text = "Shot 1-3:\nman walks\nShot 3-7:\nman runs";
        let segments = parse_smart_prompt(text);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].weight, 2.0); // 3-1
        assert_eq!(segments[1].weight, 4.0); // 7-3
    }

    #[test]
    fn test_empty_input() {
        let segments = parse_smart_prompt("");
        assert!(segments.is_empty());
    }
}
