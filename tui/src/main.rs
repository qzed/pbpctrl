use std::{io, time::Duration};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

mod app;
mod cli_client;
mod ui;

use app::App;
use cli_client::{ClientCommand, ClientEvent};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Create channels
    let (tx_event, mut rx_event) = mpsc::unbounded_channel();
    let (tx_cmd, rx_cmd) = mpsc::unbounded_channel();

    // Spawn CLI client
    tokio::spawn(cli_client::run_loop(tx_event, rx_cmd));

    // Initial check
    tx_cmd.send(ClientCommand::CheckConnection).ok();

    let _res = run_app(&mut terminal, &mut app, tx_cmd, &mut rx_event).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // if let Err(err) = res {
    //     println!("{:?}", err);
    // }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tx_cmd: mpsc::UnboundedSender<ClientCommand>,
    rx_event: &mut mpsc::UnboundedReceiver<ClientEvent>,
) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let poll_rate = Duration::from_secs(2); // Poll status/settings every 2s
    let mut last_tick = std::time::Instant::now();
    let mut last_poll = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Tab => {
                        app.next_tab();
                    }
                    KeyCode::Char('c') => {
                        tx_cmd.send(ClientCommand::CheckConnection)?;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                         if app.selected_tab == 1 {
                             app.next_setting();
                         }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.selected_tab == 1 {
                            app.previous_setting();
                        }
                    }
                    KeyCode::Left => {
                        if app.selected_tab == 1 {
                            handle_numeric_change(app, &tx_cmd, -1.0);
                        }
                    }
                    KeyCode::Right => {
                        if app.selected_tab == 1 {
                            handle_numeric_change(app, &tx_cmd, 1.0);
                        }
                    }
                    KeyCode::Enter => {
                        if app.selected_tab == 1 {
                            handle_setting_change(app, &tx_cmd);
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Handle events
        while let Ok(event) = rx_event.try_recv() {
            match event {
                ClientEvent::ConnectionState(state) => {
                    app.connection_state = state.clone();
                    if matches!(state, cli_client::ConnectionState::Connected) {
                        // Refresh all
                        // We use GetRuntime instead of GetBattery as it provides more info
                        tx_cmd.send(ClientCommand::GetRuntime)?;
                        tx_cmd.send(ClientCommand::GetSoftware)?;
                        tx_cmd.send(ClientCommand::GetHardware)?;
                        
                        // Fetch settings
                        // We need to avoid duplicate calls for same keys (like eq)
                        let mut fetched_keys = std::collections::HashSet::new();
                        for item in &app.settings {
                             if !fetched_keys.contains(&item.key) {
                                 tx_cmd.send(ClientCommand::GetSetting(item.key.clone()))?;
                                 fetched_keys.insert(item.key.clone());
                             }
                        }
                        // Explicitly fetch gesture-control as it is no longer in settings list
                        tx_cmd.send(ClientCommand::GetSetting("gesture-control".to_string()))?;
                    }
                }
                ClientEvent::Software(info) => {
                    app.software = info;
                }
                ClientEvent::Hardware(info) => {
                    app.hardware = info;
                }
                ClientEvent::Runtime(info) => {
                    app.runtime = info.clone();
                    // Runtime also contains battery info, let's update that too
                    app.battery = info.battery;
                }
                ClientEvent::Setting(key, val) => {
                    // Handle special parsing
                    if key == "gesture-control" {
                        app.gesture_control = val;
                    } else if key == "eq" {
                        // val format: [0.00, 0.50, -1.00, 0.00, 0.00]
                        let trimmed = val.trim_matches(|c| c == '[' || c == ']');
                        let parts: Vec<&str> = trimmed.split(',').map(|s| s.trim()).collect();
                        
                        if parts.len() == 5 {
                             for (i, part) in parts.iter().enumerate() {
                                 if let Ok(_) = part.parse::<f32>() {
                                     // Find item with key "eq" and index i
                                     if let Some(item) = app.settings.iter_mut().find(|it| it.key == "eq" && it.index == Some(i)) {
                                         item.value = part.to_string();
                                     }
                                 }
                             }
                        }
                    } else if key == "balance" {
                         // val format: left: 100%, right: 100%
                         // We just store the string value, logic to parse is in handle_numeric_change
                         app.update_setting(key, val);
                    } else {
                        app.update_setting(key, val);
                    }
                }
                ClientEvent::Error(msg) => {
                    app.set_error(msg);
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = std::time::Instant::now();
        }

        if last_poll.elapsed() >= poll_rate {
             if matches!(app.connection_state, cli_client::ConnectionState::Connected) {
                 tx_cmd.send(ClientCommand::GetRuntime)?;
                 // Also update status-critical settings
                 tx_cmd.send(ClientCommand::GetSetting("anc".to_string()))?;
                 tx_cmd.send(ClientCommand::GetSetting("multipoint".to_string()))?;
                 tx_cmd.send(ClientCommand::GetSetting("ohd".to_string()))?;
                 tx_cmd.send(ClientCommand::GetSetting("gesture-control".to_string()))?;
             }
             last_poll = std::time::Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_numeric_change(app: &mut App, tx_cmd: &mpsc::UnboundedSender<ClientCommand>, direction: f32) {
    if let Some(idx) = app.settings_state.selected() {
        let item = &mut app.settings[idx];
        
        if let Some((min, max, step)) = item.range {
            let change = direction * step;
            
            if item.key == "balance" {
                // Parse current balance
                // Format: "left: {l}%, right: {r}%"
                let current_val = if item.value.contains("left:") {
                    let parts: Vec<&str> = item.value.split(',').collect();
                    let mut l = 100;
                    let mut r = 100;
                    
                    for part in parts {
                        if let Some(v) = part.split(':').nth(1) {
                            let n = v.trim().trim_end_matches('%').parse::<i32>().unwrap_or(100);
                            if part.contains("left") { l = n; }
                            if part.contains("right") { r = n; }
                        }
                    }
                    
                    // Convert back to -100..100 range
                    // Logic from libmaestro:
                    // if val > 0 (right bias): L reduced, R=100. val = ((100-L) << 1) | 1 ?? No.
                    // The CLI `set` command takes -100 to 100.
                    // If val=0, L=100, R=100.
                    // If val=50, L=50, R=100.
                    // If val=-50, L=100, R=50.
                    
                    if r == 100 {
                        // Left might be reduced (right bias) -> val > 0
                        // val = 100 - l
                        100 - l
                    } else {
                        // Right is reduced (left bias) -> val < 0
                        // val = -(100 - r) = r - 100
                        r - 100
                    }
                } else {
                    0
                };
                
                let new_val = (current_val as f32 + change).clamp(min, max) as i32;
                
                // Optimistic update string (approximate)
                let l = (100 - new_val).min(100);
                let r = (100 + new_val).min(100);
                item.value = format!("left: {}%, right: {}%", l, r);
                
                let _ = tx_cmd.send(ClientCommand::SetSetting(item.key.clone(), new_val.to_string()));
                
            } else if item.key == "eq" {
                // Parse current float value
                let current_val = item.value.parse::<f32>().unwrap_or(0.0);
                let new_val = (current_val + change).clamp(min, max);
                
                // Update local value string immediately
                item.value = format!("{:.2}", new_val);
                
                // We need to gather ALL 5 EQ values to send the command
                // Because we modified `app.settings` in place (via item), we need to iterate carefully
                // But `item` is a mutable borrow of `app.settings`, so we can't iterate `app.settings` again here.
                // Workaround: We updated `item.value`. Now we need to construct the full command.
                
                // We need to drop the mutable borrow `item` to read `app.settings`.
                // But we are inside `if let Some ... item = &mut app.settings` block.
                // We can just calculate the string to send, and return it?
            }
        }
    }
    
    // Split logic to avoid borrow checker issues for EQ
    if let Some(idx) = app.settings_state.selected() {
        let key = app.settings[idx].key.clone();
        if key == "eq" {
             // Gather all eq values
             let mut eq_values = vec![0.0f32; 5];
             for it in &app.settings {
                 if it.key == "eq" {
                     if let Some(i) = it.index {
                         if i < 5 {
                             eq_values[i] = it.value.parse::<f32>().unwrap_or(0.0);
                         }
                     }
                 }
             }
             
             // Construct command string: "v1 v2 v3 v4 v5"
             let args = eq_values.iter().map(|v| format!("{:.2}", v)).collect::<Vec<_>>().join(" ");
             let _ = tx_cmd.send(ClientCommand::SetSetting("eq".to_string(), args));
        }
    }
}

fn handle_setting_change(app: &App, tx_cmd: &mpsc::UnboundedSender<ClientCommand>) {
    if let Some(idx) = app.settings_state.selected() {
        let item = &app.settings[idx];
        
        if item.options.is_empty() {
            return; // Read-only setting
        }
        
        // Cycle to next option
        let current_val = item.value.to_lowercase();
        // Find current index in options
        let current_opt_idx = item.options.iter().position(|o| o.to_lowercase() == current_val);
        
        let next_val = if let Some(i) = current_opt_idx {
            item.options[(i + 1) % item.options.len()].clone()
        } else if !item.options.is_empty() {
            item.options[0].clone()
        } else {
            return;
        };
        
        // Send set command
        let _ = tx_cmd.send(ClientCommand::SetSetting(item.key.clone(), next_val));
    }
}
