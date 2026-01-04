use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Tabs, Row, Table},
    Frame,
};
use crate::app::App;
use crate::maestro_client::ConnectionState;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Status/Help
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    
    match app.selected_tab {
        0 => draw_status(f, app, chunks[1]),
        1 => draw_settings(f, app, chunks[1]),
        _ => {},
    }
    
    draw_help(f, app, chunks[2]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app
        .tabs
        .iter()
        .map(|t| Line::from(Span::styled(t, Style::default().fg(Color::Green))))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Pixel Buds Pro Control"))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .select(app.selected_tab);
    
    f.render_widget(tabs, area);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Connection Header
            Constraint::Min(0),    // Main Content
        ])
        .split(area);

    // Connection Status
    let (conn_text, conn_style) = match app.connection_state {
        ConnectionState::Disconnected => (
            "Disconnected (Press 'c' to refresh/connect)", 
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        ),
        ConnectionState::Connected => (
            "Connected", 
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        ),
    };
    
    let p = Paragraph::new(conn_text)
        .style(conn_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(p, main_layout[0]);

    if app.connection_state == ConnectionState::Disconnected {
        return; 
    }

    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55), // Device & Placement Info
            Constraint::Percentage(45), // Battery Info
        ])
        .split(main_layout[1]);

    // Left Column: Device Info + Placement
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
             Constraint::Percentage(50),
             Constraint::Percentage(50),
        ])
        .split(content_layout[0]);

    // Device Info Table
    let rows = vec![
        Row::new(vec!["Component", "Firmware", "Serial Number"]).style(Style::default().fg(Color::Yellow)),
        Row::new(vec!["Case", &app.software.case_version, &app.hardware.case_serial]),
        Row::new(vec!["Left Bud", &app.software.left_version, &app.hardware.left_serial]),
        Row::new(vec!["Right Bud", &app.software.right_version, &app.hardware.right_serial]),
    ];

    let table = Table::new(rows, [
        Constraint::Percentage(30), 
        Constraint::Percentage(35), 
        Constraint::Percentage(35)
    ])
    .block(Block::default().title(" Device Information ").borders(Borders::ALL))
    .column_spacing(1);
    
    f.render_widget(table, left_chunks[0]);
    
    // Placement & Connection Table
    let place_rows = vec![
        Row::new(vec!["Left Placement", &app.runtime.placement_left]),
        Row::new(vec!["Right Placement", &app.runtime.placement_right]),
        Row::new(vec!["Local Peer", &app.runtime.peer_local]),
        Row::new(vec!["Remote Peer", &app.runtime.peer_remote]),
        Row::new(vec!["ANC Status", get_setting_val(app, "anc")]),
        Row::new(vec!["Multipoint", get_setting_val(app, "multipoint")]),
        Row::new(vec!["In-Ear Detection", get_setting_val(app, "ohd")]),
        Row::new(vec!["Hold Gestures", &app.gesture_control]),
    ];
    let place_table = Table::new(place_rows, [
        Constraint::Percentage(40),
        Constraint::Percentage(60),
    ])
    .block(Block::default().title(" Status & Connection ").borders(Borders::ALL))
    .column_spacing(1);
    f.render_widget(place_table, left_chunks[1]);


    // Battery Info - Custom Layout
    // We only show Case if known
    let mut constraints = vec![];
    let show_case = app.battery.case_level.is_some();
    
    // Calculate space needed for each battery item (Text + Gauge)
    // We give them 3 lines each: 1 for Text, 1 for Gauge, 1 padding/border
    // Actually let's do:
    // Case:
    //   Level: 100% (Charging)
    //   [||||||||||]
    
    if show_case {
        constraints.push(Constraint::Length(4));
    }
    constraints.push(Constraint::Length(4)); // Left
    constraints.push(Constraint::Length(4)); // Right
    constraints.push(Constraint::Min(0)); // Spacer

    let bat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .margin(1)
        .split(content_layout[1]);

    let bat_block = Block::default().title(" Battery Status ").borders(Borders::ALL);
    f.render_widget(bat_block, content_layout[1]);
    
    let mut current_chunk_idx = 0;
    
    if show_case {
        draw_battery_item(f, bat_chunks[current_chunk_idx], "Case", app.battery.case_level, &app.battery.case_status);
        current_chunk_idx += 1;
    }
    
    draw_battery_item(f, bat_chunks[current_chunk_idx], "Left Bud", app.battery.left_level, &app.battery.left_status);
    current_chunk_idx += 1;
    
    draw_battery_item(f, bat_chunks[current_chunk_idx], "Right Bud", app.battery.right_level, &app.battery.right_status);
}

fn get_setting_val<'a>(app: &'a App, key: &str) -> &'a str {
    app.settings.iter().find(|s| s.key == key).map(|s| s.value.as_str()).unwrap_or("Unknown")
}

fn draw_battery_item(f: &mut Frame, area: Rect, name: &str, level: Option<u8>, status: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let charging = status.contains("charging") && !status.contains("not charging");
    let level_val = level.unwrap_or(0);
    let color = if charging { Color::Green } else if level_val < 20 { Color::Red } else { Color::Cyan };
    
    let status_text = if let Some(l) = level {
        format!("{}  {}% ({})", name, l, status)
    } else {
        format!("{}  Unknown ({})", name, status)
    };

    let text = Paragraph::new(status_text).style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(text, chunks[0]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color))
        .ratio(level_val as f64 / 100.0)
        .label(""); // No label on gauge itself as requested
        
    f.render_widget(gauge, chunks[1]);
}

fn draw_settings(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app.settings.iter().map(|i| {
        let val_str = i.value.clone();
        
        let content = Line::from(vec![
            Span::styled(format!("{:<40}", i.name), Style::default().fg(Color::White)),
            Span::styled(val_str, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]);
        
        ListItem::new(content)
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Settings "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
        
    f.render_stateful_widget(list, area, &mut app.settings_state);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    if let Some(err) = &app.last_error {
        let p = Paragraph::new(format!("Error: {}", err))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        f.render_widget(p, area);
    } else {
        let text = "q: Quit | Tab: Switch Tab | c: Check Connection/Refresh | Enter: Toggle/Change Setting";
        let p = Paragraph::new(text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(p, area);
    }
}
