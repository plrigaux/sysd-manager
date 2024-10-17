/// follows https://github.com/xuhdev/syntax-dosini.vim/blob/master/syntax/dosini.vim
///
use std::{borrow::Cow, fmt::Debug, sync::LazyLock};

use crate::widget::journal::more_colors::TermColor;
use regex::Regex;

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

// echo "\x1b[35;47mANSI? \x1b[0m\x1b[1;32mSI\x1b[0m \x1b]8;;man:abrt(1)\x1b\\[🡕]\x1b]8;;\x1b\\ test \x1b[0m"
pub fn convert_to_mackup<'a>(text: &'a str, dark: bool) -> Cow<'a, str> {
    let token_list = get_tokens(text);

    make_markup(text, &token_list, dark)
}

fn get_tokens(text: &str) -> Vec<Token> {
    let mut token_list = Vec::<Token>::new();
    let mut last_end: usize = 0;

    for captures in RE.captures_iter(text) {
        let main_match = captures.get(0).expect("not suposed to happen");

        let end = main_match.end();
        let start = main_match.start();

        if start != last_end {
            token_list.push(Token::Text(&text[last_end..start]));
        }

        if let Some(label) = captures.get(1) {
            token_list.push(Token::Label(&label.as_str()));

            if let Some(number) = captures.get(3) {
                token_list.push(Token::Number(&number.as_str()));
            } else if let Some(value) = captures.get(4) {
                token_list.push(Token::Value(&value.as_str()));
            }
        } else if let Some(section) = captures.get(5) {
            token_list.push(Token::Section(&section.as_str()));
        } else if let Some(comment) = captures.get(6) {
            token_list.push(Token::Comment(&comment.as_str()));
        }

        last_end = end;
    }

    if text.len() != last_end {
        token_list.push(Token::Text(&text[last_end..]));
    }
    token_list
}

macro_rules! colorize {
    (  $text:expr, $color:expr, $sbuilder:expr) => {{
        $sbuilder.push_str("<span color=\"");
        $sbuilder.push_str(&$color.get_hexa_code());
        $sbuilder.push_str("\">");
        $sbuilder.push_str($text);
        $sbuilder.push_str("</span>");
    }};
}

fn make_markup<'a>(text: &'a str, token_list: &Vec<Token>, _dark: bool) -> Cow<'a, str> {
    let mut out = String::with_capacity(text.len() * 2);

    for token in token_list {
        match *token {
            Token::Text(txt) => out.push_str(txt),
            Token::Label(label) => {
                colorize!(label, TermColor::Cyan, out);
                out.push('=')
            }
            Token::Value(value) => colorize!(value, TermColor::BrightYellow, out),
            Token::Number(num) => colorize!(num, TermColor::Yellow, out),
            Token::Comment(comment) => colorize!(comment, TermColor::Green, out),
            Token::Section(section) => colorize!(section, TermColor::BrightCyan, out),
        }
    }
    Cow::from(out)
}

#[derive(Debug)]
enum Token<'a> {
    Text(&'a str),
    Label(&'a str),
    Value(&'a str),
    Number(&'a str),
    Comment(&'a str),
    Section(&'a str),
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
    fn test_color_token() {
        let tokens = get_tokens(TEST_INI_FILE);
        //println!("capture len: {}",TEST_INI_FILE);
        println!("tokens len: {:#?}", tokens.len());

        for token in tokens {
            println!("{:?}", token)
        }
    }

    #[test]
    fn test_color_convert() {
        let converted_text = convert_to_mackup(TEST_INI_FILE, true);
        //println!("capture len: {}",TEST_INI_FILE);
        println!("{}", converted_text);
    }
}
