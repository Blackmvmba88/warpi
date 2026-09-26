use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillCategory {
    pub name: String,
    pub manual_successes: u32,
    pub manual_failures: u32,
    pub ai_assisted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserIndex {
    /// Historial de comandos ejecutados manualmente por categoría/herramienta (ej. "git" -> Stats)
    pub skills: HashMap<String, SkillCategory>,
    pub total_commands_typed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AIIndex {
    /// Historial de intervenciones de la IA (traducciones o correcciones)
    pub interventions: HashMap<String, SkillCategory>,
    pub total_interventions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileTracker {
    pub user: UserIndex,
    pub ai: AIIndex,
}

impl ProfileTracker {
    pub fn new() -> Self {
        Self::default()
    }

    fn storage_path() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".warp_profile.json")
        } else {
            PathBuf::from(".warp_profile.json")
        }
    }

    /// Carga el perfil persistido automáticamente o crea uno nuevo
    pub fn load_or_default() -> Self {
        let path = Self::storage_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(tracker) = serde_json::from_str(&data) {
                return tracker;
            }
        }
        Self::new()
    }

    /// Guarda automáticamente el perfil en disco
    pub fn save(&self) {
        let path = Self::storage_path();
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, data);
        }
    }

    /// Analiza el comando base (ej. "git commit" -> "git")
    fn extract_tool(command: &str) -> String {
        command.split_whitespace().next().unwrap_or("unknown").to_string()
    }

    pub fn record_manual_success(&mut self, command: &str) {
        let tool = Self::extract_tool(command);
        self.user.total_commands_typed += 1;
        let entry = self.user.skills.entry(tool.clone()).or_insert(SkillCategory {
            name: tool,
            manual_successes: 0,
            manual_failures: 0,
            ai_assisted: 0,
        });
        entry.manual_successes += 1;
        self.save();
    }

    pub fn record_manual_failure(&mut self, command: &str) {
        let tool = Self::extract_tool(command);
        self.user.total_commands_typed += 1;
        let entry = self.user.skills.entry(tool.clone()).or_insert(SkillCategory {
            name: tool,
            manual_successes: 0,
            manual_failures: 0,
            ai_assisted: 0,
        });
        entry.manual_failures += 1;
        self.save();
    }

    pub fn record_ai_assistance(&mut self, command: &str) {
        let tool = Self::extract_tool(command);
        self.ai.total_interventions += 1;

        let user_entry = self.user.skills.entry(tool.clone()).or_insert(SkillCategory {
            name: tool.clone(),
            manual_successes: 0,
            manual_failures: 0,
            ai_assisted: 0,
        });
        user_entry.ai_assisted += 1;

        let ai_entry = self.ai.interventions.entry(tool.clone()).or_insert(SkillCategory {
            name: tool,
            manual_successes: 0,
            manual_failures: 0,
            ai_assisted: 0,
        });
        ai_entry.ai_assisted += 1;
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_tracker_skills() {
        let mut tracker = ProfileTracker::new();
        tracker.record_manual_success("cargo build");
        tracker.record_manual_failure("cargo test");
        tracker.record_ai_assistance("cargo run");

        assert_eq!(tracker.user.total_commands_typed, 2);
        assert_eq!(tracker.ai.total_interventions, 1);

        let cargo = tracker.user.skills.get("cargo").unwrap();
        assert_eq!(cargo.manual_successes, 1);
        assert_eq!(cargo.manual_failures, 1);
        assert_eq!(cargo.ai_assisted, 1);
    }
}
