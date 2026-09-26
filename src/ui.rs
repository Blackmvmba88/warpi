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
    pub selected_block_idx: Option<usize>,
    pub ai_status: String,
    pub ai_mode: bool,
    pub tracker: ProfileTracker,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            blocks: Vec::new(),
            selected_block_idx: None,
            ai_status: "IA lista. Presiona Ctrl+A para modo Consulta IA o escribe un comando.".to_string(),
            ai_mode: false,
            tracker: ProfileTracker::new(),
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
        "⚡ WARP RUST CLI [ MODO ASISTENTE IA EN LÍNEA ]"
    } else {
        "⚡ WARP RUST CLI - Terminal Inteligente por Bloques"
    };

    let header_text = vec![
        Line::from(vec![
            Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
            Span::raw(" Ejecutar | "),
            Span::styled("[Ctrl+A]", Style::default().fg(Color::Magenta)),
            Span::raw(" Consultar IA | "),
            Span::styled("[Esc]", Style::default().fg(Color::Red)),
            Span::raw(" Salir"),
        ]),
    ];

    let header_widget = Paragraph::new(header_text)
        .block(RataBlock::default().borders(Borders::ALL).title(" Estado "))
        .wrap(Wrap { trim: true });

    frame.render_widget(header_widget, area);
}

fn render_main_body(frame: &mut Frame, area: Rect, state: &AppState) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Left: Blocks History
    let mut list_items = Vec::new();
    for block in &state.blocks {
        let (status_symbol, status_color) = match block.status {
            BlockStatus::Pending => ("⏳", Color::Yellow),
            BlockStatus::Running => ("🔄", Color::Blue),
            BlockStatus::Success => ("✅", Color::Green),
            BlockStatus::Error(_code) => ("❌", Color::Red),
        };

        let header_line = Line::from(vec![
            Span::styled(format!("[{}] ", block.timestamp), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ", status_symbol), Style::default().fg(status_color)),
            Span::styled(&block.command, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]);

        list_items.push(ListItem::new(header_line));

        if !block.output.is_empty() {
            let output_preview: String = block
                .output
                .lines()
                .take(6)
                .map(|l| format!("  │ {}", l))
                .collect::<Vec<_>>()
                .join("\n");

            list_items.push(ListItem::new(Line::from(Span::styled(
                output_preview,
                Style::default().fg(Color::Gray),
            ))));
        }

        if let Some(ref suggestion) = block.ai_suggestion {
            list_items.push(ListItem::new(Line::from(Span::styled(
                format!("  └─ 💡 Sugerencia IA: {}", suggestion),
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ))));
        }

        list_items.push(ListItem::new(Line::from(Span::raw("")))); // Spacer
    }

    let blocks_list = List::new(list_items)
        .block(RataBlock::default().borders(Borders::ALL).title(format!(" Bloques de Comandos ({}) ", state.blocks.len())));

    frame.render_widget(blocks_list, body_chunks[0]);

    // Right: AI Assistant & Profile Tracker
    let mut ai_text = vec![
        Line::from(Span::styled("🤖 Asistente IA & Skills", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))),
        Line::from(Span::raw("-------------------------")),
        Line::from(Span::styled(&state.ai_status, Style::default().fg(Color::LightCyan))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(format!("📊 Índice de Nivel (Exp Total: {})", state.tracker.user.total_commands_typed), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
    ];

    for (tool, stats) in &state.tracker.user.skills {
        ai_text.push(Line::from(vec![
            Span::styled(format!("• {}: ", tool), Style::default().fg(Color::White)),
            Span::styled(format!("✅{} ", stats.manual_successes), Style::default().fg(Color::Green)),
            Span::styled(format!("❌{} ", stats.manual_failures), Style::default().fg(Color::Red)),
            Span::styled(format!("🤖{}", stats.ai_assisted), Style::default().fg(Color::Magenta)),
        ]));
    }

    ai_text.push(Line::from(Span::raw("")));
    ai_text.push(Line::from(Span::styled("Warp Engine: Async mpsc ACTIVO 🚀", Style::default().fg(Color::Green))));

    let ai_panel = Paragraph::new(ai_text)
        .block(RataBlock::default().borders(Borders::ALL).title(" Perfil & Asistencia "))
        .wrap(Wrap { trim: true });

    frame.render_widget(ai_panel, body_chunks[1]);
}

fn render_input_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let (prompt, style) = if state.ai_mode {
        ("🤖 Traducción IA > ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
    } else {
        ("❯ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    };

    let input_line = Line::from(vec![
        Span::styled(prompt, style),
        Span::styled(&state.input, Style::default().fg(Color::White)),
    ]);

    let input_widget = Paragraph::new(input_line)
        .block(RataBlock::default().borders(Borders::ALL).title(" Entrada (Escribe un comando) "));

    frame.render_widget(input_widget, area);
}
