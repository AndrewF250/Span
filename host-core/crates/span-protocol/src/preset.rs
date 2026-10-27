//! Panel documents. The host executes a document only after a person accepts
//! it. A peer-supplied layout is not an instruction to launch programs.

use serde::Deserialize;

use crate::error::ProtocolError;

const MAX_COLUMNS: u32 = 8;
const MAX_ROWS: u32 = 8;
const MAX_LABEL_CHARS: usize = 48;
const MAX_NAME_CHARS: usize = 48;
const MAX_ACTIONS: u32 = 32;
const MAX_DEPTH: u8 = 4;
const MAX_KEYS: usize = 6;
const MAX_PATH: usize = 4096;

/// `KeyboardEvent.code` names the host knows how to turn into HID usages.
const KEYS: &[&str] = &[
    "AltLeft",
    "AltRight",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "AudioVolumeDown",
    "AudioVolumeMute",
    "AudioVolumeUp",
    "Backquote",
    "Backslash",
    "Backspace",
    "BracketLeft",
    "BracketRight",
    "Comma",
    "ControlLeft",
    "ControlRight",
    "Delete",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "End",
    "Enter",
    "Equal",
    "Escape",
    "F1",
    "F10",
    "F11",
    "F12",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "Home",
    "Insert",
    "KeyA",
    "KeyB",
    "KeyC",
    "KeyD",
    "KeyE",
    "KeyF",
    "KeyG",
    "KeyH",
    "KeyI",
    "KeyJ",
    "KeyK",
    "KeyL",
    "KeyM",
    "KeyN",
    "KeyO",
    "KeyP",
    "KeyQ",
    "KeyR",
    "KeyS",
    "KeyT",
    "KeyU",
    "KeyV",
    "KeyW",
    "KeyX",
    "KeyY",
    "KeyZ",
    "MediaPlayPause",
    "MediaTrackNext",
    "MediaTrackPrevious",
    "MetaLeft",
    "MetaRight",
    "Minus",
    "PageDown",
    "PageUp",
    "Period",
    "Quote",
    "Semicolon",
    "ShiftLeft",
    "ShiftRight",
    "Slash",
    "Space",
    "Tab",
];

pub fn key_codes() -> &'static [&'static str] {
    KEYS
}

pub(crate) fn control_id_ok(text: &str) -> bool {
    plain_id(text, 32)
}

pub(crate) fn preset_id_ok(text: &str) -> bool {
    dotted_id(text, 64)
}

