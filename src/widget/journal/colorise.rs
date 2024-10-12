//https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit

use std::{
    fmt::Debug, num::ParseIntError, sync::LazyLock
};

use log::{info, warn};
use regex::Regex;

use super::more_colors;

static RE: LazyLock<Regex> = LazyLock::new(|| {
    //https://stackoverflow.com/questions/14693701/how-can-i-remove-the-ansi-escape-sequences-from-a-string-in-python
    let re = match Regex::new(r"\x1B(?:[@-Z\\-_]|\[([0-?]*)[ -/]*([@-~]))") {
        Ok(ok) => ok,
        Err(e) => {
            log::error!("Rexgex compile error : {:?}", e);
            panic!()
        }
    };

    re
});

pub fn convert_to_mackup(text: &str) -> String {
    for (_, [select_graphic_rendition, control]) in RE.captures_iter(text).map(|c| c.extract()) {
        if control != "m" {
            continue;
        }

        match capture_code(select_graphic_rendition) {
            Ok(_) => todo!(),
            Err(e) => {
                warn!("while parsing {select_graphic_rendition} got error {:?}", e)
            }
        };
    }

    let s = String::from(text);
    s
}


enum ColorCodeError {
    Malformed,
    ParseIntError(ParseIntError),
    UnexpectedCode(String),
}

impl Debug for ColorCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => write!(f, "Malformed"),
            Self::ParseIntError(parseint_error) => f.debug_tuple("ParseIntError").field(parseint_error).finish(),
            Self::UnexpectedCode(unxecpected_code) => f.debug_tuple("UnexpectedCode").field(unxecpected_code).finish(),
        }
    }
}

impl From<ParseIntError> for ColorCodeError {
    fn from(pe: ParseIntError) -> Self {
        ColorCodeError::ParseIntError(pe)
    }
}

fn capture_code(code_line: &str) -> Result<(), ColorCodeError> {
    let mut it = code_line.split(';');

    let mut sgc = SelectGraphicRendition::default();
    while let Some(code) = it.next() {
        match code {
            "0" => sgc.reset(),
            "1" => sgc.set_intensity(Intensity::Bold),
            "2" => sgc.set_intensity(Intensity::Faint),
            "3" => sgc.set_italic(true),
            "4" => sgc.set_underline(true),
            "5" => info!("blink slow"),
            "6" => info!("blink fast"),
            "7" => sgc.set_reversed(true),
            "8" => sgc.set_hidden(true),
            "9" => sgc.set_strikeout(true),
            "10" => {}
            "11" => {}
            "20" => {}
            "22" => sgc.set_intensity(Intensity::Normal),
            "28" => sgc.set_hidden(false),
            "21" => {}
            "30" => sgc.set_foreground_color(TermColor::Black),
            "31" => sgc.set_foreground_color(TermColor::Red),
            "32" => sgc.set_foreground_color(TermColor::Green),
            "33" => sgc.set_foreground_color(TermColor::Yellow),
            "34" => sgc.set_foreground_color(TermColor::Blue),
            "35" => sgc.set_foreground_color(TermColor::Magenta),
            "36" => sgc.set_foreground_color(TermColor::Cyan),
            "37" => sgc.set_foreground_color(TermColor::White),
            "38" => {
                let color = find_color(&mut it)?;
                sgc.set_foreground_color(color)
            }
            "39" => sgc.set_foreground_color_default(),

            "40" => sgc.set_background_color(TermColor::Black),
            "41" => sgc.set_background_color(TermColor::Red),
            "42" => sgc.set_background_color(TermColor::Green),
            "43" => sgc.set_background_color(TermColor::Yellow),
            "44" => sgc.set_background_color(TermColor::Blue),
            "45" => sgc.set_background_color(TermColor::Magenta),
            "46" => sgc.set_background_color(TermColor::Cyan),
            "47" => sgc.set_background_color(TermColor::White),
            "48" => {
                let color = find_color(&mut it)?;
                sgc.set_background_color(color)
            }
            "49" => sgc.set_background_color_default(),
            "90" => sgc.set_foreground_color(TermColor::BrightBlack),
            "91" => sgc.set_foreground_color(TermColor::BrightRed),
            "92" => sgc.set_foreground_color(TermColor::BrightGreen),
            "93" => sgc.set_foreground_color(TermColor::BrightYellow),
            "94" => sgc.set_foreground_color(TermColor::BrightBlue),
            "95" => sgc.set_foreground_color(TermColor::BrightMagenta),
            "96" => sgc.set_foreground_color(TermColor::BrightCyan),
            "97" => sgc.set_foreground_color(TermColor::BrightWhite),

            "100" => sgc.set_background_color(TermColor::BrightBlack),
            "101" => sgc.set_background_color(TermColor::BrightRed),
            "102" => sgc.set_background_color(TermColor::BrightGreen),
            "103" => sgc.set_background_color(TermColor::BrightYellow),
            "104" => sgc.set_background_color(TermColor::BrightBlue),
            "105" => sgc.set_background_color(TermColor::BrightMagenta),
            "106" => sgc.set_background_color(TermColor::BrightCyan),
            "107" => sgc.set_background_color(TermColor::BrightWhite),
            _ => {}
        };
    }
    Ok(())
}

