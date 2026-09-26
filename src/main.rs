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
    CommandChunk {
        block_id: usize,
        chunk: String,
    },
    CommandFinished {
        block_id: usize,
        command: String,
        exit_code: i32,
    },
    AiTranslationResult {
        original: String,
        translated: String,
    },
    AiDiagnosticResult {
        block_id: usize,
        explanation: String,
        suggested_cmd: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let ai = Arc::new(AIAssistant::new());
    let mut state = AppState::new(ai.provider_name());
    let mut block_counter = 1;

    // CANAL PRINCIPAL DE COMUNICACIÓN ASÍNCRONA
    let (tx, mut rx) = mpsc::channel(200);

    // HILO PRODUCTOR: Captura de Teclado y Ticks sin bloqueo
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

    // BUCLE PRINCIPAL DE INTERFAZ Y DESPACHO REACTIVO
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
                                state.ai_status = "Modo Asistente IA activo. Escribe lo que deseas hacer en español o inglés.".to_string();
                            } else {
                                state.ai_status = "Modo Terminal interactivo activo.".to_string();
                            }
                        }

                        // Auto-remediación: con Tab se carga la solución sugerida por la IA
                        (KeyCode::Tab, _) => {
                            state.apply_pending_fix();
                        }

                        // Navegación en historial de comandos
                        (KeyCode::Up, _) => {
                            state.history_up();
                        }
                        (KeyCode::Down, _) => {
                            state.history_down();
                        }

                        // Scroll en bloques de salida
                        (KeyCode::PageUp, _) => {
                            state.scroll_up();
                        }
                        (KeyCode::PageDown, _) => {
                            state.scroll_down();
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
                            state.history_idx = None;

                            if user_input.is_empty() {
                                continue;
                            }

                            if state.ai_mode {
                                // TRADUCCIÓN ASÍNCRONA DE LENGUAJE NATURAL
                                state.ai_status = format!("🤖 Consultando IA para '{}'...", user_input);
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
                                // REGISTRO Y EJECUCIÓN STREAMING EN PTY
                                state.history.push(user_input.clone());
                                state.pending_fix = None;

                                let mut block = CommandBlock::new(block_counter, user_input.clone());
                                block.status = crate::block::BlockStatus::Running;
                                state.blocks.push(block);

                                let current_block_id = block_counter;
                                block_counter += 1;

                                let tx_cmd = tx.clone();
                                let cmd = user_input.clone();

                                tokio::task::spawn_blocking(move || {
                                    let tx_chunk = tx_cmd.clone();
                                    let res = ShellRunner::run_command_streaming(&cmd, |chunk| {
                                        let _ = tx_chunk.blocking_send(AppEvent::CommandChunk {
                                            block_id: current_block_id,
                                            chunk,
                                        });
                                    });

                                    let exit_code = match res {
                                        Ok(code) => code,
                                        Err(e) => {
                                            let _ = tx_cmd.blocking_send(AppEvent::CommandChunk {
                                                block_id: current_block_id,
                                                chunk: format!("\nError ejecutando PTY: {}", e),
                                            });
                                            1
                                        }
                                    };

                                    let _ = tx_cmd.blocking_send(AppEvent::CommandFinished {
                                        block_id: current_block_id,
                                        command: cmd,
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

                // SALIDA EN TIEMPO REAL (Streaming)
                AppEvent::CommandChunk { block_id, chunk } => {
                    if let Some(block) = state.blocks.iter_mut().find(|b| b.id == block_id) {
                        block.append_output(&chunk);
                    }
                }

                // COMANDO FINALIZADO
                AppEvent::CommandFinished {
                    block_id,
                    command,
                    exit_code,
                } => {
                    let mut block_output = String::new();
                    if let Some(block) = state.blocks.iter_mut().find(|b| b.id == block_id) {
                        block.mark_finished(exit_code);
                        block_output = block.output.clone();
                    }

                    if exit_code == 0 {
                        state.tracker.record_manual_success(&command);
                        state.ai_status = format!("✅ `{}` finalizó exitosamente.", command);
                    } else {
                        state.tracker.record_manual_failure(&command);
                        state.ai_status = format!("❌ `{}` falló (código {}). Diagnosticando solución autónomamente...", command, exit_code);

                        // DIAGNÓSTICO AUTÓNOMO CON SUGERENCIA DE FIX
                        let tx_ai = tx.clone();
                        let ai_clone = Arc::clone(&ai);
                        let cmd_clone = command.clone();
                        tokio::spawn(async move {
                            let diag = ai_clone.diagnose_error(&cmd_clone, &block_output, exit_code).await;
                            let _ = tx_ai.send(AppEvent::AiDiagnosticResult {
                                block_id,
                                explanation: diag.explanation,
                                suggested_cmd: diag.suggested_cmd,
                            }).await;
                        });
                    }
                }

                // RESULTADO DE TRADUCCIÓN IA
                AppEvent::AiTranslationResult { original, translated } => {
                    state.tracker.record_ai_assistance(&original);
                    state.input = translated.clone();
                    state.ai_status = format!("🤖 IA sugiere: `{}`. Presiona Enter para ejecutar.", translated);
                }

                // RESULTADO DE DIAGNÓSTICO AUTÓNOMO
                AppEvent::AiDiagnosticResult {
                    block_id,
                    explanation,
                    suggested_cmd,
                } => {
                    if let Some(block) = state.blocks.iter_mut().find(|b| b.id == block_id) {
                        block.ai_suggestion = Some(explanation.clone());
                        block.suggested_fix = suggested_cmd.clone();
                    }

                    if let Some(fix) = suggested_cmd {
                        state.pending_fix = Some(fix.clone());
                        state.ai_status = format!("💡 Causa: {}. Presiona [Tab] para autocompletar corrección.", explanation);
                    } else {
                        state.ai_status = format!("💡 Diagnóstico: {}", explanation);
                    }
                }
            }
        }
    }

    // Restauración limpia de la terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    println!("¡Gracias por usar Warp Rust CLI!");
    Ok(())
}
