use num_enum::{IntoPrimitive, FromPrimitive};

use crate::protocol::types;


#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum SettingId {
    AutoOtaEnable = 1,
    OhdEnable = 2,
    OobeIsFinished = 3,
    GestureEnable = 4,
    DiagnosticsEnable = 5,
    OobeMode = 6,
    GestureControl = 7,
    AncAccessibilityMode = 8,
    AncrStateOneBud = 9,
    AncrStateTwoBuds = 10,
    MultipointEnable = 11,
    AncrGestureLoop = 12,
    CurrentAncrState = 13,
    OttsMode = 14,
    VolumeEqEnable = 15,
    CurrentUserEq = 16,
    VolumeAsymmetry = 17,
    LastSavedUserEq = 18,
    SumToMono = 19,
    VolumeExposureNotifications = 21,
    SpeechDetection = 22,

    #[num_enum(catch_all)]
    Unknown(i32),
}


#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    AutoOtaEnable(bool),
    OhdEnable(bool),
    OobeIsFinished(bool),
    GestureEnable(bool),
    DiagnosticsEnable(bool),
    OobeMode(bool),
    GestureControl(GestureControl),
    MultipointEnable(bool),
    AncrGestureLoop(AncrGestureLoop),
    CurrentAncrState(AncState),
    OttsMode(i32),
    VolumeEqEnable(bool),
    CurrentUserEq(EqBands),
    VolumeAsymmetry(VolumeAsymmetry),
    SumToMono(bool),
    VolumeExposureNotifications(bool),
    SpeechDetection(bool),
}

impl SettingValue {
    pub fn id(&self) -> SettingId {
        match self {
            SettingValue::AutoOtaEnable(_) => SettingId::AutoOtaEnable,
            SettingValue::OhdEnable(_) => SettingId::OhdEnable,
            SettingValue::OobeIsFinished(_) => SettingId::OobeIsFinished,
            SettingValue::GestureEnable(_) => SettingId::GestureEnable,
            SettingValue::DiagnosticsEnable(_) => SettingId::DiagnosticsEnable,
            SettingValue::OobeMode(_) => SettingId::OobeMode,
            SettingValue::GestureControl(_) => SettingId::GestureControl,
            SettingValue::MultipointEnable(_) => SettingId::MultipointEnable,
            SettingValue::AncrGestureLoop(_) => SettingId::AncrGestureLoop,
            SettingValue::CurrentAncrState(_) => SettingId::CurrentAncrState,
            SettingValue::OttsMode(_) => SettingId::OttsMode,
            SettingValue::VolumeEqEnable(_) => SettingId::VolumeEqEnable,
            SettingValue::CurrentUserEq(_) => SettingId::CurrentUserEq,
            SettingValue::VolumeAsymmetry(_) => SettingId::VolumeAsymmetry,
            SettingValue::SumToMono(_) => SettingId::SumToMono,
            SettingValue::VolumeExposureNotifications(_) => SettingId::VolumeExposureNotifications,
            SettingValue::SpeechDetection(_) => SettingId::SpeechDetection,
        }
    }
}