pub fn parse(bytes: &[u8]) -> Result<PresetDocument, ProtocolError> {
    if bytes.len() > crate::codes::MAX_PRESET_BYTES {
        return Err(ProtocolError::BadPreset { reason: "size" });
    }
    let document: PresetDocument =
        serde_json::from_slice(bytes).map_err(|_| ProtocolError::BadPreset { reason: "json" })?;
    document.check()?;
    Ok(document)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresetDocument {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub locale: Option<Locale>,
    pub columns: u32,
    pub rows: u32,
    pub controls: Vec<Control>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    Ru,
    En,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Control {
    Button {
        id: String,
        row: u32,
        col: u32,
        row_span: u32,
        col_span: u32,
        label: String,
        gestures: Box<Gestures>,
    },
    Pad {
        id: String,
        row: u32,
        col: u32,
        row_span: u32,
        col_span: u32,
        label: String,
        output: AnalogOutput,
    },
    Joystick {
        id: String,
        row: u32,
        col: u32,
        row_span: u32,
        col_span: u32,
        label: String,
        output: AnalogOutput,
    },
    Wheel {
        id: String,
        row: u32,
        col: u32,
        row_span: u32,
        col_span: u32,
        label: String,
        output: AnalogOutput,
    },
    Slider {
        id: String,
        row: u32,
        col: u32,
        row_span: u32,
        col_span: u32,
        label: String,
        output: AnalogOutput,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalogOutput {
    Mouse,
    Scroll,
    Volume,
    Wasd,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Gestures {
    #[serde(default)]
    pub tap: Option<Action>,
    #[serde(default)]
    pub double_tap: Option<Action>,
    #[serde(default)]
    pub long_press: Option<Action>,
    #[serde(default)]
    pub swipe_up: Option<Action>,
    #[serde(default)]
    pub swipe_down: Option<Action>,
    #[serde(default)]
    pub swipe_left: Option<Action>,
    #[serde(default)]
    pub swipe_right: Option<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Action {
    Hotkey {
        keys: Vec<String>,
    },
    Delay {
        ms: u32,
    },
    Sequence {
        steps: Vec<Action>,
    },
    Launch {
        #[serde(default)]
        windows: Option<String>,
        #[serde(default)]
        linux: Option<String>,
    },
    Media {
        command: MediaCommand,
    },
    Script {
        id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaCommand {
    Play,
    Pause,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
    Mute,
}

impl PresetDocument {
    fn check(&self) -> Result<(), ProtocolError> {
        if self.schema_version != 1 {
            return Err(ProtocolError::BadPreset { reason: "schema" });
        }
        if !dotted_id(&self.id, 64) || !label_ok(&self.name, MAX_NAME_CHARS) {
            return Err(ProtocolError::BadPreset { reason: "identity" });
        }
        if self.columns == 0 || self.rows == 0 || self.columns > MAX_COLUMNS || self.rows > MAX_ROWS
        {
            return Err(ProtocolError::BadPreset { reason: "grid" });
        }
        if self.controls.is_empty() || self.controls.len() > 64 {
            return Err(ProtocolError::BadPreset { reason: "controls" });
        }
        let mut seen = Vec::<&str>::with_capacity(self.controls.len());
        let mut rects = Vec::with_capacity(self.controls.len());
        for control in &self.controls {
            let (id, rect) = control.check()?;
            if seen.contains(&id) {
                return Err(ProtocolError::BadPreset {
                    reason: "duplicate",
                });
            }
            seen.push(id);
            rects.push(rect);
        }
        for (index, rect) in rects.iter().enumerate() {
            if rect.col + rect.col_span > self.columns || rect.row + rect.row_span > self.rows {
                return Err(ProtocolError::BadPreset { reason: "grid" });
            }
            for other in &rects[index + 1..] {
                if rect.overlaps(other) {
                    return Err(ProtocolError::BadPreset { reason: "overlap" });
                }
            }
        }
        Ok(())
    }
}

struct Rect {
    row: u32,
    col: u32,
    row_span: u32,
    col_span: u32,
}

impl Rect {
    fn overlaps(&self, other: &Self) -> bool {
        let separated = self.col + self.col_span <= other.col
            || other.col + other.col_span <= self.col
            || self.row + self.row_span <= other.row
            || other.row + other.row_span <= self.row;
        !separated
    }
}

impl Control {
    fn check(&self) -> Result<(&str, Rect), ProtocolError> {
        let (id, row, col, row_span, col_span, label) = match self {
            Self::Button {
                id,
                row,
                col,
                row_span,
                col_span,
                label,
                gestures,
            } => {
                gestures.check()?;
                (id, *row, *col, *row_span, *col_span, label)
            }
            Self::Pad {
                id,
                row,
                col,
                row_span,
                col_span,
                label,
                output,
            } => {
                if !matches!(output, AnalogOutput::Mouse | AnalogOutput::Scroll) {
                    return Err(ProtocolError::BadPreset { reason: "output" });
                }
                (id, *row, *col, *row_span, *col_span, label)
            }
            Self::Joystick {
                id,
                row,
                col,
                row_span,
                col_span,
                label,
                output,
            } => {
                if !matches!(output, AnalogOutput::Mouse | AnalogOutput::Wasd) {
                    return Err(ProtocolError::BadPreset { reason: "output" });
                }
                (id, *row, *col, *row_span, *col_span, label)
            }
            Self::Wheel {
                id,
                row,
                col,
                row_span,
                col_span,
                label,
                output,
            } => {
                if !matches!(output, AnalogOutput::Scroll | AnalogOutput::Volume) {
                    return Err(ProtocolError::BadPreset { reason: "output" });
                }
                (id, *row, *col, *row_span, *col_span, label)
            }
            Self::Slider {
                id,
                row,
                col,
                row_span,
                col_span,
                label,
                output,
            } => {
                if !matches!(output, AnalogOutput::Volume) {
                    return Err(ProtocolError::BadPreset { reason: "output" });
                }
                (id, *row, *col, *row_span, *col_span, label)
            }
        };
        if !plain_id(id, 32) || !label_ok(label, MAX_LABEL_CHARS) || row_span == 0 || col_span == 0
        {
            return Err(ProtocolError::BadPreset { reason: "control" });
        }
        Ok((
            id.as_str(),
            Rect {
                row,
                col,
                row_span,
                col_span,
            },
        ))
    }
}

impl Gestures {
    fn check(&self) -> Result<(), ProtocolError> {
        let actions = [
            self.tap.as_ref(),
            self.double_tap.as_ref(),
            self.long_press.as_ref(),
            self.swipe_up.as_ref(),
            self.swipe_down.as_ref(),
            self.swipe_left.as_ref(),
            self.swipe_right.as_ref(),
        ];
        if actions.iter().all(Option::is_none) {
            return Err(ProtocolError::BadPreset { reason: "gesture" });
        }
        let mut count = 0u32;
        for action in actions.into_iter().flatten() {
            check_action(action, 1, &mut count)?;
        }
        Ok(())
    }
}

fn check_action(action: &Action, depth: u8, count: &mut u32) -> Result<(), ProtocolError> {
    if depth > MAX_DEPTH || *count >= MAX_ACTIONS {
        return Err(ProtocolError::BadPreset { reason: "depth" });
    }
    *count += 1;
    match action {
        Action::Hotkey { keys } => {
            if keys.is_empty() || keys.len() > MAX_KEYS {
                return Err(ProtocolError::BadPreset { reason: "keys" });
            }
            let mut seen = Vec::with_capacity(keys.len());
            for key in keys {
                if KEYS.binary_search(&key.as_str()).is_err() {
                    return Err(ProtocolError::BadPreset { reason: "keys" });
                }
                if seen.contains(&key.as_str()) {
                    return Err(ProtocolError::BadPreset { reason: "keys" });
                }
                seen.push(key.as_str());
            }
        }
        Action::Delay { ms } => {
            if *ms == 0 || *ms > 10_000 {
                return Err(ProtocolError::BadPreset { reason: "delay" });
            }
        }
        Action::Sequence { steps } => {
            if steps.is_empty() {
                return Err(ProtocolError::BadPreset { reason: "sequence" });
            }
            for step in steps {
                check_action(step, depth + 1, count)?;
            }
        }
        Action::Launch { windows, linux } => {
            if windows.is_none() && linux.is_none() {
                return Err(ProtocolError::BadPreset { reason: "launch" });
            }
            if let Some(path) = windows {
                check_path(path)?;
            }
            if let Some(path) = linux {
                check_path(path)?;
            }
        }
        Action::Media { .. } => {}
        Action::Script { id } => {
            if !dotted_id(id, 64) {
                return Err(ProtocolError::BadPreset { reason: "script" });
            }
        }
    }
    Ok(())
}

fn check_path(path: &str) -> Result<(), ProtocolError> {
    if path.is_empty()
        || path.len() > MAX_PATH
        || path.chars().any(|ch| ch == '\0' || ch.is_control())
    {
        return Err(ProtocolError::BadPreset { reason: "launch" });
    }
    Ok(())
}

fn label_ok(text: &str, max_chars: usize) -> bool {
    let count = text.chars().count();
    (1..=max_chars).contains(&count) && !text.chars().any(|ch| ch.is_control())
}

fn plain_id(text: &str, max: usize) -> bool {
    ident(text, max, false)
}

fn dotted_id(text: &str, max: usize) -> bool {
    ident(text, max, true)
}

fn ident(text: &str, max: usize, dots: bool) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() > max || !bytes[0].is_ascii_lowercase() {
        return false;
    }
    bytes[1..].iter().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || *byte == b'_'
            || *byte == b'-'
            || (dots && *byte == b'.')
    })
}
