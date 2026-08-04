//! Terminal-theme negotiation and the xterm-256 palette projection used by Kore.

use crate::sys;
use std::time::{Duration, Instant};

use crate::term::{C_BG, C_TEXT};

const QUERY: &[u8] = b"\x1b]10;?\x1b\\\x1b]11;?\x1b\\";
const PROBE_LIMIT: Duration = Duration::from_millis(120);
const MIN_CONTRAST: f64 = 4.5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Color {
    Default,
    Indexed(u8),
}

#[derive(Clone)]
pub struct Palette {
    map: [u8; 256],
    themed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Default)]
struct OscCollector {
    state: u8,
    frame: Vec<u8>,
    foreground: Option<Rgb>,
    background: Option<Rgb>,
    unclaimed: Vec<u8>,
}

impl Default for Palette {
    fn default() -> Self {
        let mut map = [0u8; 256];
        for (index, slot) in map.iter_mut().enumerate() {
            *slot = index as u8;
        }
        Self { map, themed: false }
    }
}

impl Palette {
    // Output: a palette derived from a complete, readable OSC 10/11 pair, or the exact legacy
    // palette on timeout, filtering, malformed replies, or inadequate contrast.
    pub fn probe() -> Self {
        let (reported, unclaimed) = query_with(
            |bytes| sys::write_stdout(bytes),
            |millis| sys::rbyte_raw(millis),
            PROBE_LIMIT,
        );
        sys::unread_input(&unclaimed);
        reported
            .and_then(|(foreground, background)| Self::from_reported(foreground, background))
            .unwrap_or_default()
    }

    pub fn foreground(&self, requested: u8) -> Color {
        if self.themed && (requested == 0 || requested == C_TEXT) {
            Color::Default
        } else {
            Color::Indexed(self.map[if requested == 0 { C_TEXT } else { requested } as usize])
        }
    }

    pub fn background(&self, requested: u8) -> Color {
        if self.themed && (requested == 0 || requested == C_BG) {
            Color::Default
        } else {
            Color::Indexed(self.map[if requested == 0 { C_BG } else { requested } as usize])
        }
    }

    // Inputs: terminal default foreground/background. Output: a readable indexed accent map;
    // None means the terminal's own defaults are not a safe canvas.
    fn from_reported(foreground: Rgb, background: Rgb) -> Option<Self> {
        if contrast(foreground, background) < MIN_CONTRAST {
            return None;
        }
        let mut palette = Self::default();
        palette.themed = true;
        for index in 0..=255u8 {
            let source = xterm_rgb(index);
            let target = readable_toward(source, foreground, background);
            palette.map[index as usize] = closest_readable(target, background);
        }
        palette.map[C_BG as usize] = closest_xterm(background);
        palette.map[C_TEXT as usize] = closest_readable(foreground, background);
        Some(palette)
    }
}

impl OscCollector {
    // Input: one byte from the terminal. Output: collected color state; non-OSC input is retained
    // for the ordinary event decoder, while malformed and unsolicited OSC frames are discarded.
    fn feed(&mut self, byte: u8) {
        match self.state {
            0 if byte == 0x1b => self.state = 1,
            0 if byte == 0x9d => {
                self.frame.clear();
                self.state = 2;
            }
            0 => self.unclaimed.push(byte),
            1 if byte == b']' => {
                self.frame.clear();
                self.state = 2;
            }
            1 if byte == 0x1b => self.unclaimed.push(0x1b),
            1 => {
                self.unclaimed.extend_from_slice(&[0x1b, byte]);
                self.state = 0;
            }
            2 if byte == 0x07 || byte == 0x9c => self.finish(),
            2 if byte == 0x1b => self.state = 3,
            2 => {
                if self.frame.len() < 128 {
                    self.frame.push(byte);
                } else {
                    self.frame.clear();
                    self.state = 4;
                }
            }
            3 if byte == b'\\' => self.finish(),
            3 => {
                self.frame.clear();
                self.state = if byte == 0x1b { 1 } else { 0 };
            }
            4 if byte == 0x07 || byte == 0x9c => self.state = 0,
            4 if byte == 0x1b => self.state = 5,
            5 if byte == b'\\' => self.state = 0,
            5 => self.state = if byte == 0x1b { 5 } else { 4 },
            _ => self.state = 0,
        }
    }

    fn finish(&mut self) {
        let mut fields = self.frame.split(|&byte| byte == b';');
        let Some(code) = fields.next().and_then(parse_decimal) else {
            self.frame.clear();
            self.state = 0;
            return;
        };
        for (offset, field) in fields.enumerate() {
            let Some(color) = parse_color(field) else {
                continue;
            };
            match code.checked_add(offset as u16) {
                Some(10) if self.foreground.is_none() => self.foreground = Some(color),
                Some(11) if self.background.is_none() => self.background = Some(color),
                _ => {}
            }
        }
        self.frame.clear();
        self.state = 0;
    }

    fn complete(&self) -> bool {
        self.foreground.is_some() && self.background.is_some()
    }

    fn release_pending(&mut self) {
        if self.state == 1 {
            self.unclaimed.push(0x1b);
        }
        self.frame.clear();
        self.state = 0;
    }
}

