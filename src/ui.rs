use crate::block::{BlockStatus, CommandBlock};
use crate::index::ProfileTracker;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block as RataBlock, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub struct AppState {
    pub input: String,
    pub blocks: Vec<CommandBlock>,
    pub ai_status: String,
    pub ai_mode: bool,
    pub tracker: ProfileTracker,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub pending_fix: Option<String>,
    pub scroll_offset: usize,
    pub ai_engine_name: String,
}

impl AppState {
    pub fn new(engine_name: &str) -> Self {
        Self {
            input: String::new(),
            blocks: Vec::new(),
            ai_status: "Sistema listo. Escribe un comando o presiona Ctrl+A para consultar IA.".to_string(),
            ai_mode: false,
            tracker: ProfileTracker::load_or_default(),
            history: Vec::new(),
            history_idx: None,
            pending_fix: None,
            scroll_offset: 0,
            ai_engine_name: engine_name.to_string(),
        }
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_idx = match self.history_idx {
            None => self.history.len().saturating_sub(1),
            Some(idx) => idx.saturating_sub(1),
        };
        self.history_idx = Some(next_idx);
        if let Some(cmd) = self.history.get(next_idx) {
            self.input = cmd.clone();
        }
    }

    pub fn history_down(&mut self) {
        if let Some(idx) = self.history_idx {
            if idx + 1 < self.history.len() {
                self.history_idx = Some(idx + 1);
                self.input = self.history[idx + 1].clone();
            } else {
                self.history_idx = None;
                self.input.clear();
            }
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(3);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(3);
    }

    pub fn apply_pending_fix(&mut self) -> bool {
        if let Some(fix) = self.pending_fix.take() {
            self.input = fix;
            self.ai_status = "⚡ Corrección automática aplicada a la entrada. Presiona Enter para ejecutar.".to_string();
            true
        } else {
            false
        }
    }
}

pub fn render_ui(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main content (Blocks + AI)
            Constraint::Length(3), // Input line
        ])
        .split(frame.area());

    render_header(frame, chunks[0], state);
    render_main_body(frame, chunks[1], state);
    render_input_bar(frame, chunks[2], state);
}

fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = if state.ai_mode {
        "⚡ WARP RUST CLI [ MODO ASISTENTE IA ACTIVO ]"
    } else {
        "⚡ WARP RUST CLI - Terminal Reactiva por Bloques"
    };

    let header_text = vec![Line::from(vec![
        Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
        Span::raw(" Ejecutar | "),
        Span::styled("[Ctrl+A]", Style::default().fg(Color::Magenta)),
        Span::raw(" IA | "),
        Span::styled("[Tab]", Style::default().fg(Color::LightGreen)),
        Span::raw(" Auto-Fix | "),
        Span::styled("[↑/↓]", Style::default().fg(Color::White)),
        Span::raw(" Historial | "),
        Span::styled("[PgUp/Dn]", Style::default().fg(Color::LightBlue)),
        Span::raw(" Scroll | "),
        Span::styled("[Esc]", Style::default().fg(Color::Red)),
        Span::raw(" Salir"),
    ])];

    let header_widget = Paragraph::new(header_text)
        .block(RataBlock::default().borders(Borders::ALL).title(" Control & Accesos Rápidos "))
        .wrap(Wrap { trim: true });

    frame.render_widget(header_widget, area);
}

