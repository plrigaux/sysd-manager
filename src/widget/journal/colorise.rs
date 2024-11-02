//https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit

use std::{borrow::Cow, fmt::Debug, sync::LazyLock};

use log::{debug, info, warn};
use regex::Regex;

use super::more_colors::{self, ColorCodeError, Intensity, TermColor};

static RE: LazyLock<Regex> = LazyLock::new(|| {
    //https://stackoverflow.com/questions/14693701/how-can-i-remove-the-ansi-escape-sequences-from-a-string-in-python
    let re = match Regex::new(
        r"(?x)
        \u{1b}  # ESC
        (?:   # 7-bit C1 Fe (except CSI)
            [@-Z\^-_]
        |   # or [ for CSI, followed by a control sequence
            \[
            ([0-?]*)  # Parameter bytes
            [ -/]*    # Intermediate bytes
            ([@-~])   # Final byte
        |
            # or ] for OSC hyperlink
            \]8;;
            ([\s!-~]*) #link
            [\u{7}\\]
            (.*)       #link text
            \u{1b}\]8;;[\u{7}\\]
        )",
    ) {
        Ok(ok) => ok,
        Err(e) => {
            log::error!("Rexgex compile error : {:?}", e);
            panic!()
        }
    };

    re
});

// echo "\x1b[35;47mANSI? \x1b[0m\x1b[1;32mSI\x1b[0m \x1b]8;;man:abrt(1)\x1b\\[🡕]\x1b]8;;\x1b\\ test \x1b[0m"
pub fn convert_to_mackup<'a>(text: &'a str, text_color: &'a TermColor) -> Cow<'a, str> {
    let token_list = get_tokens(text);

    make_markup(text, &token_list, text_color)
}

fn get_tokens(text: &str) -> Vec<Token> {
    let mut token_list = Vec::<Token>::new();
    let mut last_end: usize = 0;

    for captures in RE.captures_iter(text) {
        let main_match = captures.get(0).expect("not supose to happen");
        let end = main_match.end();
        let start = main_match.start();

        if start != last_end {
            token_list.push(Token::Text(&text[last_end..start]));
        }
        last_end = end;

        if let Some(osc_contol) = captures.get(2) {
            let control = osc_contol.as_str();
            if control == "m" {
                if let Some(select_graphic_rendition_match) = captures.get(1) {
                    let select_graphic_rendition = select_graphic_rendition_match.as_str();
                    match capture_code(select_graphic_rendition, &mut token_list) {
                        Ok(_) => {
                            continue;
                        }
                        Err(e) => {
                            warn!("while parsing {select_graphic_rendition} got error {:?}", e)
                        }
                    };
                }
            }
        } else if let Some(link_match) = captures.get(3) {
            if let Some(link_text_match) = captures.get(4) {
                token_list.push(Token::Hyperlink(
                    link_match.as_str(),
                    link_text_match.as_str(),
                ));
                continue;
            }
        }
        token_list.push(Token::UnHandled(main_match.as_str().to_owned()));
    }

    if text.len() != last_end {
        token_list.push(Token::Text(&text[last_end..]));
    }
    token_list
}

fn make_markup<'a>(
    text: &'a str,
    token_list: &Vec<Token>,
    _text_color: &TermColor,
) -> Cow<'a, str> {
    if token_list.len() == 1 {
        return Cow::from(text);
    }

    let mut out = String::with_capacity((text.len() as f32 * 1.5) as usize);

    let mut sgr = SelectGraphicRendition::default();
    let mut first = true;
    for token in token_list {
        match token {
            Token::Text(sub_text) => {
                first = !sgr.append_tags(&mut out, first);
                out.push_str(sub_text)
            }
            Token::Intensity(intensity) => sgr.set_intensity(Some(*intensity)),
            Token::FgColor(term_color) => sgr.set_foreground_color(Some(*term_color)),
            Token::BgColor(term_color) => sgr.set_background_color(Some(*term_color)),
            Token::Italic => sgr.set_italic(true),
            Token::Underline(underline) => sgr.set_underline(*underline),
            Token::Blink => sgr.set_blink(true),
            Token::Reversed => sgr.set_reversed(true),
            Token::Hidden => sgr.set_hidden(true),
            Token::Strikeout => sgr.set_strikeout(true),
            Token::Hyperlink(link, link_text) => {
                debug!("Do hyperlink {link} {link_text}");

                //out.push_str("<a href=\"");
                //out.push_str(&link_text);
                //out.push_str("\">");
                let new_link_text = convert_to_mackup(link_text, &TermColor::Black);
                out.push_str(&new_link_text); //TODO escape <>
                                              //out.push_str("</a>");
            }
            Token::UnHandledCode(code) => info!("UnHandledCode {code}"),
            Token::UnHandled(a) => debug!("UnHandled {a}"),

            Token::Reset(reset_type) => match reset_type {
                ResetType::All => sgr.reset(),
                ResetType::FgColor => sgr.set_foreground_color(None),
                ResetType::BgColor => sgr.set_background_color(None),
                ResetType::Intensity => sgr.set_intensity(None),
                ResetType::Hidden => sgr.set_hidden(false),
            },
        }
    }

    if !first {
        out.push_str("</span>")
    }

    Cow::from(out)
}

