use crate::block::{BlockStatus, CommandBlock};
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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            blocks: Vec::new(),
            selected_block_idx: None,
            ai_status: "IA lista. Presiona Ctrl+A para modo Consulta IA o escribe un comando.".to_string(),
            ai_mode: false,
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
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
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

        // Output lines snippet
        if !block.output.is_empty() {
            let output_preview: String = block
                .output
                .lines()
                .take(4)
                .map(|l| format!("  │ {}", l))
                .collect::<Vec<_>>()
                .join("\n");

            list_items.push(ListItem::new(Line::from(Span::styled(
                output_preview,
                Style::default().fg(Color::Gray),
            ))));
        }

        // AI Suggestion snippet if present
        if let Some(ref suggestion) = block.ai_suggestion {
            list_items.push(ListItem::new(Line::from(Span::styled(
                format!("  └─ {}", suggestion),
                Style::default().fg(Color::Magenta),
            ))));
        }

        list_items.push(ListItem::new(Line::from(Span::raw("")))); // Spacer
    }

    let blocks_list = List::new(list_items)
        .block(RataBlock::default().borders(Borders::ALL).title(format!(" Bloques de Comandos ({}) ", state.blocks.len())));

    frame.render_widget(blocks_list, body_chunks[0]);

    // Right: AI Assistant & Web panel
    let ai_text = vec![
        Line::from(Span::styled("🤖 Asistente IA & Web", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))),
        Line::from(Span::raw("-------------------------")),
        Line::from(Span::styled(&state.ai_status, Style::default().fg(Color::LightCyan))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Índice de Comparación Warp:", Style::default().fg(Color::Yellow))),
        Line::from(Span::styled("• UI por Bloques: ACTIVO ✅", Style::default().fg(Color::Green))),
        Line::from(Span::styled("• PTY Shell Integration: ACTIVO ✅", Style::default().fg(Color::Green))),
        Line::from(Span::styled("• Diagnóstico IA: ACTIVO ✅", Style::default().fg(Color::Green))),
        Line::from(Span::styled("• Workflows: Próximamente ⏳", Style::default().fg(Color::DarkGray))),
    ];

    let ai_panel = Paragraph::new(ai_text)
        .block(RataBlock::default().borders(Borders::ALL).title(" Asistencia en Tiempo Real "))
        .wrap(Wrap { trim: true });

    frame.render_widget(ai_panel, body_chunks[1]);
}

fn render_input_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let (prompt, style) = if state.ai_mode {
        ("🤖 Preguntar a la IA > ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
    } else {
        ("❯ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    };

    let input_line = Line::from(vec![
        Span::styled(prompt, style),
        Span::styled(&state.input, Style::default().fg(Color::White)),
    ]);

    let input_widget = Paragraph::new(input_line)
        .block(RataBlock::default().borders(Borders::ALL).title(" Entrada "));

    frame.render_widget(input_widget, area);
}