impl From<types::setting_value::ValueOneof> for SettingValue {
    fn from(value: crate::protocol::types::setting_value::ValueOneof) -> Self {
        use types::setting_value::ValueOneof;

        match value {
            ValueOneof::AutoOtaEnable(x) => SettingValue::AutoOtaEnable(x),
            ValueOneof::OhdEnable(x) => SettingValue::OhdEnable(x),
            ValueOneof::OobeIsFinished(x) => SettingValue::OobeIsFinished(x),
            ValueOneof::GestureEnable(x) => SettingValue::GestureEnable(x),
            ValueOneof::DiagnosticsEnable(x) => SettingValue::DiagnosticsEnable(x),
            ValueOneof::OobeMode(x) => SettingValue::OobeMode(x),
            ValueOneof::GestureControl(x) => SettingValue::GestureControl(GestureControl::from(x)),
            ValueOneof::MultipointEnable(x) => SettingValue::MultipointEnable(x),
            ValueOneof::AncrGestureLoop(x) => SettingValue::AncrGestureLoop(AncrGestureLoop::from(x)),
            ValueOneof::CurrentAncrState(x) => SettingValue::CurrentAncrState(AncState::from_primitive(x)),
            ValueOneof::OttsMode(x) => SettingValue::OttsMode(x),
            ValueOneof::VolumeEqEnable(x) => SettingValue::VolumeEqEnable(x),
            ValueOneof::CurrentUserEq(x) => SettingValue::CurrentUserEq(EqBands::from(x)),
            ValueOneof::VolumeAsymmetry(x) => SettingValue::VolumeAsymmetry(VolumeAsymmetry::from_raw(x)),
            ValueOneof::SumToMono(x) => SettingValue::SumToMono(x),
            ValueOneof::VolumeExposureNotifications(x) => SettingValue::VolumeExposureNotifications(x),
            ValueOneof::SpeechDetection(x) => SettingValue::SpeechDetection(x),
        }
    }
}

impl From<SettingValue> for types::setting_value::ValueOneof {
    fn from(value: SettingValue) -> Self {
        use types::setting_value::ValueOneof;

        match value {
            SettingValue::AutoOtaEnable(x) => ValueOneof::AutoOtaEnable(x),
            SettingValue::OhdEnable(x) => ValueOneof::OhdEnable(x),
            SettingValue::OobeIsFinished(x) => ValueOneof::OobeIsFinished(x),
            SettingValue::GestureEnable(x) => ValueOneof::GestureEnable(x),
            SettingValue::DiagnosticsEnable(x) => ValueOneof::DiagnosticsEnable(x),
            SettingValue::OobeMode(x) => ValueOneof::OobeMode(x),
            SettingValue::GestureControl(x) => ValueOneof::GestureControl(x.into()),
            SettingValue::MultipointEnable(x) => ValueOneof::MultipointEnable(x),
            SettingValue::AncrGestureLoop(x) => ValueOneof::AncrGestureLoop(x.into()),
            SettingValue::CurrentAncrState(x) => ValueOneof::CurrentAncrState(x.into()),
            SettingValue::OttsMode(x) => ValueOneof::OttsMode(x),
            SettingValue::VolumeEqEnable(x) => ValueOneof::VolumeEqEnable(x),
            SettingValue::CurrentUserEq(x) => ValueOneof::CurrentUserEq(x.into()),
            SettingValue::VolumeAsymmetry(x) => ValueOneof::VolumeAsymmetry(x.raw()),
            SettingValue::SumToMono(x) => ValueOneof::SumToMono(x),
            SettingValue::VolumeExposureNotifications(x) => ValueOneof::VolumeExposureNotifications(x),
            SettingValue::SpeechDetection(x) => ValueOneof::SpeechDetection(x),
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GestureControl {
    pub left: RegularActionTarget,
    pub right: RegularActionTarget,
}

impl From<types::GestureControl> for GestureControl {
    fn from(value: types::GestureControl) -> Self {
        let left = value.left
            .and_then(|v| v.value_oneof)
            .map(|types::device_gesture_control::ValueOneof::Type(x)| x)
            .map(|v| RegularActionTarget::from_primitive(v.value))
            .unwrap_or(RegularActionTarget::Unknown(-1));

        let right = value.right
            .and_then(|v| v.value_oneof)
            .map(|types::device_gesture_control::ValueOneof::Type(x)| x)
            .map(|v| RegularActionTarget::from_primitive(v.value))
            .unwrap_or(RegularActionTarget::Unknown(-1));

        GestureControl { left, right }
    }
}

impl From<GestureControl> for types::GestureControl {
    fn from(value: GestureControl) -> Self {
        use types::device_gesture_control::ValueOneof;

        let left = types::DeviceGestureControl {
            value_oneof: Some(ValueOneof::Type(types::GestureControlType {
                value: value.left.into(),
            })),
        };

        let right = types::DeviceGestureControl {
            value_oneof: Some(ValueOneof::Type(types::GestureControlType {
                value: value.right.into(),
            })),
        };

        Self {
            left: Some(left),
            right: Some(right),
        }
    }
}

impl Default for GestureControl {
    fn default() -> Self {
        Self {
            left: RegularActionTarget::AncControl,
            right: RegularActionTarget::AncControl,
        }
    }
}

impl std::fmt::Display for GestureControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "left: {}, right: {}", self.left, self.right)
    }
}


