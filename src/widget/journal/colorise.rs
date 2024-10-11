#![allow(dead_code)]

enum ColorCodeError {
    Malformed,
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
            "5" => sgc.set_blink(Blink::Slow),
            "6" => sgc.set_blink(Blink::Fast),
            "7" => sgc.set_reversed(true),
            "8" => sgc.set_hidden(true),
            "9" => sgc.set_strikeout(true),
            "10" => {}
            "11" => {}
            "20" => {}
            "28" => sgc.set_hidden(false),
            "21" => {}
            "30" => sgc.set_foreground_color(Color::Black),
            "31" => sgc.set_foreground_color(Color::Red),
            "32" => sgc.set_foreground_color(Color::Green),
            "33" => sgc.set_foreground_color(Color::Yellow),
            "34" => sgc.set_foreground_color(Color::Blue),
            "35" => sgc.set_foreground_color(Color::Magenta),
            "36" => sgc.set_foreground_color(Color::Cyan),
            "37" => sgc.set_foreground_color(Color::White),
            "38" => {
                if let Some(sub_code) = it.next() {
                    match sub_code {
                        "5" => {}
                        "2" => {}
                        _ => {
                            return Err(ColorCodeError::Malformed);
                        }
                    };
                } else {
                    return Err(ColorCodeError::Malformed);
                }
            }
            "39" => sgc.set_foreground_color(Color::Default),

            "40" => sgc.set_background_color(Color::Black),
            "41" => sgc.set_background_color(Color::Red),
            "42" => sgc.set_background_color(Color::Green),
            "43" => sgc.set_background_color(Color::Yellow),
            "44" => sgc.set_background_color(Color::Blue),
            "45" => sgc.set_background_color(Color::Magenta),
            "46" => sgc.set_background_color(Color::Cyan),
            "47" => sgc.set_background_color(Color::White),

            "90" => sgc.set_foreground_color(Color::BrightBlack),
            "91" => sgc.set_foreground_color(Color::BrightRed),
            "92" => sgc.set_foreground_color(Color::BrightGreen),
            "93" => sgc.set_foreground_color(Color::BrightYellow),
            "94" => sgc.set_foreground_color(Color::BrightBlue),
            "95" => sgc.set_foreground_color(Color::BrightMagenta),
            "96" => sgc.set_foreground_color(Color::BrightCyan),
            "97" => sgc.set_foreground_color(Color::BrightWhite),

            "100" => sgc.set_background_color(Color::BrightBlack),
            "101" => sgc.set_background_color(Color::BrightRed),
            "102" => sgc.set_background_color(Color::BrightGreen),
            "103" => sgc.set_background_color(Color::BrightYellow),
            "104" => sgc.set_background_color(Color::BrightBlue),
            "105" => sgc.set_background_color(Color::BrightMagenta),
            "106" => sgc.set_background_color(Color::BrightCyan),
            "107" => sgc.set_background_color(Color::BrightWhite),
            _ => {}
        };
    }
    Ok(())
}

#[derive(Default)]
struct SelectGraphicRendition {
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    intensity: Option<Intensity>,
    italic: Option<bool>,
    underline: Option<bool>,
    blink: Option<Blink>,
    reversed: Option<bool>,
    hidden: Option<bool>,
    strikeout: Option<bool>,
}

impl SelectGraphicRendition {
    fn new() -> Self {
        SelectGraphicRendition::default()
    }

    fn set_bold(&mut self) {
        self.intensity = Some(Intensity::Bold)
    }

    fn reset(&mut self) {
        self.foreground_color = None;
        self.background_color = None;
        self.intensity = None;
        self.italic = None;
        self.underline = None;
        self.blink = None;
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

    fn set_blink(&mut self, blink: Blink) {
        self.blink = Some(blink)
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

    fn set_foreground_color(&mut self, color: Color) {
        self.foreground_color = Some(color);
    }

    fn set_background_color(&mut self, color: Color) {
        self.background_color = Some(color);
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

pub enum Blink {
    Slow,
    Fast,
}

/// The 8 standard colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
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
    Default,
}

#[cfg(test)]
mod tests {
    use ansi_parser::AnsiSequence;
    use ansi_parser::{AnsiParser, Output};

    extern crate cansi;
    extern crate colored;
    use super::*;
    use cansi::*;
    use regex::Regex;

    const CYAN: &str = "\u{1b}[96m";
    const DARKCYAN: &str = "\u{1b}[36m";
    const BLUE: &str = "\u{1b}[94m";
    const GREEN: &str = "\u{1b}[92m";
    const YELLOW: &str = "\u{1b}[93m";
    const RED: &str = "\u{1b}[91m";
    const BOLD: &str = "\u{1b}[1m";
    const UNDERLINE: &str = "\u{1b}[4m";
    const END: &str = "\u{1b}[0m";

    const TEST_STRS : [&str; 4] = [  "This is \u{1b}[4mvery\u{1b}[0m\u{1b}[1m\u{1b}[96m Important\u{1b}[0m",
    "asdf \u{1b}[38;2;255;140;0;48;2;255;228;225mExample 24 bit color escape sequence\u{1b}[0m",
    "0:13:37 fedora abrt-server[90694]: \u{1b}[0;1;38;5;185m\u{1b}[0;1;39m\u{1b}[0;1;38;5;185m'post-create' on '/var/spool/abrt/ccpp-2024-10-08-10:13:37.85581-16875' exited with 1\u{1b}[0m",
    "\u{1b}[91mframed\u{1b}[7m test ok[0m"];

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
            let parsed: Vec<Output> = s.ansi_parse().collect();

            println!("{:?}", parsed);

            let result = v3::categorise_text(s); // cansi function

            println!("{:#?}", result);
        }
    }

    #[test]
    fn test_color_regex() {
        //https://stackoverflow.com/questions/14693701/how-can-i-remove-the-ansi-escape-sequences-from-a-string-in-python
        let re = match Regex::new(r"\x1B(?:[@-Z\\-_]|\[([0-?]*)[ -/]*([@-~]))") {
            Ok(ok) => ok,
            Err(e) => {
                log::error!("Rexgex compile error : {:?}", e);
                return;
            }
        };

        let mut results = vec![];

        let mut line: usize = 0;
        for haystack in TEST_STRS {
            for capt in re.captures_iter(haystack) {
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
        let _res = capture_code("0;1;38;5;185");

        // println!("{:?}", res)
    }
}