fn render_main_body(frame: &mut Frame, area: Rect, state: &AppState) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(area);

    // Left: Blocks History
    let mut list_items = Vec::new();
    for block in &state.blocks {
        let (status_symbol, status_color) = match block.status {
            BlockStatus::Pending => ("⏳ PENDING", Color::Yellow),
            BlockStatus::Running => ("🔄 EJECUTANDO (Streaming)", Color::Cyan),
            BlockStatus::Success => ("✅ ÉXITO", Color::Green),
            BlockStatus::Error(_code) => ("❌ ERROR", Color::Red),
        };

        let header_line = Line::from(vec![
            Span::styled(format!("[{}] ", block.timestamp), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ", status_symbol), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            Span::styled(&block.command, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]);

        list_items.push(ListItem::new(header_line));

        // Output lines snippet
        if !block.output.is_empty() {
            let output_lines: Vec<&str> = block.output.lines().collect();
            // Show up to 10 latest lines for real-time visibility
            let take_count = 10;
            let skip_count = output_lines.len().saturating_sub(take_count);
            let output_preview = output_lines
                .into_iter()
                .skip(skip_count)
                .map(|l| format!("  │ {}", l))
                .collect::<Vec<_>>()
                .join("\n");

            list_items.push(ListItem::new(Line::from(Span::styled(
                output_preview,
                Style::default().fg(Color::Gray),
            ))));
        }

        // AI Diagnostic & Suggested Fix
        if let Some(ref suggestion) = block.ai_suggestion {
            list_items.push(ListItem::new(Line::from(Span::styled(
                format!("  └─ 💡 Causa: {}", suggestion),
                Style::default().fg(Color::LightYellow),
            ))));
        }

        if let Some(ref fix) = block.suggested_fix {
            list_items.push(ListItem::new(Line::from(vec![
                Span::styled("  └─ ⚡ Fix Automático: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(fix, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled("  [Presiona Tab para aplicar]", Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC)),
            ])));
        }

        list_items.push(ListItem::new(Line::from(Span::raw("")))); // Spacer
    }

    if list_items.is_empty() {
        list_items.push(ListItem::new(Line::from(Span::styled(
            "Terminal lista. Escribe un comando (ej: `cargo check`, `git status`, `ls -la`)",
            Style::default().fg(Color::DarkGray),
        ))));
    }

    let title = format!(" Historial de Bloques ({}) ", state.blocks.len());
    let blocks_list = List::new(list_items)
        .block(RataBlock::default().borders(Borders::ALL).title(title));

    frame.render_widget(blocks_list, body_chunks[0]);

    // Right: AI Assistant & Profile Tracker
    let mut ai_text = vec![
        Line::from(Span::styled("🤖 Inteligencia & Telemetría", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))),
        Line::from(Span::raw("-----------------------------------")),
        Line::from(vec![
            Span::styled("Motor: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&state.ai_engine_name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Estado Actual:", Style::default().fg(Color::Yellow))),
        Line::from(Span::styled(&state.ai_status, Style::default().fg(Color::LightCyan))),
        Line::from(Span::raw("")),
    ];

    if let Some(ref fix) = state.pending_fix {
        ai_text.push(Line::from(Span::styled("⚡ SUGERENCIA LISTA:", Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD))));
        ai_text.push(Line::from(Span::styled(format!("`{}`", fix), Style::default().fg(Color::White))));
        ai_text.push(Line::from(Span::styled("👉 Presiona [Tab] para autocompletar.", Style::default().fg(Color::Yellow))));
        ai_text.push(Line::from(Span::raw("")));
    }

    ai_text.push(Line::from(Span::styled(
        format!("📊 Nivel de Usuario (Total: {} cmds)", state.tracker.user.total_commands_typed),
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )));

    if state.tracker.user.skills.is_empty() {
        ai_text.push(Line::from(Span::styled("• Sin comandos registrados aún.", Style::default().fg(Color::DarkGray))));
    } else {
        for (tool, stats) in &state.tracker.user.skills {
            ai_text.push(Line::from(vec![
                Span::styled(format!("• {}: ", tool), Style::default().fg(Color::White)),
                Span::styled(format!("✅{} ", stats.manual_successes), Style::default().fg(Color::Green)),
                Span::styled(format!("❌{} ", stats.manual_failures), Style::default().fg(Color::Red)),
                Span::styled(format!("🤖{}", stats.ai_assisted), Style::default().fg(Color::Magenta)),
            ]));
        }
    }

    ai_text.push(Line::from(Span::raw("")));
    ai_text.push(Line::from(Span::styled("• PTY Streaming: ACTIVO 🟢", Style::default().fg(Color::Green))));
    ai_text.push(Line::from(Span::styled("• Auto-Persistencia: ACTIVO 💾", Style::default().fg(Color::Green))));
    ai_text.push(Line::from(Span::styled("• Auto-Fix [Tab]: ACTIVO ⚡", Style::default().fg(Color::Green))));

    let ai_panel = Paragraph::new(ai_text)
        .block(RataBlock::default().borders(Borders::ALL).title(" Perfil & Asistencia "))
        .wrap(Wrap { trim: true });

    frame.render_widget(ai_panel, body_chunks[1]);
}

fn render_input_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let (prompt, style) = if state.ai_mode {
        ("🤖 Preguntar / Traducir IA > ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
    } else {
        ("❯ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    };

    let mut spans = vec![
        Span::styled(prompt, style),
        Span::styled(&state.input, Style::default().fg(Color::White)),
    ];

    if state.input.is_empty() {
        if state.pending_fix.is_some() {
            spans.push(Span::styled(" (Presiona [Tab] para autocompletar solución sugerida)", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)));
        } else if state.ai_mode {
            spans.push(Span::styled(" (Escribe en lenguaje natural, ej: 'ver puertos en uso')", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)));
        }
    }

    let input_widget = Paragraph::new(Line::from(spans))
        .block(RataBlock::default().borders(Borders::ALL).title(" Entrada Interactiva "));

    frame.render_widget(input_widget, area);
}
