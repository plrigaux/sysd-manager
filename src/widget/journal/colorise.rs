//https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit

use std::{fmt::Debug, num::ParseIntError, sync::LazyLock};

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
    let token_list = get_tokens(text);

    let s = make_markup(text, &token_list);
    s
}

fn get_tokens(text: &str) -> Vec<Token> {
    let mut token_list = Vec::<Token>::new();
    let mut last_end: usize = 0;

    for captures in RE.captures_iter(text) {
        let main_match = captures.get(0).expect("not supose to happen");
        let end = main_match.end();
        let start = main_match.start();

        if start != last_end {
            token_list.push(Token::Text(last_end, start));
        }
        last_end = end;

        let control = captures.get(2).map_or("", |m| m.as_str());
        if control != "m" {
            token_list.push(Token::UnHandled(main_match.as_str().to_owned()));
            continue;
        }

        if let Some(select_graphic_rendition_match) = captures.get(1) {
            let select_graphic_rendition = select_graphic_rendition_match.as_str();
            match capture_code(select_graphic_rendition, &mut token_list) {
                Ok(_) => {}
                Err(e) => {
                    warn!("while parsing {select_graphic_rendition} got error {:?}", e)
                }
            };
        }
    }

    if text.len() != last_end {
        token_list.push(Token::Text(last_end, text.len()));
    }
    token_list
}

fn make_markup(text: &str, token_list: &Vec<Token>) -> String {
    let mut out = String::with_capacity((text.len() as f32 * 1.5) as usize);

    for token in token_list {
        match token {
            Token::Text(begin, end) => out.push_str(&text[*begin..*end]),
            _ => {}
        }
    }

    out
}

#[derive(Debug)]
enum ColorCodeError {
    Malformed,
    ParseIntError(ParseIntError),
    UnexpectedCode(String),
}

impl From<ParseIntError> for ColorCodeError {
    fn from(pe: ParseIntError) -> Self {
        ColorCodeError::ParseIntError(pe)
    }
}

fn capture_code(code_line: &str, vec: &mut Vec<Token>) -> Result<(), ColorCodeError> {
    let mut it = code_line.split(';');

    while let Some(code) = it.next() {
        let token = match code {
            "0" => Token::Reset(ResetType::All),
            "1" => Token::Intensity(Intensity::Bold),
            "2" => Token::Intensity(Intensity::Faint),
            "3" => Token::Italic,
            "4" => Token::Underline,
            "5" => Token::Blink,
            "6" => Token::Blink,
            "7" => Token::Reversed,
            "8" => Token::Hidden,
            "9" => Token::Strikeout,
            "22" => Token::Reset(ResetType::Intensity),
            "28" => Token::Reset(ResetType::Hidden),
            "21" => Token::DoubleUnderline,
            "30" => Token::FgColor(TermColor::Black),
            "31" => Token::FgColor(TermColor::Red),
            "32" => Token::FgColor(TermColor::Green),
            "33" => Token::FgColor(TermColor::Yellow),
            "34" => Token::FgColor(TermColor::Blue),
            "35" => Token::FgColor(TermColor::Magenta),
            "36" => Token::FgColor(TermColor::Cyan),
            "37" => Token::FgColor(TermColor::White),
            "38" => {
                let color = find_color(&mut it)?;
                Token::FgColor(color)
            }
            "39" => Token::Reset(ResetType::FgColor),

            "40" => Token::BgColor(TermColor::Black),
            "41" => Token::BgColor(TermColor::Red),
            "42" => Token::BgColor(TermColor::Green),
            "43" => Token::BgColor(TermColor::Yellow),
            "44" => Token::BgColor(TermColor::Blue),
            "45" => Token::BgColor(TermColor::Magenta),
            "46" => Token::BgColor(TermColor::Cyan),
            "47" => Token::BgColor(TermColor::White),
            "48" => {
                let color = find_color(&mut it)?;
                Token::BgColor(color)
            }
            "49" => Token::Reset(ResetType::BgColor),
            "90" => Token::FgColor(TermColor::BrightBlack),
            "91" => Token::FgColor(TermColor::BrightRed),
            "92" => Token::FgColor(TermColor::BrightGreen),
            "93" => Token::FgColor(TermColor::BrightYellow),
            "94" => Token::FgColor(TermColor::BrightBlue),
            "95" => Token::FgColor(TermColor::BrightMagenta),
            "96" => Token::FgColor(TermColor::BrightCyan),
            "97" => Token::FgColor(TermColor::BrightWhite),

            "100" => Token::BgColor(TermColor::BrightBlack),
            "101" => Token::BgColor(TermColor::BrightRed),
            "102" => Token::BgColor(TermColor::BrightGreen),
            "103" => Token::BgColor(TermColor::BrightYellow),
            "104" => Token::BgColor(TermColor::BrightBlue),
            "105" => Token::BgColor(TermColor::BrightMagenta),
            "106" => Token::BgColor(TermColor::BrightCyan),
            "107" => Token::BgColor(TermColor::BrightWhite),
            unknown_code => Token::UnHandledCode(unknown_code.to_string()),
        };

        vec.push(token)
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

#[derive(Debug)]
enum Token {
    FgColor(TermColor),
    BgColor(TermColor),
    Intensity(Intensity),
    Italic,
    Underline,
    Blink,
    Reversed,
    Hidden,
    Strikeout,
    Text(usize, usize),
    Reset(ResetType),
    DoubleUnderline,
    UnHandledCode(String),
    UnHandled(String),
}

#[derive(Debug)]
enum ResetType {
    All,
    FgColor,
    BgColor,
    Intensity,
    Hidden,
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
    fn test_tokens() {
        for s in TEST_STRS {
            println!("{}", s);

            let result = get_tokens(s);

            println!("{:?}", result);
        }
    }

    #[test]
    fn test_full() {
        for s in TEST_STRS {
            println!("{}", s);

            let result = convert_to_mackup(s);

            println!("{}", result);
        }
    }

    #[test]
    fn test_reverse() {
        let s = "reverse test \u{1b}[7m reverse test \u{1b}[0;m test test \u{1b}[97mwhite\u{1b}[0m";
        println!("{}", s);

        let result = convert_to_mackup(s);

        println!("{}", result);
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

        for capt in results {
            println!("line {} capture: {:#?}", capt.0, capt.1)
        }
    }

    #[test]
    fn test_capture_code() {
        let mut vec = Vec::<Token>::new();
        let _res = capture_code("0;1;38;5;185", &mut vec);

        println!("{:?}", vec)
    }
}