#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum RegularActionTarget {
    CheckNotifications = 1,
    PreviousTrackRepeat = 2,
    NextTrack = 3,
    PlayPauseTrack = 4,
    AncControl = 5,
    AssistantQuery = 6,

    #[num_enum(catch_all)]
    Unknown(i32),
}

impl RegularActionTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            RegularActionTarget::CheckNotifications => "check-notifications",
            RegularActionTarget::PreviousTrackRepeat => "previous",
            RegularActionTarget::NextTrack => "next",
            RegularActionTarget::PlayPauseTrack => "play-pause",
            RegularActionTarget::AncControl => "anc",
            RegularActionTarget::AssistantQuery => "assistant",
            RegularActionTarget::Unknown(_) => "unknown",
        }
    }
}

impl std::fmt::Display for RegularActionTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegularActionTarget::CheckNotifications => write!(f, "check-notifications"),
            RegularActionTarget::PreviousTrackRepeat => write!(f, "previous"),
            RegularActionTarget::NextTrack => write!(f, "next"),
            RegularActionTarget::PlayPauseTrack => write!(f, "play-pause"),
            RegularActionTarget::AncControl => write!(f, "anc"),
            RegularActionTarget::AssistantQuery => write!(f, "assistant"),
            RegularActionTarget::Unknown(x) => write!(f, "unknown ({x})"),
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AncrGestureLoop {
    pub active: bool,
    pub off: bool,
    pub aware: bool,
}

impl AncrGestureLoop {
    pub fn is_valid(&self) -> bool {
        // at least two need to be set
        (self.active as u32 + self.off as u32 + self.aware as u32) >= 2
    }
}

impl From<types::AncrGestureLoop> for AncrGestureLoop {
    fn from(other: types::AncrGestureLoop) -> Self {
        AncrGestureLoop { active: other.active, off: other.off, aware: other.aware }
    }
}

impl From<AncrGestureLoop> for types::AncrGestureLoop {
    fn from(other: AncrGestureLoop) -> Self {
        Self {
            active: other.active,
            off: other.off,
            aware: other.aware,
        }
    }
}

impl std::fmt::Display for AncrGestureLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut n = 0;

        write!(f, "[")?;

        if self.active {
            write!(f, "active")?;
            n += 1;
        }

        if self.off {
            if n > 0 {
                write!(f, ", ")?;
            }

            write!(f, "off")?;
            n += 1;
        }

        if self.aware {
            if n > 0 {
                write!(f, ", ")?;
            }

            write!(f, "aware")?;
        }

        write!(f, "]")
    }
}


#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum AncState {
    Off = 1,
    Active = 2,
    Aware = 3,

    #[num_enum(catch_all)]
    Unknown(i32),
}

impl AncState {
    pub fn as_str(&self) -> &'static str {
        match self {
            AncState::Off => "off",
            AncState::Active => "active",
            AncState::Aware => "aware",
            AncState::Unknown(_) => "unknown",
        }
    }
}

// #[derive(Default)] clashes with #[derive(FromPrimitive)]
#[allow(clippy::derivable_impls)]
impl Default for AncState {
    fn default() -> Self {
        AncState::Off
    }
}

