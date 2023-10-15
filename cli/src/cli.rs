use bluer::Address;
use clap::{Parser, Subcommand, ValueEnum};

use maestro::service::settings;


/// Control Google Pixel Buds Pro from the command line
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Device to use (search for compatible device if unspecified)
    #[arg(short, long, global=true)]
    pub device: Option<Address>,

    #[command(subcommand)]
    pub command: Command
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show device information
    Show {
        #[command(subcommand)]
        command: ShowCommand
    },

    /// Read settings value
    Get {
        #[command(subcommand)]
        setting: GetSetting
    },

    /// Write settings value
    Set {
        #[command(subcommand)]
        setting: SetSetting
    },
}

#[derive(Debug, Subcommand)]
pub enum ShowCommand {
    /// Show software information.
    Software,

    /// Show hardware information.
    Hardware,

    /// Show runtime information.
    Runtime,

    /// Show battery status.
    Battery,
}

#[derive(Debug, Subcommand)]
pub enum GetSetting {
    /// Get automatic over-the-air update status
    AutoOta,

    /// Get on-head-detection state (enabled/disabled)
    Ohd,

    /// Get the flag indicating whether the out-of-box experience phase is
    /// finished
    OobeIsFinished,

    /// Get gesture state (enabled/disabled)
    Gestures,

    /// Get diagnostics state (enabled/disabled)
    Diagnostics,

    /// Get out-of-box-experience mode state (enabled/disabled)
    OobeMode,

    /// Get hold-gesture action
    GestureControl,

    /// Get multipoint audio state (enabled/disabled)
    Multipoint,

    /// Get adaptive noise-cancelling gesture loop
    AncGestureLoop,

    /// Get adaptive noise-cancelling state
    Anc,

    /// Get volume-dependent EQ state (enabled/disabled)
    VolumeEq,

    /// Get 5-band EQ
    Eq,

    /// Get volume balance
    Balance,

    /// Get mono output state
    Mono,

    /// Get volume exposure notifications state (enabled/disabled)
    VolumeExposureNotifications,

    /// Get automatic transparency mode state (enabled/disabled)
    SpeechDetection,
}

#[derive(Debug, Subcommand)]
pub enum SetSetting {
    /// Enable/disable automatic over-the-air updates
    ///
    /// Note: Updates are initiated by the Google Buds app on your phone. This
    /// flag controls whether updates can be done automatically when the device
    /// is not in use.
    AutoOta {
        /// Whether to enable or disable automatic over-the-air (OTA) updates
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Enable/disable on-head detection
    Ohd {
        /// Whether to enable or disable on-head detection
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Set the flag indicating whether the out-of-box experience phase is
    /// finished
    ///
    /// Note: You normally do not want to change this flag. It is used to
    /// indicate whether the out-of-box experience (OOBE) phase has been
    /// concluded, i.e., the setup wizard has been run and the device has been
    /// set up.
    OobeIsFinished {
        /// Whether the OOBE setup has been finished
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Enable/disable gestures
    Gestures {
        /// Whether to enable or disable gestures
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Enable/disable diagnostics
    ///
    /// Note: This will also cause the Google Buds app on your phone to send
    /// diagnostics data to Google.
    Diagnostics {
        /// Whether to enable or disable diagnostics
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Enable/disable out-of-box-experience mode
    ///
    /// Note: You normally do not want to enable this mode. It is used to
    /// intercept and block touch gestures during the setup wizard.
    OobeMode {
        /// Whether to enable or disable the out-of-box experience mode
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Set hold-gesture action
    GestureControl {
        /// Left gesture action
        #[arg(value_enum)]
        left: HoldGestureAction,

        /// Right gesture action
        #[arg(value_enum)]
        right: HoldGestureAction,
    },

    /// Enable/disable multipoint audio
    Multipoint {
        /// Whether to enable or disable multipoint audio
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Set adaptive noise-cancelling gesture loop
    AncGestureLoop {
        /// Enable 'off' mode in loop
        #[arg(action=clap::ArgAction::Set)]
        off: bool,

        /// Enable 'active' mode in loop
        #[arg(action=clap::ArgAction::Set)]
        active: bool,

        /// Enable 'aware' mode in loop
        #[arg(action=clap::ArgAction::Set)]
        aware: bool,
    },

    /// Set adaptive noise-cancelling state
    Anc {
        /// New ANC state or action to change state
        #[arg(value_enum)]
        value: AncState,
    },

    /// Enable/disable volume-dependent EQ
    VolumeEq {
        /// Whether to enable or disable volume-dependent EQ
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Set 5-band EQ
    Eq {
        /// Low-bass band (min: -6.0, max: 6.0)
        #[arg(value_parser=parse_eq_value)]
        low_bass: f32,

        /// Bass band (min: -6.0, max: 6.0)
        #[arg(value_parser=parse_eq_value)]
        bass: f32,

        /// Mid band (min: -6.0, max: 6.0)
        #[arg(value_parser=parse_eq_value)]
        mid: f32,

        /// Treble band (min: -6.0, max: 6.0)
        #[arg(value_parser=parse_eq_value)]
        treble: f32,

        /// Upper treble band (min: -6.0, max: 6.0)
        #[arg(value_parser=parse_eq_value)]
        upper_treble: f32,
    },

    /// Set volume balance
    Balance {
        /// Volume balance from -100 (left) to +100 (right)
        #[arg(value_parser=parse_balance)]
        value: i32,
    },

    /// Set mono output
    Mono {
        /// Whether to force mono output
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Enable/disable volume level exposure notifications
    VolumeExposureNotifications {
        /// Whether to enable or disable volume level exposure notifications
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Enable/disable automatic transparency mode via speech detection
    SpeechDetection {
        /// Whether to enable or disable the automatic transparency mode via speech detection
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },
}

#[derive(Debug, ValueEnum, Clone, Copy)]
pub enum AncState {
    Off,
    Active,
    Aware,
    CycleNext,
    CyclePrev,
}

#[derive(Debug, ValueEnum, Clone, Copy)]
pub enum HoldGestureAction {
    Anc,
    Assistant,
}

impl From<HoldGestureAction> for settings::RegularActionTarget {
    fn from(value: HoldGestureAction) -> Self {
        match value {
            HoldGestureAction::Anc => settings::RegularActionTarget::AncControl,
            HoldGestureAction::Assistant => settings::RegularActionTarget::AssistantQuery,
        }
    }
}

fn parse_eq_value(s: &str) -> std::result::Result<f32, String> {
    let val = s.parse().map_err(|e| format!("{e}"))?;

    if val > settings::EqBands::MAX_VALUE {
        Err(format!("exceeds maximum of {}", settings::EqBands::MAX_VALUE))
    } else if val < settings::EqBands::MIN_VALUE {
        Err(format!("exceeds minimum of {}", settings::EqBands::MIN_VALUE))
    } else {
        Ok(val)
    }
}

fn parse_balance(s: &str) -> std::result::Result<i32, String> {
    let val = s.parse().map_err(|e| format!("{e}"))?;

    if val > 100 {
        Err("exceeds maximum of 100".to_string())
    } else if val < -100 {
        Err("exceeds minimum of -100".to_string())
    } else {
        Ok(val)
    }
}
