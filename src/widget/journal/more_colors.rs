#![allow(dead_code)]
use std::num::ParseIntError;

use gtk::gdk;

pub fn get_256color(code: u8) -> TermColor {
    let color = match code {
        0 => TermColor::Black,
        1 => TermColor::Red,
        2 => TermColor::Green,
        3 => TermColor::Yellow,
        4 => TermColor::Blue,
        5 => TermColor::Magenta,
        6 => TermColor::Cyan,
        7 => TermColor::White,
        8 => TermColor::BrightBlack,
        9 => TermColor::BrightRed,
        10 => TermColor::BrightGreen,
        11 => TermColor::BrightYellow,
        12 => TermColor::BrightBlue,
        13 => TermColor::BrightMagenta,
        14 => TermColor::BrightCyan,
        15 => TermColor::BrightWhite,
        16..=231 => color_map216(code),
        232..=255 => grayscale(code),
    };

    color
}

fn grayscale(code: u8) -> TermColor {
    let gray_scale: u8 = (code - 232) * 10 + 8;

    TermColor::VGA(gray_scale, gray_scale, gray_scale)
}

fn color_map216(code: u8) -> TermColor {
    let base = code - 16;

    let mut r: u8 = (base / 36) * 40;
    if r > 0 {
        r += 55
    }

    let mut g: u8 = ((base % 36) / 6) * 40;
    if g > 0 {
        g += 55
    }

    let mut b: u8 = (base % 6) * 40;

    if b > 0 {
        b += 55
    }

    TermColor::VGA(r, g, b)
}

/// The 8 standard colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    VGA(u8, u8, u8),
}

impl TermColor {
    pub fn get_hexa_code(&self) -> String {
        match self {
            Self::VGA(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
            _ => self.get_vga().get_hexa_code(),
        }
    }

    pub fn get_vga(&self) -> Self {
        match self {
            TermColor::Black => Self::VGA(0, 0, 0),
            TermColor::Red => Self::VGA(0x80, 0, 0),
            TermColor::Green => Self::VGA(0, 0x80, 0),
            TermColor::Yellow => Self::VGA(0x80, 0x80, 0),
            TermColor::Blue => Self::VGA(0, 0, 0x80),
            TermColor::Magenta => Self::VGA(0x80, 0, 0x80),
            TermColor::Cyan => Self::VGA(0, 0x80, 0x80),
            TermColor::White => Self::VGA(0xc0, 0xc0, 0xc0),
            TermColor::BrightBlack => Self::VGA(0x80, 0x80, 0x80),
            TermColor::BrightRed => Self::VGA(0xff, 0, 0),
            TermColor::BrightGreen => Self::VGA(0, 0xff, 0),
            TermColor::BrightYellow => Self::VGA(0xff, 0xff, 0),
            TermColor::BrightBlue => Self::VGA(0, 0, 0xff),
            TermColor::BrightMagenta => Self::VGA(0xff, 0, 0xff),
            TermColor::BrightCyan => Self::VGA(0, 0xff, 0xff),
            TermColor::BrightWhite => Self::VGA(0xff, 0xff, 0xff),
            TermColor::VGA(_, _, _) => *self,
        }
    }

    pub fn new_vga(r: &str, g: &str, b: &str) -> Result<Self, ColorCodeError> {
        let r: u8 = r.parse()?;
        let g: u8 = g.parse()?;
        let b: u8 = b.parse()?;

        Ok(TermColor::VGA(r, g, b))
    }
}

impl From<gdk::RGBA> for TermColor {
    fn from(color: gdk::RGBA) -> Self {
        TermColor::from(&color)
    }
}

impl From<&gdk::RGBA> for TermColor {
    fn from(color: &gdk::RGBA) -> Self {
        let r: u8 = (color.red() * 256.0) as u8;
        let g: u8 = (color.green() * 256.0) as u8;
        let b: u8 = (color.blue() * 256.0) as u8;
        TermColor::VGA(r, g, b)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum ColorCodeError {
    Malformed,
    ParseIntError(ParseIntError),
    UnexpectedCode(String),
}

impl From<ParseIntError> for ColorCodeError {
    fn from(pe: ParseIntError) -> Self {
        ColorCodeError::ParseIntError(pe)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_color_map() {
        let serie = [
            (16, "#000000"),
            (52, "#5f0000"),
            (160, "#d70000"),
            (196, "#ff0000"),
            (22, "#005f00"),
            (46, "#00ff00"),
            (17, "#00005f"),
            (146, "#afafd7"),
        ];

        for elem in serie {
            let color = color_map216(elem.0);
            let hex_val = color.get_hexa_code();

            assert_eq!(hex_val, elem.1, "code {}", elem.0)
        }
    }

    #[test]
    fn test_grayscale() {
        let serie = [
            (232, "#080808"),
            (233, "#121212"),
            (254, "#e4e4e4"),
            (255, "#eeeeee"),
        ];

        for elem in serie {
            let color = grayscale(elem.0);
            let hex_val = color.get_hexa_code();

            assert_eq!(hex_val, elem.1, "code {}", elem.0)
        }
    }
}