// Inputs: terminal write/read functions and a total deadline. Output: a complete reported pair
// plus ordinary input encountered during negotiation. The read function must honor its timeout.
fn query_with<W, R>(mut write: W, mut read: R, limit: Duration) -> (Option<(Rgb, Rgb)>, Vec<u8>)
where
    W: FnMut(&[u8]),
    R: FnMut(i32) -> i32,
{
    write(QUERY);
    let deadline = Instant::now() + limit;
    let mut collector = OscCollector::default();
    while !collector.complete() {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        let remaining = deadline
            .saturating_duration_since(now)
            .as_millis()
            .clamp(1, 20) as i32;
        let byte = read(remaining);
        if byte >= 0 {
            collector.feed(byte as u8);
        }
    }
    collector.release_pending();
    let pair = collector.foreground.zip(collector.background);
    (pair, collector.unclaimed)
}

fn parse_decimal(bytes: &[u8]) -> Option<u16> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    bytes.iter().try_fold(0u16, |value, byte| {
        value.checked_mul(10)?.checked_add((byte - b'0') as u16)
    })
}

fn parse_color(bytes: &[u8]) -> Option<Rgb> {
    if bytes.len() >= 4 && bytes[..4].eq_ignore_ascii_case(b"rgb:") {
        let mut parts = bytes[4..].split(|&byte| byte == b'/');
        let r = parse_hex_component(parts.next()?)?;
        let g = parse_hex_component(parts.next()?)?;
        let b = parse_hex_component(parts.next()?)?;
        return parts.next().is_none().then_some(Rgb { r, g, b });
    }
    if bytes.first() == Some(&b'#') {
        let digits = &bytes[1..];
        if !matches!(digits.len(), 3 | 6 | 9 | 12) {
            return None;
        }
        let width = digits.len() / 3;
        return Some(Rgb {
            r: parse_hex_component(&digits[..width])?,
            g: parse_hex_component(&digits[width..width * 2])?,
            b: parse_hex_component(&digits[width * 2..])?,
        });
    }
    None
}

fn parse_hex_component(bytes: &[u8]) -> Option<u8> {
    if bytes.is_empty() || bytes.len() > 4 {
        return None;
    }
    let mut value = 0u32;
    for byte in bytes {
        value = value.checked_mul(16)?.checked_add(byte.to_digit(16)?)?;
    }
    let maximum = 16u32.pow(bytes.len() as u32) - 1;
    Some(((value * 255 + maximum / 2) / maximum) as u8)
}

trait HexDigit {
    fn to_digit(self, radix: u32) -> Option<u32>;
}

impl HexDigit for &u8 {
    fn to_digit(self, radix: u32) -> Option<u32> {
        (*self as char).to_digit(radix)
    }
}

fn xterm_rgb(index: u8) -> Rgb {
    const ANSI: [Rgb; 16] = [
        Rgb { r: 0, g: 0, b: 0 },
        Rgb { r: 128, g: 0, b: 0 },
        Rgb { r: 0, g: 128, b: 0 },
        Rgb {
            r: 128,
            g: 128,
            b: 0,
        },
        Rgb { r: 0, g: 0, b: 128 },
        Rgb {
            r: 128,
            g: 0,
            b: 128,
        },
        Rgb {
            r: 0,
            g: 128,
            b: 128,
        },
        Rgb {
            r: 192,
            g: 192,
            b: 192,
        },
        Rgb {
            r: 128,
            g: 128,
            b: 128,
        },
        Rgb { r: 255, g: 0, b: 0 },
        Rgb { r: 0, g: 255, b: 0 },
        Rgb {
            r: 255,
            g: 255,
            b: 0,
        },
        Rgb { r: 0, g: 0, b: 255 },
        Rgb {
            r: 255,
            g: 0,
            b: 255,
        },
        Rgb {
            r: 0,
            g: 255,
            b: 255,
        },
        Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
    ];
    if index < 16 {
        return ANSI[index as usize];
    }
    if index < 232 {
        let value = index - 16;
        let component = |part: u8| if part == 0 { 0 } else { 55 + 40 * part };
        return Rgb {
            r: component(value / 36),
            g: component(value / 6 % 6),
            b: component(value % 6),
        };
    }
    let gray = 8 + 10 * (index - 232);
    Rgb {
        r: gray,
        g: gray,
        b: gray,
    }
}