impl std::fmt::Display for AncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AncState::Off => write!(f, "off"),
            AncState::Active => write!(f, "active"),
            AncState::Aware => write!(f, "aware"),
            AncState::Unknown(x) => write!(f, "unknown ({x})"),
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqBands {
    low_bass: f32,
    bass: f32,
    mid: f32,
    treble: f32,
    upper_treble: f32,
}

impl EqBands {
    pub const MIN_VALUE: f32 = -6.0;
    pub const MAX_VALUE: f32 = 6.0;

    pub fn new(low_bass: f32, bass: f32, mid: f32, treble: f32, upper_treble: f32) -> Self {
        Self {
            low_bass: low_bass.clamp(Self::MIN_VALUE, Self::MAX_VALUE),
            bass: bass.clamp(Self::MIN_VALUE, Self::MAX_VALUE),
            mid: mid.clamp(Self::MIN_VALUE, Self::MAX_VALUE),
            treble: treble.clamp(Self::MIN_VALUE, Self::MAX_VALUE),
            upper_treble: upper_treble.clamp(Self::MIN_VALUE, Self::MAX_VALUE),
        }
    }

    pub fn low_bass(&self) -> f32 {
        self.low_bass
    }

    pub fn bass(&self) -> f32 {
        self.bass
    }

    pub fn mid(&self) -> f32 {
        self.mid
    }

    pub fn treble(&self) -> f32 {
        self.treble
    }

    pub fn upper_treble(&self) -> f32 {
        self.upper_treble
    }

    pub fn set_low_bass(&mut self, value: f32) {
        self.low_bass = value.clamp(Self::MIN_VALUE, Self::MAX_VALUE)
    }

    pub fn set_bass(&mut self, value: f32) {
        self.bass = value.clamp(Self::MIN_VALUE, Self::MAX_VALUE)
    }

    pub fn set_mid(&mut self, value: f32) {
        self.mid = value.clamp(Self::MIN_VALUE, Self::MAX_VALUE)
    }

    pub fn set_treble(&mut self, value: f32) {
        self.treble = value.clamp(Self::MIN_VALUE, Self::MAX_VALUE)
    }

    pub fn set_upper_treble(&mut self, value: f32) {
        self.upper_treble = value.clamp(Self::MIN_VALUE, Self::MAX_VALUE)
    }
}

impl Default for EqBands {
    fn default() -> Self {
        Self {
            low_bass: 0.0,
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            upper_treble: 0.0
        }
    }
}

impl From<types::EqBands> for EqBands {
    fn from(other: types::EqBands) -> Self {
        Self {
            low_bass: other.low_bass,
            bass: other.bass,
            mid: other.mid,
            treble: other.treble,
            upper_treble: other.upper_treble,
        }
    }
}

impl From<EqBands> for types::EqBands {
    fn from(other: EqBands) -> Self {
        Self {
            low_bass: other.low_bass.clamp(EqBands::MIN_VALUE, EqBands::MAX_VALUE),
            bass: other.bass.clamp(EqBands::MIN_VALUE, EqBands::MAX_VALUE),
            mid: other.mid.clamp(EqBands::MIN_VALUE, EqBands::MAX_VALUE),
            treble: other.treble.clamp(EqBands::MIN_VALUE, EqBands::MAX_VALUE),
            upper_treble: other.upper_treble.clamp(EqBands::MIN_VALUE, EqBands::MAX_VALUE),
        }
    }
}

impl std::fmt::Display for EqBands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f, "[{:.2}, {:.2}, {:.2}, {:.2}, {:.2}]",
            self.low_bass, self.bass, self.mid, self.treble, self.upper_treble,
        )
    }
}


#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct VolumeAsymmetry {
    value: i32,
}

impl VolumeAsymmetry {
    pub fn from_normalized(value: i32) -> Self {
        Self { value: value.clamp(-100, 100) }
    }

    pub fn from_raw(value: i32) -> Self {
        let direction = value & 0x01;
        let value = value >> 1;

        let normalized = if direction != 0 {
            value + 1
        } else {
            - value
        };

        Self { value: normalized }
    }

