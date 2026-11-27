//! Local application state — projects, characters, shots, assets and run history.
//!
//! Persisted to ~/.h3-studio/state.json. Server-provided data (queue, GPU
//! stats, assets, history) is NOT persisted — it is fetched live.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub genre: String,
    pub description: String,
    pub aspect: String,
    pub fps: u32,
    pub resolution: String,
    pub updated_at: String,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub shots: Vec<Shot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shot {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub mode: String,      // FL2VA / Ref2VA
    pub duration: u32,     // seconds
    pub status: ShotStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShotStatus {
    Done,
    Running,
    Queued,
    Pending,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRecord {
    pub id: String,
    pub name: String,
    pub role: String,
    pub consistency: f32,   // 0..100
    pub accent: String,     // "teal" | "indigo" | "amber" | "purple"
    pub updated_at: String,
    pub refs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLocalState {
    pub projects: Vec<ProjectRecord>,
    pub characters: Vec<CharacterRecord>,
    pub selected_project_id: Option<String>,
    pub selected_shot_id: Option<String>,
    /// UI language — "en" (default) or "zh".
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "en".to_string()
}

impl AppLocalState {
    pub fn state_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".h3-studio").join("state.json")
    }

    pub fn load() -> Self {
        let path = Self::state_path();
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str::<AppLocalState>(&text) {
                return state;
            }
        }
        Self::default_seeded()
    }

    pub fn save(&self) {
        let path = Self::state_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, text);
        }
    }

    pub fn default_seeded() -> Self {
        // Bamboo Grove — the demo project from the design system.
        let scenes: Vec<Scene> = vec![
            Scene {
                id: "scene-01".into(),
                name: "Rainy Bamboo Forest".into(),
                shots: (1..=8)
                    .map(|i| Shot {
                        id: format!("shot-01-{:02}", i),
                        number: i,
                        title: [
                            "Establishing wide — bamboo forest",
                            "Sword drawn close-up",
                            "Rain walk through bamboo",
                            "Cloak flutter in wind",
                            "Scene 02 — moonrise wide",
                            "Duel preparation",
                            "Reveal hidden blade",
                            "Misty retreat",
                        ][(i - 1) as usize]
                        .into(),
                        mode: if i % 2 == 0 { "Ref2VA".into() } else { "FL2VA".into() },
                        duration: [6, 4, 8, 5, 6, 4, 5, 4][(i - 1) as usize],
                        status: match i {
                            1 => ShotStatus::Done,
                            2 => ShotStatus::Done,
                            3 => ShotStatus::Running,
                            4 => ShotStatus::Queued,
                            _ => ShotStatus::Pending,
                        },
                    })
                    .collect(),
            },
            Scene {
                id: "scene-02".into(),
                name: "Moonrise Duel".into(),
                shots: (1..=6).map(|i| Shot {
                    id: format!("shot-02-{:02}", i),
                    number: i,
                    title: format!("Scene 02 · shot {}", i),
                    mode: "Ref2VA".into(),
                    duration: 5,
                    status: ShotStatus::Pending,
                }).collect(),
            },
            Scene {
                id: "scene-03".into(),
                name: "Blade Awakening".into(),
                shots: (1..=6).map(|i| Shot {
                    id: format!("shot-03-{:02}", i),
                    number: i,
                    title: format!("Scene 03 · shot {}", i),
                    mode: "FL2VA".into(),
                    duration: 4,
                    status: ShotStatus::Pending,
                }).collect(),
            },
            Scene {
                id: "scene-04".into(),
                name: "Village Festival".into(),
                shots: (1..=6).map(|i| Shot {
                    id: format!("shot-04-{:02}", i),
                    number: i,
                    title: format!("Scene 04 · shot {}", i),
                    mode: "FL2VA".into(),
                    duration: 4,
                    status: ShotStatus::Pending,
                }).collect(),
            },
        ];

        let project = ProjectRecord {
            id: "bamboo-grove".into(),
            name: "Bamboo Grove".into(),
            genre: "Wuxia".into(),
            description: "Wuxia · female swordswoman · shot on H3 FL2VA + Ref2VA".into(),
            aspect: "16:9".into(),
            fps: 24,
            resolution: "1920×1080".into(),
            updated_at: "2m ago".into(),
            scenes,
        };

        let jade_blade = ProjectRecord {
            id: "jade-blade".into(),
            name: "Jade Blade".into(),
            genre: "Character test".into(),
            description: "Character test · expression & costume study".into(),
            aspect: "16:9".into(),
            fps: 24,
            resolution: "1920×1080".into(),
            updated_at: "3h ago".into(),
            scenes: vec![Scene {
                id: "jade-s1".into(),
                name: "Jade Blade · Act 01".into(),
                shots: vec![
                    Shot { id: "jade-1".into(), number: 1, title: "Face close-up".into(), mode: "FL2VA".into(), duration: 4, status: ShotStatus::Done },
                    Shot { id: "jade-2".into(), number: 2, title: "Costume detail".into(), mode: "Ref2VA".into(), duration: 3, status: ShotStatus::Done },
                    Shot { id: "jade-3".into(), number: 3, title: "Expression turn".into(), mode: "Ref2VA".into(), duration: 4, status: ShotStatus::Done },
                    Shot { id: "jade-4".into(), number: 4, title: "Standing wide".into(), mode: "Ref2VA".into(), duration: 5, status: ShotStatus::Done },
                    Shot { id: "jade-5".into(), number: 5, title: "Emotional beat".into(), mode: "Ref2VA".into(), duration: 3, status: ShotStatus::Queued },
                    Shot { id: "jade-6".into(), number: 6, title: "Action hold".into(), mode: "Ref2VA".into(), duration: 4, status: ShotStatus::Pending },
                ],
            }],
        };

        let characters = vec![
            CharacterRecord { id: "char_001".into(), name: "Xiao Mei".into(), role: "Swordswoman".into(), consistency: 92.0, accent: "teal".into(), updated_at: "4m ago".into(), refs: 12 },
            CharacterRecord { id: "char_002".into(), name: "Uncle Chen".into(), role: "Village Elder".into(), consistency: 88.0, accent: "indigo".into(), updated_at: "2h ago".into(), refs: 5 },
            CharacterRecord { id: "char_003".into(), name: "Captain Zhao".into(), role: "Border Guard".into(), consistency: 74.0, accent: "amber".into(), updated_at: "5m ago".into(), refs: 3 },
            CharacterRecord { id: "char_004".into(), name: "Bamboo Spirit".into(), role: "Forest Entity".into(), consistency: 81.0, accent: "purple".into(), updated_at: "6m ago".into(), refs: 1 },
        ];

        AppLocalState {
            projects: vec![project, jade_blade],
            characters,
            selected_project_id: Some("bamboo-grove".into()),
            selected_shot_id: Some("shot-01-03".into()),
            language: "en".into(),
        }
    }

    pub fn project(&self) -> Option<&ProjectRecord> {
        let id = self.selected_project_id.as_ref()?;
        self.projects.iter().find(|p| &p.id == id)
    }

    pub fn selected_shot(&self) -> Option<(&ProjectRecord, &Scene, &Shot)> {
        let id = self.selected_shot_id.as_ref()?;
        let p = self.project()?;
        for scene in &p.scenes {
            for shot in &scene.shots {
                if &shot.id == id {
                    return Some((p, scene, shot));
                }
            }
        }
        None
    }
}