fn find_color(it: &mut std::str::Split<'_, char>) -> Result<TermColor, ColorCodeError> {
    let Some(sub_code) = it.next() else {
        return Err(ColorCodeError::Malformed);
    };
    let color = match sub_code {
        "5" => {
            if let Some(color_code) = it.next() {
                let color_code_u8 = color_code.parse::<u8>()?;
                more_colors::get_256color(color_code_u8)
            } else {
                return Err(ColorCodeError::Malformed);
            }
        }
        "2" => {
            let Some(r) = it.next() else {
                return Err(ColorCodeError::Malformed);
            };

            let Some(g) = it.next() else {
                return Err(ColorCodeError::Malformed);
            };

            let Some(b) = it.next() else {
                return Err(ColorCodeError::Malformed);
            };

            TermColor::new_vga(r, g, b)?
        }
        unexpected_code => {
            return Err(ColorCodeError::UnexpectedCode(unexpected_code.to_owned()));
        }
    };
    Ok(color)
}

#[derive(Default)]
struct SelectGraphicRendition {
    foreground_color: Option<TermColor>,
    background_color: Option<TermColor>,
    intensity: Option<Intensity>,
    italic: Option<bool>,
    underline: Option<bool>,
    //blink: Option<Blink>,
    reversed: Option<bool>,
    hidden: Option<bool>,
    strikeout: Option<bool>,
}

impl SelectGraphicRendition {

    fn reset(&mut self) {
        self.foreground_color = None;
        self.background_color = None;
        self.intensity = None;
        self.italic = None;
        self.underline = None;
        self.reversed = None;
        self.hidden = None;
        self.strikeout = None;
    }

    fn set_intensity(&mut self, intensity: Intensity) {
        self.intensity = Some(intensity);
    }

    fn set_italic(&mut self, italic: bool) {
        self.italic = Some(italic);
    }

    fn set_underline(&mut self, underline: bool) {
        self.underline = Some(underline);
    }

    fn set_reversed(&mut self, reversed: bool) {
        self.reversed = Some(reversed);
    }
    fn set_strikeout(&mut self, strikeout: bool) {
        self.strikeout = Some(strikeout);
    }

    fn set_hidden(&mut self, hidden: bool) {
        self.hidden = Some(hidden);
    }

    fn set_foreground_color(&mut self, color: TermColor) {
        self.foreground_color = Some(color);
    }

    fn set_foreground_color_default(&mut self) {
        self.foreground_color = None;
    }

    fn set_background_color(&mut self, color: TermColor) {
        self.background_color = Some(color);
    }

    fn set_background_color_default(&mut self) {
        self.background_color = None;
    }
}

/// The emphasis (bold, faint) states.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Intensity {
    /// Normal intensity (no emphasis).
    Normal,
    /// Bold.
    Bold,
    /// Faint.
    Faint,
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

    fn new_vga(r: &str, g: &str, b: &str) -> Result<Self, ColorCodeError> {
        let r: u8 = r.parse()?;
        let g: u8 = g.parse()?;
        let b: u8 = b.parse()?;

        Ok(TermColor::VGA(r, g, b))
    }
}

#[cfg(test)]
mod tests {
    use ansi_parser::AnsiSequence;
    use ansi_parser::{AnsiParser, Output};

    extern crate cansi;
    extern crate colored;
    use super::*;
    use cansi::*;

    const TEST_STRS : [&str; 4] = [  "This is \u{1b}[4mvery\u{1b}[0m\u{1b}[1m\u{1b}[96m Important\u{1b}[0m",
    "asdf \u{1b}[38;2;255;140;0;48;2;255;228;225mExample 24 bit color escape sequence\u{1b}[0m",
    "0:13:37 fedora abrt-server[90694]: \u{1b}[0;1;38;5;185m\u{1b}[0;1;39m\u{1b}[0;1;38;5;185m'post-create' on '/var/spool/abrt/ccpp-2024-10-08-10:13:37.85581-16875' exited with 1\u{1b}[0m",
    "nothing \u{1b}[91mframed\u{1b}[7m test ok\u{1b}[0m"];

    #[test]
    fn test_enable_unit_files_path() {
        let parsed: Vec<Output> = "This is \u{1b}[3Asome text!".ansi_parse().take(2).collect();

        assert_eq!(
            vec![
                Output::TextBlock("This is "),
                Output::Escape(AnsiSequence::CursorUp(3))
            ],
            parsed
        );

        for block in parsed.into_iter() {
            match block {
                Output::TextBlock(text) => println!("{}", text),
                Output::Escape(seq) => println!("{}", seq),
            }
        }
    }

    #[test]
    fn test_display() {
        let mut line = 0;
        for s in TEST_STRS {
            println!("line {} {}", line, s);
            line += 1;
        }
    }

    #[test]
    fn test_color() {
        for s in TEST_STRS {
            println!("{}", s);
            let parsed: Vec<Output> = s.ansi_parse().collect();

            println!("{:?}", parsed);
        }
    }

    #[test]
    fn test_color1() {
        for s in TEST_STRS {
            println!("{}", s);

            let result = v3::categorise_text(s); // cansi function

            println!("{:#?}", result);
        }
    }

    #[test]
    fn test_color_regex() {
        let mut results = vec![];

        let mut line: usize = 0;
        for haystack in TEST_STRS {
            for capt in RE.captures_iter(haystack) {
                results.push((line, capt));
            }
            line += 1;
        }

        /*        for (text, [ select_graphic_rendition,control]) in RE.captures_iter(text).map(|c| c.extract()) {

        } */

        for capt in results {
            println!("line {} capture: {:#?}", capt.0, capt.1)
        }
    }

    #[test]
    fn test_capture_code() {
        let _res = capture_code("0;1;38;5;185");

        // println!("{:?}", res)
    }
}
