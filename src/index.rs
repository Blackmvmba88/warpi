use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Historial de comandos donde la IA tuvo que intervenir (traducciones o sugerencias de error)
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
    }

    pub fn record_ai_assistance(&mut self, command: &str) {
        let tool = Self::extract_tool(command);
        self.ai.total_interventions += 1;
        
        // Registramos también en las stats del usuario que necesitó ayuda con esta herramienta
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
    }
}
