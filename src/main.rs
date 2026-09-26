mod ai;
mod block;
mod index;
mod pty;
mod ui;

use ai::AIAssistant;
use anyhow::Result;
use block::CommandBlock;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use pty::ShellRunner;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use ui::{render_ui, AppState};

enum AppEvent {
    Input(Event),
    Tick,
    CommandResult {
        block_id: usize,
        command: String,
        output: String,
        exit_code: i32,
    },
    AiTranslationResult {
        original: String,
        translated: String,
    },
    AiExplanationResult {
        block_id: usize,
        explanation: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new();
    let ai = Arc::new(AIAssistant::new());
    let mut block_counter = 1;

    // EL CORAZÓN ASÍNCRONO: mpsc channel
    let (tx, mut rx) = mpsc::channel(100);

    // HILO 1: Captura de Teclado Continua (No bloqueante)
    let tx_input = tx.clone();
    tokio::spawn(async move {
        loop {
            if event::poll(Duration::from_millis(16)).unwrap_or(false) {
                if let Ok(event) = event::read() {
                    let _ = tx_input.send(AppEvent::Input(event)).await;
                }
            } else {
                let _ = tx_input.send(AppEvent::Tick).await;
            }
        }
    });

    // BUCLE PRINCIPAL (UI y Recepción de Eventos)
    loop {
        terminal.draw(|f| render_ui(f, &state))?;

        if let Some(app_event) = rx.recv().await {
            match app_event {
                AppEvent::Input(Event::Key(key)) => {
                    match (key.code, key.modifiers) {
                        (KeyCode::Esc, _) => break,
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,

                        (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                            state.ai_mode = !state.ai_mode;
                            if state.ai_mode {
                                state.ai_status = "Modo IA activo. Ingresa lenguaje natural.".to_string();
                            } else {
                                state.ai_status = "Modo Terminal activo.".to_string();
                            }
                        }
                        (KeyCode::Backspace, _) => {
                            state.input.pop();
                        }
                        (KeyCode::Char(c), _) => {
                            state.input.push(c);
                        }
                        (KeyCode::Enter, _) => {
                            let user_input = state.input.trim().to_string();
                            state.input.clear();
                            if user_input.is_empty() {
                                continue;
                            }

                            if state.ai_mode {
                                // ASYNC IA TRANSLATION
                                state.ai_status = format!("Consultando IA para: '{}'...", user_input);
                                let tx_ai = tx.clone();
                                let ai_clone = Arc::clone(&ai);
                                tokio::spawn(async move {
                                    let translated = ai_clone.natural_language_to_command(&user_input).await;
                                    let _ = tx_ai.send(AppEvent::AiTranslationResult {
                                        original: user_input,
                                        translated,
                                    }).await;
                                });
                                state.ai_mode = false;
                            } else {
                                // ASYNC PTY EXECUTION
                                let mut block = CommandBlock::new(block_counter, user_input.clone());
                                block.status = crate::block::BlockStatus::Running;
                                state.blocks.push(block);

                                let current_block_id = block_counter;
                                block_counter += 1;

                                let tx_cmd = tx.clone();
                                let cmd = user_input.clone();
                                tokio::task::spawn_blocking(move || {
                                    let (output, exit_code) = match ShellRunner::run_command(&cmd) {
                                        Ok((o, c)) => (o, c),
                                        Err(e) => (format!("Error interno: {}", e), 1),
                                    };
                                    let _ = tx_cmd.blocking_send(AppEvent::CommandResult {
                                        block_id: current_block_id,
                                        command: cmd,
                                        output,
                                        exit_code,
                                    });
                                });
                            }
                        }
                        _ => {}
                    }
                }
                AppEvent::Input(_) => {}
                AppEvent::Tick => {}
                AppEvent::CommandResult {
                    block_id,
                    command,
                    output,
                    exit_code,
                } => {
                    // Update Block
                    if let Some(block) = state.blocks.iter_mut().find(|b| b.id == block_id) {
                        block.append_output(&output);
                        block.mark_finished(exit_code);
                    }

                    // Update Tracker
                    if exit_code == 0 {
                        state.tracker.record_manual_success(&command);
                        state.ai_status = format!("Comando `{}` ejecutado con éxito.", command);
                    } else {
                        state.tracker.record_manual_failure(&command);
                        state.ai_status = format!("Comando `{}` falló. Consultando IA...", command);

                        // ASYNC ERROR EXPLANATION
                        let tx_ai = tx.clone();
                        let ai_clone = Arc::clone(&ai);
                        let cmd_clone = command.clone();
                        let out_clone = output.clone();
                        tokio::spawn(async move {
                            let explanation = ai_clone.explain_error(&cmd_clone, &out_clone, exit_code).await;
                            let _ = tx_ai.send(AppEvent::AiExplanationResult {
                                block_id,
                                explanation,
                            }).await;
                        });
                    }
                }
                AppEvent::AiTranslationResult { original, translated } => {
                    state.tracker.record_ai_assistance(&original);
                    state.input = translated.clone();
                    state.ai_status = format!("IA sugiere: `{}`. Presiona Enter para ejecutar.", translated);
                }
                AppEvent::AiExplanationResult { block_id, explanation } => {
                    if let Some(block) = state.blocks.iter_mut().find(|b| b.id == block_id) {
                        block.ai_suggestion = Some(explanation.clone());
                    }
                    state.ai_status = "💡 Sugerencia IA recibida. Revisa el bloque.".to_string();
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    println!("¡Gracias por usar Warp Rust CLI V2 (Async Edition)!");
    Ok(())
}
