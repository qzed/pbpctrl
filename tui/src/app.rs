use ratatui::widgets::ListState;
use crate::cli_client::{ConnectionState, BatteryState, SoftwareInfo, HardwareInfo, RuntimeInfo};

#[derive(Debug, Clone)]
pub struct SettingItem {
    pub key: String, // Command key for CLI, e.g., "anc"
    pub name: String, // Display name
    pub value: String, // Current value string
    pub options: Vec<String>, // Possible values to cycle through. Empty means read-only.
}

pub struct App {
    pub should_quit: bool,
    pub connection_state: ConnectionState,
    pub battery: BatteryState,
    pub software: SoftwareInfo,
    pub hardware: HardwareInfo,
    pub runtime: RuntimeInfo,
    pub selected_tab: usize,
    pub tabs: Vec<String>,
    
    pub settings_state: ListState,
    pub settings: Vec<SettingItem>,
}

impl App {
    pub fn new() -> Self {
        let mut settings_state = ListState::default();
        settings_state.select(Some(0));
        
        Self {
            should_quit: false,
            connection_state: ConnectionState::Disconnected,
            battery: BatteryState::default(),
            software: SoftwareInfo::default(),
            hardware: HardwareInfo::default(),
            runtime: RuntimeInfo::default(),
            selected_tab: 0,
            tabs: vec!["Status".to_string(), "Settings".to_string()],
            settings_state,
            settings: vec![
                // --- Audio & Noise Control ---
                SettingItem { 
                    key: "anc".to_string(), 
                    name: "ANC Mode".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["off".to_string(), "active".to_string(), "adaptive".to_string(), "aware".to_string()],
                },
                SettingItem { 
                    key: "volume-eq".to_string(), 
                    name: "Volume EQ".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["true".to_string(), "false".to_string()],
                },
                SettingItem { 
                    key: "mono".to_string(), 
                    name: "Mono Audio".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["true".to_string(), "false".to_string()],
                },
                SettingItem { 
                    key: "speech-detection".to_string(), 
                    name: "Conversation Detect".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["true".to_string(), "false".to_string()],
                },
                
                // --- Connectivity & Sensors ---
                SettingItem { 
                    key: "multipoint".to_string(), 
                    name: "Multipoint".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["true".to_string(), "false".to_string()],
                },
                SettingItem { 
                    key: "ohd".to_string(), 
                    name: "In-Ear Detection".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["true".to_string(), "false".to_string()],
                },
                
                // --- Controls ---
                SettingItem { 
                    key: "gestures".to_string(), 
                    name: "Touch Controls".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["true".to_string(), "false".to_string()],
                },
                
                // --- System & Diagnostics ---
                SettingItem { 
                    key: "volume-exposure-notifications".to_string(), 
                    name: "Volume Notifications".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["true".to_string(), "false".to_string()],
                },
                SettingItem { 
                    key: "diagnostics".to_string(), 
                    name: "Diagnostics".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["true".to_string(), "false".to_string()],
                },
                SettingItem { 
                    key: "oobe-mode".to_string(), 
                    name: "OOBE Mode".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["true".to_string(), "false".to_string()],
                },
                SettingItem { 
                    key: "oobe-is-finished".to_string(), 
                    name: "OOBE Finished".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec!["true".to_string(), "false".to_string()],
                },

                // --- Complex / Read-Only Settings ---
                SettingItem { 
                    key: "balance".to_string(), 
                    name: "Volume Balance".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec![], // Read-only
                },
                SettingItem { 
                    key: "eq".to_string(), 
                    name: "Equalizer".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec![], // Read-only
                },
                SettingItem { 
                    key: "gesture-control".to_string(), 
                    name: "Hold Gestures".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec![], // Read-only
                },
                SettingItem { 
                    key: "anc-gesture-loop".to_string(), 
                    name: "ANC Loop".to_string(), 
                    value: "Pending...".to_string(),
                    options: vec![], // Read-only
                },
            ],
        }
    }

    pub fn on_tick(&mut self) {
        // Handle tick if needed
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % self.tabs.len();
    }

    #[allow(dead_code)]
    pub fn previous_tab(&mut self) {
        if self.selected_tab > 0 {
            self.selected_tab -= 1;
        } else {
            self.selected_tab = self.tabs.len() - 1;
        }
    }

    pub fn next_setting(&mut self) {
        if let Some(selected) = self.settings_state.selected() {
            let next = (selected + 1) % self.settings.len();
            self.settings_state.select(Some(next));
        }
    }

    pub fn previous_setting(&mut self) {
        if let Some(selected) = self.settings_state.selected() {
            let next = if selected == 0 { self.settings.len() - 1 } else { selected - 1 };
            self.settings_state.select(Some(next));
        }
    }
    
    pub fn update_setting(&mut self, key: String, val: String) {
        for item in &mut self.settings {
            if item.key == key {
                // Normalize value (lowercase)
                item.value = val.to_lowercase();
            }
        }
    }
}
