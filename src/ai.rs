use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AIAssistant {
    pub api_key: Option<String>,
    pub provider: String,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

impl AIAssistant {
    pub fn new() -> Self {
        let api_key = std::env::var("OPENAI_API_KEY").ok().or_else(|| std::env::var("GEMINI_API_KEY").ok());
        Self {
            api_key,
            provider: "default".to_string(),
        }
    }

    pub async fn explain_error(&self, command: &str, output: &str, exit_code: i32) -> String {
        if let Some(ref _key) = self.api_key {
            // Attempt API call if key is set
            if let Ok(res) = self.query_llm(&format!(
                "El comando '{}' falló con código {}. Salida de error: '{}'. Explica la causa en 2 frases cortas en español y da el comando corregido.",
                command, exit_code, output
            )).await {
                return res;
            }
        }

        // Fallback intelligent heuristics for common CLI errors
        if output.contains("Permission denied") || output.contains("permiso denegado") {
            format!("💡 **Sugerencia IA**: El comando requiere permisos elevados. Intenta anteceder `sudo`: `sudo {}`", command)
        } else if output.contains("command not found") || output.contains("no se encontró el comando") {
            let cmd_name = command.split_whitespace().next().unwrap_or(command);
            format!("💡 **Sugerencia IA**: El ejecutable `{}` no está instalado o no está en tu PATH. Instálalo con tu gestor de paquetes (brew, apt, cargo).", cmd_name)
        } else if output.contains("No such file or directory") {
            "💡 **Sugerencia IA**: La ruta especificada no existe. Verifica con `ls` o `pwd` el directorio actual.".to_string()
        } else if output.contains("git") {
            "💡 **Sugerencia IA**: Revisa el estado de tu repositorio Git con `git status` o `git remote -v`.".to_string()
        } else {
            format!("💡 **Sugerencia IA (Código {})**: Ocurrió un error en la ejecución. Puedes activar una API KEY para recibir análisis completo vía LLM.", exit_code)
        }
    }

    pub async fn natural_language_to_command(&self, prompt: &str) -> String {
        if let Some(ref _key) = self.api_key {
            if let Ok(res) = self.query_llm(&format!(
                "Convierte esta petición del usuario a un comando de terminal (zsh/bash). Responde ÚNICAMENTE con el comando sin formateo ni comillas: '{}'",
                prompt
            )).await {
                return res.trim().to_string();
            }
        }

        // Fallback local smart prompts
        let lower = prompt.to_lowercase();
        if lower.contains("buscar") && lower.contains("archivo") {
            "find . -name \"*pattern*\"".to_string()
        } else if lower.contains("git") && lower.contains("commit") {
            "git commit -m \"Mensaje de cambio\"".to_string()
        } else if lower.contains("procesos") || lower.contains("puerto") {
            "lsof -i :8080".to_string()
        } else {
            format!("echo \"[IA Simulación]: Comando para '{}'\"", prompt)
        }
    }

    async fn query_llm(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let api_key = self.api_key.as_ref().unwrap();

        let req_body = ChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&req_body)
            .send()
            .await?;

        let res_data: ChatResponse = response.json().await?;
        if let Some(choice) = res_data.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow::anyhow!("Respuesta vacía de la API de IA"))
        }
    }
}
