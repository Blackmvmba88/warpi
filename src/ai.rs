use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AiDiagnostic {
    pub explanation: String,
    pub suggested_cmd: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AiProvider {
    Gemini(String),
    OpenAI(String),
    OfflineHeuristics,
}

#[derive(Debug, Clone)]
pub struct AIAssistant {
    pub provider: AiProvider,
}

// OpenAI Schemas
#[derive(Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

// Gemini Schemas
#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiPartResp {
    text: Option<String>,
}

#[derive(Deserialize)]
struct GeminiContentResp {
    parts: Option<Vec<GeminiPartResp>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContentResp>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

impl AIAssistant {
    pub fn new() -> Self {
        if let Ok(gemini_key) = std::env::var("GEMINI_API_KEY") {
            if !gemini_key.trim().is_empty() {
                return Self {
                    provider: AiProvider::Gemini(gemini_key.trim().to_string()),
                };
            }
        }
        if let Ok(openai_key) = std::env::var("OPENAI_API_KEY") {
            if !openai_key.trim().is_empty() {
                return Self {
                    provider: AiProvider::OpenAI(openai_key.trim().to_string()),
                };
            }
        }
        Self {
            provider: AiProvider::OfflineHeuristics,
        }
    }

    pub fn provider_name(&self) -> &'static str {
        match self.provider {
            AiProvider::Gemini(_) => "Google Gemini Cloud",
            AiProvider::OpenAI(_) => "OpenAI GPT Cloud",
            AiProvider::OfflineHeuristics => "Motor Inteligente Offline",
        }
    }

    /// Diagnóstico autónomo y generación de comando correctivo
    pub async fn diagnose_error(&self, command: &str, output: &str, exit_code: i32) -> AiDiagnostic {
        // Intentar llamada remota si hay API Key configurada
        match &self.provider {
            AiProvider::Gemini(key) => {
                let prompt = format!(
                    "El comando de terminal '{}' falló con código {}.\nSalida:\n{}\nExplica la causa brevemente y en la última línea responde EXACTAMENTE: 'SUGGESTION: <comando_corregido>'.",
                    command, exit_code, output
                );
                if let Ok(ans) = self.call_gemini(key, &prompt).await {
                    return Self::parse_diagnostic_answer(&ans, command);
                }
            }
            AiProvider::OpenAI(key) => {
                let prompt = format!(
                    "El comando de terminal '{}' falló con código {}.\nSalida:\n{}\nExplica la causa brevemente y en la última línea responde EXACTAMENTE: 'SUGGESTION: <comando_corregido>'.",
                    command, exit_code, output
                );
                if let Ok(ans) = self.call_openai(key, &prompt).await {
                    return Self::parse_diagnostic_answer(&ans, command);
                }
            }
            AiProvider::OfflineHeuristics => {}
        }

        // Motor heurístico offline automático y preciso
        Self::offline_diagnostic(command, output, exit_code)
    }

    fn parse_diagnostic_answer(answer: &str, fallback_cmd: &str) -> AiDiagnostic {
        let lines: Vec<&str> = answer.lines().collect();
        let mut suggested_cmd = None;
        let mut explanation_lines = Vec::new();

        for line in lines {
            if let Some(stripped) = line.strip_prefix("SUGGESTION:") {
                let clean_cmd = stripped.trim().trim_matches('`').trim().to_string();
                if !clean_cmd.is_empty() {
                    suggested_cmd = Some(clean_cmd);
                }
            } else {
                explanation_lines.push(line);
            }
        }

        let explanation = explanation_lines.join(" ").trim().to_string();
        AiDiagnostic {
            explanation: if explanation.is_empty() {
                format!("Error ejecutando `{}`", fallback_cmd)
            } else {
                explanation
            },
            suggested_cmd,
        }
    }

    fn offline_diagnostic(command: &str, output: &str, exit_code: i32) -> AiDiagnostic {
        let lower_out = output.to_lowercase();
        let lower_cmd = command.to_lowercase();

        if lower_out.contains("permission denied") || lower_out.contains("permiso denegado") {
            let fix = format!("sudo {}", command);
            AiDiagnostic {
                explanation: "Permiso denegado. Se requieren privilegios de administrador.".to_string(),
                suggested_cmd: Some(fix),
            }
        } else if lower_out.contains("command not found") || lower_out.contains("no se encontró el comando") {
            let binary = command.split_whitespace().next().unwrap_or(command);
            AiDiagnostic {
                explanation: format!("El ejecutable '{}' no está instalado en el sistema.", binary),
                suggested_cmd: Some(format!("brew install {}", binary)),
            }
        } else if lower_out.contains("no such file or directory") || lower_out.contains("no existe el archivo") {
            AiDiagnostic {
                explanation: "La ruta o directorio objetivo no existe.".to_string(),
                suggested_cmd: Some("ls -la".to_string()),
            }
        } else if lower_cmd.starts_with("git push") && lower_out.contains("set-upstream") {
            let branch = lower_out
                .lines()
                .find(|l| l.contains("--set-upstream"))
                .and_then(|l| l.split("--set-upstream").nth(1))
                .map(|s| s.trim())
                .unwrap_or("origin main");
            AiDiagnostic {
                explanation: "La rama actual no tiene upstream configurado en el repositorio remoto.".to_string(),
                suggested_cmd: Some(format!("git push --set-upstream {}", branch)),
            }
        } else if lower_out.contains("already exists") || lower_out.contains("ya existe") {
            AiDiagnostic {
                explanation: "El recurso o archivo ya existe en la ruta de destino.".to_string(),
                suggested_cmd: None,
            }
        } else if lower_out.contains("address already in use") || lower_out.contains("port") {
            AiDiagnostic {
                explanation: "El puerto de red solicitado ya se encuentra ocupado por otro proceso.".to_string(),
                suggested_cmd: Some("lsof -iTCP -sTCP:LISTEN -P".to_string()),
            }
        } else {
            AiDiagnostic {
                explanation: format!("El comando falló con código {}. Revisa los parámetros.", exit_code),
                suggested_cmd: None,
            }
        }
    }