fn capture_code(code_line: &str, vec: &mut Vec<Token>) -> Result<(), ColorCodeError> {
    let mut it = code_line.split(&[';',':']); // insome case they use : as separator

    while let Some(code) = it.next() {
        let token = match code {
            "0" => Token::Reset(ResetType::All),
            "1" => Token::Intensity(Intensity::Bold),
            "2" => Token::Intensity(Intensity::Faint),
            "3" => Token::Italic,
            "4" => Token::Underline(Underline::Single),
            "5" => Token::Blink,
            "6" => Token::Blink,
            "7" => Token::Reversed,
            "8" => Token::Hidden,
            "9" => Token::Strikeout,
            "22" => Token::Reset(ResetType::Intensity),
            "28" => Token::Reset(ResetType::Hidden),
            "21" => Token::Underline(Underline::Double),
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

fn find_color(it: &mut std::str::Split<'_, &[char; 2]>) -> Result<TermColor, ColorCodeError> {
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
enum Token<'a> {
    FgColor(TermColor),
    BgColor(TermColor),
    Intensity(Intensity),
    Italic,
    Underline(Underline),
    Blink,
    Reversed,
    Hidden,
    Strikeout,
    Text(&'a str),
    Reset(ResetType),
    Hyperlink(&'a str, &'a str),
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

#[derive(Default, PartialEq, Eq)]
struct SelectGraphicRendition {
    foreground_color: Option<TermColor>,
    background_color: Option<TermColor>,
    intensity: Option<Intensity>,
    italic: Option<bool>,
    underline: Option<Underline>,
    blink: Option<bool>,
    reversed: Option<bool>,
    hidden: Option<bool>,
    strikeout: Option<bool>,
}

macro_rules! span {
    (  $first:expr, $out:expr  ) => {{
        if !$first {
            $out.push_str("<span");
            $first = true
        }
    }};
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
        self.blink = None;
    }

    fn set_intensity(&mut self, intensity: Option<Intensity>) {
        self.intensity = intensity;
    }

    fn set_italic(&mut self, italic: bool) {
        self.italic = Some(italic);
    }

    fn set_underline(&mut self, underline: Underline) {
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

    fn set_foreground_color(&mut self, color: Option<TermColor>) {
        self.foreground_color = color;
    }

    fn set_background_color(&mut self, color: Option<TermColor>) {
        self.background_color = color;
    }

    fn set_blink(&mut self, blink: bool) {
        self.blink = Some(blink);
    }

    fn append_tags(&self, out: &mut String, first: bool) -> bool {
        if !first {
            out.push_str("</span>")
        }

        let mut write_something = false;

        if let Some(underline) = self.underline {
            span!(write_something, out);

            out.push_str(" underline=\"");
            out.push_str(underline.pango());
            out.push('\"');
        }

        if let Some(_strikeout) = self.strikeout {
            span!(write_something, out);

            out.push_str(" strikethrough=\"true\"");
        }

        if let Some(_italic) = self.italic {
            span!(write_something, out);

            out.push_str(" style=\"italic\"");
        }

        if let Some(intensity) = self.intensity {
            span!(write_something, out);

            out.push_str(" weight=\"");
            out.push_str(intensity.pango());
            out.push('\"');
        }

        if let Some(color) = self.foreground_color {
            span!(write_something, out);

            out.push_str(" color=\"");
            out.push_str(&color.get_hexa_code());
            out.push('\"');
        }

        if let Some(color) = self.background_color {
            span!(write_something, out);

            out.push_str(" background=\"");
            out.push_str(&color.get_hexa_code());
            out.push('\"');
        }

        if write_something {
            out.push('>');
        }

        write_something
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Underline {
    Single,

    Double,
}
impl Underline {
    fn pango(&self) -> &str {
        match self {
            Underline::Single => "single",
            Underline::Double => "double",
        }
    }
}

#[cfg(test)]
mod tests {

    use gtk::gdk;

    use crate::widget::journal::colorise::Intensity;

    use super::*;

    const TEST_STRS : [&str; 4] = [  "This is \u{1b}[4mvery\u{1b}[0m\u{1b}[1m\u{1b}[96m Important\u{1b}[0m",
    "asdf \u{1b}[38;2;255;140;0;48;2;255;228;225mExample 24 bit color escape sequence\u{1b}[0m",
    "0:13:37 fedora abrt-server[90694]: \u{1b}[0;1;38;5;185m\u{1b}[0;1;39m\u{1b}[0;1;38;5;185m'post-create' on '/var/spool/abrt/ccpp-2024-10-08-10:13:37.85581-16875' exited with 1\u{1b}[0m",
    "nothing \u{1b}[91mframed\u{1b}[7m test ok\u{1b}[0m"];

    #[test]
    fn test_display() {
        let mut line = 0;
        for s in TEST_STRS {
            println!("line {} {}", line, s);
            line += 1;
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

            let result = convert_to_mackup(s, &TermColor::Black);

            println!("{}", result);
        }
    }

    #[test]
    fn test_reverse() {
        let s = "reverse test \u{1b}[7m reverse test \u{1b}[0;m test test \u{1b}[97mwhite\u{1b}[0m";
        println!("{}", s);

        let result = convert_to_mackup(s, &TermColor::Black);

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

    #[test]
    fn test_convert_color() {
        for (a, b) in [
            (gdk::RGBA::WHITE, TermColor::BrightWhite),
            (gdk::RGBA::RED, TermColor::BrightRed),
            (gdk::RGBA::GREEN, TermColor::BrightGreen),
            (gdk::RGBA::BLUE, TermColor::BrightBlue),
            (gdk::RGBA::BLACK, TermColor::Black),
        ] {
            assert_convert_color(a, b);
        }
    }

    fn assert_convert_color(color_gtk: gdk::RGBA, color_term: TermColor) {
        let color: TermColor = color_gtk.into();
        println!("{:?} {:?}", color, color_gtk);
        assert_eq!(color, color_term.get_vga(),);
    }

    #[test]
    fn test_make_markup() {
        let text = "this text is in italic not in bold.";
        let vaec = vec![
            Token::Text(&text[0..16]),
            Token::Italic,
            Token::Text(&text[16..22]),
            Token::Reset(ResetType::All),
            Token::Text(&text[22..30]),
            Token::Intensity(Intensity::Bold),
            Token::Text(&text[30..]),
        ];

        let out = make_markup(text, &vaec, &TermColor::Black);

        println!("out: {out}");
    }

    #[test]
    fn test_make_markup2() {
        let text = "this text is in italic not in bold.";
        let vaec = vec![
            Token::Text(&text[0..16]),
            Token::Italic,
            Token::Intensity(Intensity::Bold),
            Token::Italic,
            Token::Text(&text[16..20]),
            Token::Text(&text[20..22]),
            Token::Reset(ResetType::All),
            Token::Text(&text[22..30]),
            Token::Intensity(Intensity::Bold),
            Token::Text(&text[30..]),
        ];

        let out = make_markup(text, &vaec, &TermColor::Black);

        println!("out: {out}");
    }

    #[test]
    fn test_link_regex() {
        let test_str = "\x1b[35;47mANSI? \x1b[0m\x1b[1;32mSI\x1b[0m \x1b]8;;man:abrt(1)\x1b\u{07}[🡕]\x1b]8;;\x1b\u{7} test \x1b[0m";

        //let test_str = "begin \x1b]8;;man:abrt(1)\x1b\\[🡕]\x1b]8;;\x1b\\ test";
        //let test_str = "begin \x1b]8;;qwer:\x1b\\[🡕]\x1b]8;;\x1b\\ test";

        for capt in RE.captures_iter(test_str) {
            println!("capture: {:#?}", capt)
        }
    }

    #[test]
    fn test_link_regex2() {
        let test_text = "Oct 15 08:07:19 fedora abrt-notification[160431]: \u{1b}]8;;man:abrt(1)\u{7}[🡕]\u{1b}]8;;\u{7}   end of line";

        for capt in RE.captures_iter(test_text) {
            println!("capture: {:#?}", capt);
            assert_eq!("man:abrt(1)", capt.get(3).unwrap().as_str());
            assert_eq!("[🡕]", capt.get(4).unwrap().as_str());
        }
    }

    #[test]
    fn test_link_regex3() {
        let test_text =  "Oct 16 16:03:05 fedora systemd[1]: \u{1b}[0;1;38;5;185m\u{1b}]8;;file://fedora/etc/systemd/system/tiny_daemon.service\u{7}/etc/s\u{1b}[0;1;39m\u{1b}[0;1;38;5;185mystemd/system/tiny_daemon.service\u{1b}]8;;\u{7}:18: Unknown key 'test' in section [Install], ignoring.\u{1b}[0m\n";

        for capt in RE.captures_iter(test_text) {
            println!("capture: {:#?}", capt);
        }

        //convert_to_mackup(&test_text, &gdk::RGBA::BLACK);

        let token_list = get_tokens(test_text);
        println!("token_list: {:#?}", token_list);
        let out = make_markup(test_text, &token_list, &TermColor::Black);

        println!("out {out}");
    }
}