    pub fn raw(&self) -> i32 {
        if self.value > 0 {
            ((self.value - 1) << 1) | 0x01
        } else {
            (-self.value) << 1
        }
    }

    pub fn value(&self) -> i32 {
        self.value
    }
}

impl std::fmt::Debug for VolumeAsymmetry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl std::fmt::Display for VolumeAsymmetry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let left = (100 - self.value).min(100);
        let right = (100 + self.value).min(100);

        write!(f, "left: {left}%, right: {right}%")
    }
}


pub trait Setting {
    type Type;

    fn id(&self) -> SettingId;
    fn from_var(var: SettingValue) -> Option<Self::Type>;
}

impl Setting for SettingId {
    type Type = SettingValue;

    fn id(&self) -> SettingId {
        *self
    }

    fn from_var(var: SettingValue) -> Option<Self::Type> {
        Some(var)
    }
}


pub mod id {
    use super::*;

    pub struct AutoOtaEnable;
    pub struct OhdEnable;
    pub struct OobeIsFinished;
    pub struct GestureEnable;
    pub struct DiagnosticsEnable;
    pub struct OobeMode;
    pub struct GestureControl;
    pub struct MultipointEnable;
    pub struct AncrGestureLoop;
    pub struct CurrentAncrState;
    pub struct OttsMode;
    pub struct VolumeEqEnable;
    pub struct CurrentUserEq;
    pub struct VolumeAsymmetry;
    pub struct SumToMono;
    pub struct VolumeExposureNotifications;
    pub struct SpeechDetection;

    impl Setting for AutoOtaEnable {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::AutoOtaEnable
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::AutoOtaEnable(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for OhdEnable {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::OhdEnable
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::OhdEnable(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for OobeIsFinished {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::OobeIsFinished
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::OobeIsFinished(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for GestureEnable {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::GestureEnable
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::GestureEnable(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for DiagnosticsEnable {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::DiagnosticsEnable
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::DiagnosticsEnable(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for OobeMode {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::OobeMode
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::OobeMode(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for GestureControl {
        type Type = super::GestureControl;

        fn id(&self) -> SettingId {
            SettingId::GestureControl
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::GestureControl(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for MultipointEnable {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::MultipointEnable
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::MultipointEnable(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for AncrGestureLoop {
        type Type = super::AncrGestureLoop;

        fn id(&self) -> SettingId {
            SettingId::AncrGestureLoop
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::AncrGestureLoop(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for CurrentAncrState {
        type Type = AncState;

        fn id(&self) -> SettingId {
            SettingId::CurrentAncrState
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::CurrentAncrState(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for OttsMode {
        type Type = i32;

        fn id(&self) -> SettingId {
            SettingId::OttsMode
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::OttsMode(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for VolumeEqEnable {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::VolumeEqEnable
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::VolumeEqEnable(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for CurrentUserEq {
        type Type = EqBands;

        fn id(&self) -> SettingId {
            SettingId::CurrentUserEq
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::CurrentUserEq(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for VolumeAsymmetry {
        type Type = super::VolumeAsymmetry;

        fn id(&self) -> SettingId {
            SettingId::VolumeAsymmetry
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::VolumeAsymmetry(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for SumToMono {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::SumToMono
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::SumToMono(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for VolumeExposureNotifications {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::VolumeExposureNotifications
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::VolumeExposureNotifications(x) => Some(x),
                _ => None,
            }
        }
    }

    impl Setting for SpeechDetection {
        type Type = bool;

        fn id(&self) -> SettingId {
            SettingId::SpeechDetection
        }

        fn from_var(var: SettingValue) -> Option<Self::Type> {
            match var {
                SettingValue::SpeechDetection(x) => Some(x),
                _ => None,
            }
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_volume_assymetry_conversion() {
        for i in 0..=200 {
            assert_eq!(VolumeAsymmetry::from_raw(i).raw(), i)
        }
    }
}
