//! Blocking HTTP client for the comfyui-rust server.
//!
//! Every method returns `Result<_, String>` so the UI can degrade gracefully
//! when the server is offline.

use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct ComfyClient {
    base: String,
    pub http: Client,
}

impl ComfyClient {
    pub fn new(base: &str) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        ComfyClient { base: base.trim_end_matches('/').to_string(), http }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn get_json(&self, path: &str) -> Result<serde_json::Value, String> {
        let resp = self.http.get(self.url(path)).send().map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("GET {} → HTTP {}", path, status));
        }
        resp.json().map_err(|e| e.to_string())
    }

    fn post_json(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
        let resp = self.http.post(self.url(path)).json(&body).send().map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(format!("POST {} → HTTP {} {}", path, status, text));
        }
        let text = resp.text().map_err(|e| e.to_string())?;
        if text.trim().is_empty() {
            return Ok(serde_json::json!({"ok": true}));
        }
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    pub fn ping(&self) -> bool {
        self.http.get(self.url("/config")).timeout(Duration::from_secs(3)).send().map(|r| r.status().is_success()).unwrap_or(false)
    }

    pub fn submit_prompt(&self, workflow: serde_json::Value, client_id: &str) -> Result<serde_json::Value, String> {
        self.post_json("/prompt", serde_json::json!({
            "prompt": workflow,
            "extra_data": {"h3_studio": true},
            "client_id": client_id,
        }))
    }

    pub fn queue(&self) -> Result<serde_json::Value, String> {
        self.get_json("/queue")
    }

    pub fn history(&self, prompt_id: &str) -> Result<serde_json::Value, String> {
        self.get_json(&format!("/history/{}", prompt_id))
    }

    pub fn history_recent(&self) -> Result<serde_json::Value, String> {
        self.get_json("/history")
    }

    pub fn system_stats(&self) -> Result<serde_json::Value, String> {
        self.get_json("/system_stats")
    }

    pub fn list_assets(&self) -> Result<serde_json::Value, String> {
        self.get_json("/assets?limit=500")
    }

    pub fn list_input_images(&self) -> Result<serde_json::Value, String> {
        self.get_json("/input_images")
    }

    pub fn view_input_image(&self, filename: &str) -> Result<Vec<u8>, String> {
        let url = self.url(&format!("/view_input?filename={}", urlencoding(filename)));
        let resp = self.http.get(url).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("view_input {} → {}", filename, resp.status()));
        }
        resp.bytes().map_err(|e| e.to_string()).map(|b| b.to_vec())
    }

    pub fn view_output_image(&self, filename: &str) -> Result<Vec<u8>, String> {
        let url = self.url(&format!("/view?filename={}", urlencoding(filename)));
        let resp = self.http.get(url).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("view {} → {}", filename, resp.status()));
        }
        resp.bytes().map_err(|e| e.to_string()).map(|b| b.to_vec())
    }

    pub fn view_asset(&self, path: &str) -> Result<Vec<u8>, String> {
        let url = self.url(&format!("/view_asset?path={}", urlencoding(path)));
        let resp = self.http.get(url).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("view_asset {} → {}", path, resp.status()));
        }
        resp.bytes().map_err(|e| e.to_string()).map(|b| b.to_vec())
    }

    pub fn upload_asset(&self, file_bytes: Vec<u8>, filename: &str) -> Result<serde_json::Value, String> {
        let part = Part::bytes(file_bytes).file_name(filename.to_string());
        let form = Form::new().part("file", part);
        let resp = self.http.post(self.url("/assets/upload")).multipart(form).send().map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("upload → HTTP {} {}", status, text));
        }
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    pub fn upload_input_image(&self, file_bytes: Vec<u8>, filename: &str) -> Result<serde_json::Value, String> {
        let part = Part::bytes(file_bytes).file_name(filename.to_string());
        let form = Form::new().part("image", part);
        let resp = self.http.post(self.url("/upload/input_image")).multipart(form).send().map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("upload input image → HTTP {} {}", status, text));
        }
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }
}

fn urlencoding(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