fn linear(component: u8) -> f64 {
    let value = component as f64 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(color: Rgb) -> f64 {
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (light, dark) = if luminance(a) >= luminance(b) {
        (luminance(a), luminance(b))
    } else {
        (luminance(b), luminance(a))
    };
    (light + 0.05) / (dark + 0.05)
}

fn blend(a: Rgb, b: Rgb, step: u16, steps: u16) -> Rgb {
    let channel = |left: u8, right: u8| {
        ((left as u16 * (steps - step) + right as u16 * step + steps / 2) / steps) as u8
    };
    Rgb {
        r: channel(a.r, b.r),
        g: channel(a.g, b.g),
        b: channel(a.b, b.b),
    }
}

fn readable_toward(source: Rgb, foreground: Rgb, background: Rgb) -> Rgb {
    for step in 0..=16 {
        let candidate = blend(source, foreground, step, 16);
        if contrast(candidate, background) >= MIN_CONTRAST {
            return candidate;
        }
    }
    foreground
}

fn distance(a: Rgb, b: Rgb) -> u32 {
    let dr = a.r as i32 - b.r as i32;
    let dg = a.g as i32 - b.g as i32;
    let db = a.b as i32 - b.b as i32;
    (dr * dr + dg * dg + db * db) as u32
}

fn closest_xterm(target: Rgb) -> u8 {
    (16..=255u8)
        .min_by_key(|&index| distance(xterm_rgb(index), target))
        .unwrap_or(C_TEXT)
}

fn closest_readable(target: Rgb, background: Rgb) -> u8 {
    (16..=255u8)
        .filter(|&index| contrast(xterm_rgb(index), background) >= MIN_CONTRAST)
        .min_by_key(|&index| distance(xterm_rgb(index), target))
        .unwrap_or(C_TEXT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::pty::openpty;
    use nix::sys::termios::{SetArg, cfmakeraw, tcgetattr, tcsetattr};
    use nix::unistd::{read, write};
    use std::os::fd::AsFd;
    use std::thread;

    fn pseudo_terminal(reply: Option<&'static [u8]>) -> (Option<(Rgb, Rgb)>, Duration) {
        let pair = openpty(None, None).unwrap();
        let mut settings = tcgetattr(&pair.slave).unwrap();
        cfmakeraw(&mut settings);
        tcsetattr(&pair.slave, SetArg::TCSANOW, &settings).unwrap();
        let master = pair.master;
        let emulator = thread::spawn(move || {
            let mut query = [0u8; 64];
            let count = read(&master, &mut query).unwrap();
            assert_eq!(&query[..count], QUERY);
            if let Some(reply) = reply {
                let mut offset = 0;
                while offset < reply.len() {
                    offset += write(&master, &reply[offset..]).unwrap();
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        let start = Instant::now();
        let (reported, _) = query_with(
            |bytes| {
                let mut offset = 0;
                while offset < bytes.len() {
                    offset += write(&pair.slave, &bytes[offset..]).unwrap();
                }
            },
            |millis| sys::rbyte_from(pair.slave.as_fd(), millis),
            PROBE_LIMIT,
        );
        let elapsed = start.elapsed();
        emulator.join().unwrap();
        (reported, elapsed)
    }

    #[test]
    fn pseudo_terminal_accepts_complete_reply() {
        let (reported, _) =
            pseudo_terminal(Some(b"\x1b]10;rgb:eeee/ffff/dddd\x07\x1b]11;#102030\x1b\\"));
        assert_eq!(
            reported,
            Some((
                Rgb {
                    r: 238,
                    g: 255,
                    b: 221
                },
                Rgb {
                    r: 16,
                    g: 32,
                    b: 48
                }
            ))
        );
    }

    #[test]
    fn pseudo_terminal_timeout_is_bounded() {
        let (reported, elapsed) = pseudo_terminal(None);
        assert_eq!(reported, None);
        assert!(elapsed >= PROBE_LIMIT);
        assert!(elapsed < Duration::from_millis(350));
    }

    #[test]
    fn pseudo_terminal_ignores_malformed_and_unsolicited_replies() {
        let reply =
            b"\x1b]12;rgb:ffff/ffff/ffff\x07\x1b]10;rgb:nope/ffff/ffff\x07\x1b]11;#000000\x07";
        let (reported, _) = pseudo_terminal(Some(reply));
        assert_eq!(reported, None);
    }

    #[test]
    fn inadequate_contrast_selects_the_exact_fallback() {
        let fallback = Palette::default();
        let rejected = Palette::from_reported(
            Rgb {
                r: 120,
                g: 120,
                b: 120,
            },
            Rgb {
                r: 125,
                g: 125,
                b: 125,
            },
        );
        assert!(rejected.is_none());
        assert_eq!(fallback.foreground(0), Color::Indexed(C_TEXT));
        assert_eq!(fallback.background(0), Color::Indexed(C_BG));
    }

    #[test]
    fn ordinary_input_survives_negotiation() {
        let mut bytes = b"x\x1b[A\x1b]10;#ffffff\x07\x1b]11;#000000\x07"
            .iter()
            .copied();
        let (reported, unclaimed) = query_with(
            |_| {},
            |_| bytes.next().map(i32::from).unwrap_or(-1),
            Duration::from_millis(20),
        );
        assert!(reported.is_some());
        assert_eq!(unclaimed, b"x\x1b[A");
    }

    #[test]
    fn oversized_osc_code_is_ignored_without_overflow() {
        let mut bytes = b"\x1b]65535;#fff;#000\x07".iter().copied();
        let (reported, unclaimed) = query_with(
            |_| {},
            |_| bytes.next().map(i32::from).unwrap_or(-1),
            Duration::from_millis(2),
        );
        assert_eq!(reported, None);
        assert!(unclaimed.is_empty());
    }
}