    /// Traducción de lenguaje natural a comando de terminal
    pub async fn natural_language_to_command(&self, prompt: &str) -> String {
        match &self.provider {
            AiProvider::Gemini(key) => {
                let system_prompt = format!(
                    "Convierte la siguiente petición de usuario a un comando ejecutable de terminal (zsh/bash para macOS). Responde EXCLUSIVAMENTE con el comando sin explicaciones, sin markdown y sin comillas:\n{}",
                    prompt
                );
                if let Ok(res) = self.call_gemini(key, &system_prompt).await {
                    let cleaned = res.trim().trim_matches('`').trim().to_string();
                    if !cleaned.is_empty() {
                        return cleaned;
                    }
                }
            }
            AiProvider::OpenAI(key) => {
                let system_prompt = format!(
                    "Convierte la siguiente petición de usuario a un comando ejecutable de terminal (zsh/bash para macOS). Responde EXCLUSIVAMENTE con el comando sin explicaciones, sin markdown y sin comillas:\n{}",
                    prompt
                );
                if let Ok(res) = self.call_openai(key, &system_prompt).await {
                    let cleaned = res.trim().trim_matches('`').trim().to_string();
                    if !cleaned.is_empty() {
                        return cleaned;
                    }
                }
            }
            AiProvider::OfflineHeuristics => {}
        }

        // Diccionario heurístico local offline
        Self::offline_translate(prompt)
    }

    fn offline_translate(prompt: &str) -> String {
        let p = prompt.to_lowercase();
        if (p.contains("buscar") || p.contains("encuentra")) && (p.contains("archivo") || p.contains("fichero")) {
            "find . -type f -name \"*query*\"".to_string()
        } else if p.contains("puerto") || p.contains("puertos") {
            "lsof -iTCP -sTCP:LISTEN -n -P".to_string()
        } else if p.contains("memoria") || p.contains("ram") {
            "top -l 1 -s 0 | grep PhysMem".to_string()
        } else if p.contains("espacio") || p.contains("disco") {
            "df -h".to_string()
        } else if p.contains("procesos") || p.contains("cpu") {
            "ps aux --sort=-%cpu | head -n 10".to_string()
        } else if p.contains("git") && (p.contains("commit") || p.contains("guardar")) {
            "git add . && git commit -m \"Auto-commit: cambios guardados\"".to_string()
        } else if p.contains("git") && p.contains("status") {
            "git status -sb".to_string()
        } else if p.contains("git") && p.contains("historial") {
            "git log --oneline -n 10".to_string()
        } else if p.contains("ip") || p.contains("red") {
            "ifconfig | grep \"inet \"".to_string()
        } else if p.contains("matar") || p.contains("kill") {
            "kill -9 <PID>".to_string()
        } else {
            format!("echo \"[IA]: {}\"", prompt)
        }
    }

    async fn call_gemini(&self, api_key: &str, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
            api_key
        );

        let req = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
        };

        let res = client
            .post(&url)
            .json(&req)
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await?;

        let body: GeminiResponse = res.json().await?;
        if let Some(candidates) = body.candidates {
            if let Some(candidate) = candidates.first() {
                if let Some(ref content) = candidate.content {
                    if let Some(ref parts) = content.parts {
                        if let Some(part) = parts.first() {
                            if let Some(ref text) = part.text {
                                return Ok(text.trim().to_string());
                            }
                        }
                    }
                }
            }
        }
        Err(anyhow::anyhow!("Respuesta vacía de Gemini"))
    }

    async fn call_openai(&self, api_key: &str, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let req_body = OpenAiChatRequest {
            model: "gpt-4o-mini".to_string(),
            messages: vec![OpenAiMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&req_body)
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await?;

        let res_data: OpenAiChatResponse = response.json().await?;
        if let Some(choice) = res_data.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow::anyhow!("Respuesta vacía de OpenAI"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_offline_diagnostics_permission() {
        let ai = AIAssistant {
            provider: AiProvider::OfflineHeuristics,
        };
        let diag = ai.diagnose_error("apt-get update", "Permission denied", 1).await;
        assert!(diag.explanation.contains("Permiso"));
        assert_eq!(diag.suggested_cmd, Some("sudo apt-get update".to_string()));
    }

    #[tokio::test]
    async fn test_offline_diagnostics_command_not_found() {
        let ai = AIAssistant {
            provider: AiProvider::OfflineHeuristics,
        };
        let diag = ai.diagnose_error("htop", "command not found: htop", 127).await;
        assert_eq!(diag.suggested_cmd, Some("brew install htop".to_string()));
    }

    #[tokio::test]
    async fn test_offline_translation() {
        let ai = AIAssistant {
            provider: AiProvider::OfflineHeuristics,
        };
        let cmd = ai.natural_language_to_command("ver puertos en uso").await;
        assert!(cmd.contains("lsof"));
    }
}
