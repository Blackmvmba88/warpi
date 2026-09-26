mod ai;
mod block;
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
use std::time::Duration;
use ui::{render_ui, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal raw mode
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new();
    let ai = AIAssistant::new();
    let mut block_counter = 1;

    loop {
        terminal.draw(|f| render_ui(f, &state))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => break,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,

                    // Toggle AI mode with Ctrl+A
                    (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                        state.ai_mode = !state.ai_mode;
                        if state.ai_mode {
                            state.ai_status = "Modo Asistente IA activo. Describe lo que quieres hacer en lenguaje natural.".to_string();
                        } else {
                            state.ai_status = "Modo Terminal activo. Ejecutando comandos directamente en el PTY.".to_string();
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
                            // AI translation mode
                            state.ai_status = format!("Consultando IA para: '{}'...", user_input);
                            let suggested_cmd = ai.natural_language_to_command(&user_input).await;
                            state.ai_status = format!("IA sugiere ejecutar: `{}`. Desactiva Ctrl+A o presiona Enter para ejecutar.", suggested_cmd);
                            state.input = suggested_cmd;
                            state.ai_mode = false;
                        } else {
                            // Execute command via PTY
                            let mut block = CommandBlock::new(block_counter, user_input.clone());
                            block_counter += 1;

                            match ShellRunner::run_command(&user_input) {
                                Ok((output, exit_code)) => {
                                    block.append_output(&output);
                                    block.mark_finished(exit_code);

                                    // If command failed, request AI explanation
                                    if exit_code != 0 {
                                        let suggestion = ai.explain_error(&user_input, &output, exit_code).await;
                                        block.ai_suggestion = Some(suggestion.clone());
                                        state.ai_status = suggestion;
                                    } else {
                                        state.ai_status = format!("Comando `{}` ejecutado con éxito (código 0).", user_input);
                                    }
                                }
                                Err(e) => {
                                    block.append_output(&format!("Error al spawnear comando: {}", e));
                                    block.mark_finished(1);
                                }
                            }

                            state.blocks.push(block);
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    println!("¡Gracias por usar Warp Rust CLI!");
    Ok(())
}
