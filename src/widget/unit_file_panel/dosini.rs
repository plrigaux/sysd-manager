/// follows https://github.com/xuhdev/syntax-dosini.vim/blob/master/syntax/dosini.vim
///
use std::{borrow::Cow, fmt::Debug, sync::LazyLock};

use crate::widget::journal::more_colors::Intensity;
use regex::Regex;

use super::palette::Palette;

static RE: LazyLock<Regex> = LazyLock::new(|| {
    let re = match Regex::new(
        r"(?xm)
            (?:  
                (^\w+\s*)=                            # Label
                (:?  
                    (                                 # Number
                        \s*\d+\s*                             
                    |                 
                        \s*\d*\.\d+\s*  
                    |
                        \s*[+-]?\d+\s* 
                    )
                |
                    (.*)                              # Value    
                )$
            |
                (^\s*\[\w+\])$                        # Section                  
            |
                (^[\#;].*)$                           # Comment    
            )",
    ) {
        Ok(ok) => ok,
        Err(e) => {
            log::error!("Rexgex compile error : {:?}", e);
            panic!("Rexgex compile error : {:?}", e)
        }
    };

    re
});

macro_rules! colorize_str {
    (  $text:expr, $token:expr, $dark:expr, $sbuilder:expr) => {{
        let style = $token.get_style($dark);

        $sbuilder.push_str("<span color=\"");
        $sbuilder.push_str(style.color.get_color());
        $sbuilder.push_str("\"");
        if let Some(intensity) = style.intensity {
            $sbuilder.push_str(" weight=\"");
            $sbuilder.push_str(intensity.pango());
            $sbuilder.push('\"');
        } 
        $sbuilder.push('>');
        $sbuilder.push_str($text);
        $sbuilder.push_str("</span>");
    }};
}

macro_rules! colorize {
    ($text:expr, $token:expr, $dark:expr, $sbuilder:expr) => {{
        colorize_str!($text.as_str(), $token, $dark, $sbuilder)
    }};
}

// echo "\x1b[35;47mANSI? \x1b[0m\x1b[1;32mSI\x1b[0m \x1b]8;;man:abrt(1)\x1b\\[🡕]\x1b]8;;\x1b\\ test \x1b[0m"
pub fn convert_to_mackup<'a>(text: &'a str, dark: bool) -> Cow<'a, str> {
    let mut last_end: usize = 0;

    let mut out = String::with_capacity(text.len() * 2);

    for captures in RE.captures_iter(text) {
        let main_match = captures.get(0).expect("not suposed to happen");

        let end = main_match.end();
        let start = main_match.start();

        if start != last_end {
            colorize_str!(&text[last_end..start], Token::Text, dark, out);
        }

        if let Some(label) = captures.get(1) {
            colorize!(label, Token::Label, dark, out);
            out.push('=');

            if let Some(number) = captures.get(3) {
                colorize!(number, Token::Number, dark, out);
            } else if let Some(value) = captures.get(4) {
                colorize!(value, Token::Value, dark, out);
            }
        } else if let Some(section) = captures.get(5) {
            colorize!(section, Token::Section, dark, out);
        } else if let Some(comment) = captures.get(6) {
            colorize!(comment, Token::Comment, dark, out);
        }

        last_end = end;
    }

    if last_end == 0 {
        return Cow::from(text);
    }

    out.push_str(&text[last_end..]);

    Cow::from(out)
}

#[derive(Debug)]
enum Token {
    Text,
    Label,
    Value,
    Number,
    Comment,
    Section,
}

#[derive(Debug)]
struct Style<'a> {
    color: Palette<'a>,
    intensity: Option<Intensity>,
}

impl<'a> Style<'a> {
    fn new(color: Palette<'a>, intensity: Option<Intensity>) -> Style<'a> {
        Self { color, intensity }
    }
}

impl Token {
    fn get_style(&self, dark: bool) -> Style {
        let style = match self {
            Token::Text => {
                if dark {
                    Style::new(Palette::Light5, None)
                } else {
                    Style::new(Palette::Dark5, None)
                }
            }
            Token::Label => {
                if dark {
                    Style::new(Palette::Custom("#5bc8af"), Some(Intensity::Bold))
                } else {
                    Style::new(Palette::Custom("#218787"), Some(Intensity::Bold))
                }
            }
            Token::Value => {
                if dark {
                    Style::new(Palette::Light4, None)
                } else {
                    Style::new(Palette::Custom("#504e55"), None)
                }
            }
            Token::Number => {
                if dark {
                    Style::new(Palette::Custom("#7d8ac7"), None)
                } else {
                    Style::new(Palette::Custom("#4e57ba"), None)
                }
            }
            Token::Comment => {
                if dark {
                    Style::new(Palette::Dark1, None)
                } else {
                    Style::new(Palette::Dark1, None)
                }
            }
            Token::Section => {
                if dark {
                    Style::new(Palette::Orange2, Some(Intensity::Bold))
                } else {
                    Style::new(Palette::Orange5, Some(Intensity::Bold))
                }
            }
        };
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_INI_FILE: &str = ";Comment line 1
# Comment line 2
[Unit]
Description=It is tiny, but is not the tiniest
After=network.target

#Comment line 3
[Service]
SyslogIdentifier=tiny_daemon #dfgsdfgdsfg
Restart=always
RestartSec=5
Type=simple
User=pier
Group=pier
WorkingDirectory=/home/pier/bin
ExecStart= \"/home/pier/bin/tiny_daemon\" --port 33001
TimeoutStopSec=30
some text

[Install]
WantedBy=multi-user.target
test=some=weird=text \"in quote\"
number1=2
number2=3.1416
number3=-4

some text

";

    #[test]
    fn test_color_regex() {
        let mut results = vec![];

        for capt in RE.captures_iter(TEST_INI_FILE) {
            results.push(capt);
        }
        //println!("capture len: {}",TEST_INI_FILE);
        println!("capture len: {:#?}", results.len());

        for capt in results {
            println!("capture: {:#?}", capt)
        }
    }

    #[test]
    fn test_color_convert() {
        let converted_text = convert_to_mackup(TEST_INI_FILE, true);
        //println!("capture len: {}",TEST_INI_FILE);
        println!("{}", converted_text);
    }
}