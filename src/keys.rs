//! Resolve key names ("KEY_UP") to evdev codes, and back. Profiles reference keys
//! by name; unknown names fail at load with a clear message rather than silently.

use evdev::{AbsoluteAxisType, Key};

/// Human name for a key code, e.g. 103 -> "KEY_UP" (for logs).
pub fn name(code: u16) -> String {
    format!("{:?}", Key::new(code))
}

/// Human name for an absolute-axis code, e.g. "ABS_Y" (for capture output).
pub fn abs_name(code: u16) -> String {
    format!("{:?}", AbsoluteAxisType(code))
}

/// Map an evdev key name to its numeric code. Covers the keys useful for driving
/// a terminal/TUI; extend as needed.
pub fn code(name: &str) -> Option<u16> {
    let k = match name {
        // navigation / editing
        "KEY_UP" => Key::KEY_UP,
        "KEY_DOWN" => Key::KEY_DOWN,
        "KEY_LEFT" => Key::KEY_LEFT,
        "KEY_RIGHT" => Key::KEY_RIGHT,
        "KEY_ENTER" => Key::KEY_ENTER,
        "KEY_KPENTER" => Key::KEY_KPENTER,
        "KEY_ESC" => Key::KEY_ESC,
        "KEY_TAB" => Key::KEY_TAB,
        "KEY_SPACE" => Key::KEY_SPACE,
        "KEY_BACKSPACE" => Key::KEY_BACKSPACE,
        "KEY_DELETE" => Key::KEY_DELETE,
        "KEY_INSERT" => Key::KEY_INSERT,
        "KEY_HOME" => Key::KEY_HOME,
        "KEY_END" => Key::KEY_END,
        "KEY_PAGEUP" => Key::KEY_PAGEUP,
        "KEY_PAGEDOWN" => Key::KEY_PAGEDOWN,
        // modifiers
        "KEY_LEFTCTRL" => Key::KEY_LEFTCTRL,
        "KEY_RIGHTCTRL" => Key::KEY_RIGHTCTRL,
        "KEY_LEFTALT" => Key::KEY_LEFTALT,
        "KEY_RIGHTALT" => Key::KEY_RIGHTALT,
        "KEY_LEFTSHIFT" => Key::KEY_LEFTSHIFT,
        "KEY_RIGHTSHIFT" => Key::KEY_RIGHTSHIFT,
        "KEY_LEFTMETA" => Key::KEY_LEFTMETA,
        // letters
        "KEY_A" => Key::KEY_A,
        "KEY_B" => Key::KEY_B,
        "KEY_C" => Key::KEY_C,
        "KEY_D" => Key::KEY_D,
        "KEY_E" => Key::KEY_E,
        "KEY_F" => Key::KEY_F,
        "KEY_G" => Key::KEY_G,
        "KEY_H" => Key::KEY_H,
        "KEY_I" => Key::KEY_I,
        "KEY_J" => Key::KEY_J,
        "KEY_K" => Key::KEY_K,
        "KEY_L" => Key::KEY_L,
        "KEY_M" => Key::KEY_M,
        "KEY_N" => Key::KEY_N,
        "KEY_O" => Key::KEY_O,
        "KEY_P" => Key::KEY_P,
        "KEY_Q" => Key::KEY_Q,
        "KEY_R" => Key::KEY_R,
        "KEY_S" => Key::KEY_S,
        "KEY_T" => Key::KEY_T,
        "KEY_U" => Key::KEY_U,
        "KEY_V" => Key::KEY_V,
        "KEY_W" => Key::KEY_W,
        "KEY_X" => Key::KEY_X,
        "KEY_Y" => Key::KEY_Y,
        "KEY_Z" => Key::KEY_Z,
        // media / consumer (what many of these gadgets natively emit)
        "KEY_VOLUMEUP" => Key::KEY_VOLUMEUP,
        "KEY_VOLUMEDOWN" => Key::KEY_VOLUMEDOWN,
        "KEY_MUTE" => Key::KEY_MUTE,
        "KEY_PLAYPAUSE" => Key::KEY_PLAYPAUSE,
        "KEY_NEXTSONG" => Key::KEY_NEXTSONG,
        "KEY_PREVIOUSSONG" => Key::KEY_PREVIOUSSONG,
        "KEY_STOPCD" => Key::KEY_STOPCD,
        "KEY_POWER" => Key::KEY_POWER,
        // gamepad buttons (as an input side of a keymap translator)
        "BTN_SOUTH" => Key::BTN_SOUTH,
        "BTN_EAST" => Key::BTN_EAST,
        "BTN_NORTH" => Key::BTN_NORTH,
        "BTN_WEST" => Key::BTN_WEST,
        "BTN_TL" => Key::BTN_TL,
        "BTN_TR" => Key::BTN_TR,
        "BTN_SELECT" => Key::BTN_SELECT,
        "BTN_START" => Key::BTN_START,
        "BTN_MODE" => Key::BTN_MODE,
        "BTN_THUMBL" => Key::BTN_THUMBL,
        "BTN_THUMBR" => Key::BTN_THUMBR,
        _ => return None,
    };
    Some(k.code())
}

/// Resolve an absolute-axis name ("ABS_HAT0Y") to its code, for the `axis`
/// translator. Returns None for unknown names (fails at profile load).
pub fn abs_code(name: &str) -> Option<u16> {
    use evdev::AbsoluteAxisType as A;
    let a = match name {
        "ABS_X" => A::ABS_X,
        "ABS_Y" => A::ABS_Y,
        "ABS_Z" => A::ABS_Z,
        "ABS_RX" => A::ABS_RX,
        "ABS_RY" => A::ABS_RY,
        "ABS_RZ" => A::ABS_RZ,
        "ABS_HAT0X" => A::ABS_HAT0X,
        "ABS_HAT0Y" => A::ABS_HAT0Y,
        _ => return None,
    };
    Some(a.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_and_rejects_unknown() {
        assert_eq!(code("KEY_UP"), Some(Key::KEY_UP.code()));
        assert_eq!(code("KEY_ENTER"), Some(Key::KEY_ENTER.code()));
        assert_eq!(code("KEY_NOPE"), None);
    }

    #[test]
    fn round_trips_to_name() {
        assert_eq!(name(Key::KEY_UP.code()), "KEY_UP");
    }
}
