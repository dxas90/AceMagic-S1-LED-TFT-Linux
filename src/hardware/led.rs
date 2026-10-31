//! LED strip driver for the AceMagic S1 (CH340 USB-serial bridge).
//!
//! Protocol reference from s1panel/led_device.js:
//! - VID: 0x1A86, PID: 0x7523  (CH340 USB-serial)
//! - Baud: 10000 (non-standard, must be set exactly)
#![allow(dead_code)]
//! - 5-byte packet: [0xFA][theme][intensity][speed][checksum]
//! - checksum = (0xFA + theme + intensity + speed) mod 256
//! - Inter-byte delay: 5 ms between each byte

use serialport::SerialPort;
use std::time::Duration;

pub const LED_VID: u16 = 0x1A86;
pub const LED_PID: u16 = 0x7523;
const LED_BAUD: u32 = 10_000;

const MAGIC_HEADER: u8 = 0xFA;
const INTER_BYTE_MS: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum LedTheme {
    Rainbow = 1,
    Breathing = 2,
    Colors = 3,
    Off = 4,
    Auto = 5,
}

impl LedTheme {
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::Rainbow),
            2 => Some(Self::Breathing),
            3 => Some(Self::Colors),
            4 => Some(Self::Off),
            5 => Some(Self::Auto),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LedCommand {
    pub theme: u8,     // LedTheme as u8
    pub intensity: u8, // user-facing 1-5 scale
    pub speed: u8,     // user-facing 1-5 scale
}

/// Invert the user-facing 1–5 scale to the hardware wire value.
///
/// The original JS comment: "fix_value makes sure the LEDs work correct"
/// Hardware expects values 1–5 where 1 = fast/bright and 5 = slow/dim,
/// which is the opposite of the user-facing scale.
#[inline(always)]
fn fix_value(n: u8) -> u8 {
    (6u8.saturating_sub(n)).clamp(1, 5)
}

pub struct LedDevice {
    port: Box<dyn SerialPort>,
}

impl LedDevice {
    pub fn open(path: &str) -> crate::error::Result<Self> {
        let port = serialport::new(path, LED_BAUD)
            .timeout(Duration::from_millis(100))
            .open()?;
        Ok(Self { port })
    }

    /// Send a command to the LED strip.
    ///
    /// Bytes are sent one at a time with a 5 ms pause between each to match
    /// the timing the CH340 firmware expects at this non-standard baud rate.
    pub fn send(&mut self, cmd: LedCommand) -> crate::error::Result<()> {
        let theme = cmd.theme;
        let fi = if theme == LedTheme::Off as u8 {
            5
        } else {
            fix_value(cmd.intensity)
        };
        let fs = if theme == LedTheme::Off as u8 {
            5
        } else {
            fix_value(cmd.speed)
        };

        let checksum = MAGIC_HEADER
            .wrapping_add(theme)
            .wrapping_add(fi)
            .wrapping_add(fs);

        let packet = [MAGIC_HEADER, theme, fi, fs, checksum];
        for byte in &packet {
            self.port.write_all(&[*byte])?;
            std::thread::sleep(Duration::from_millis(INTER_BYTE_MS));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fix_value_inversion() {
        // User value 1 (slowest/dimmest for user) → hardware 5 (slowest/dimmest on wire)
        assert_eq!(fix_value(1), 5);
        // User value 5 (fastest/brightest for user) → hardware 1
        assert_eq!(fix_value(5), 1);
        // Middle
        assert_eq!(fix_value(3), 3);
    }

    #[test]
    fn fix_value_clamps() {
        assert_eq!(fix_value(0), 5); // 6-0=6, clamp → 5
        assert_eq!(fix_value(6), 1); // 6-6=0, clamp → 1
    }

    #[test]
    fn checksum_calculation() {
        // theme=2, fi=fix(3)=3, fs=fix(3)=3  →  0xFA+2+3+3 = 0x02 (256 mod 256 = 0, wrapping)
        let theme: u8 = 2;
        let fi: u8 = fix_value(3);
        let fs: u8 = fix_value(3);
        let checksum = MAGIC_HEADER
            .wrapping_add(theme)
            .wrapping_add(fi)
            .wrapping_add(fs);
        // 0xFA + 2 + 3 + 3 = 258 = 0x02
        assert_eq!(checksum, 2u8);
    }

    #[test]
    fn led_theme_roundtrip() {
        for n in 1u8..=5 {
            let t = LedTheme::from_u8(n).unwrap();
            assert_eq!(t as u8, n);
        }
        assert!(LedTheme::from_u8(0).is_none());
        assert!(LedTheme::from_u8(6).is_none());
    }
}
