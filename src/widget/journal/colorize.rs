//https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit

use std::{fmt::Debug, sync::LazyLock};

use gtk::{pango, prelude::TextBufferExt};
use log::{debug, info, warn};
use regex::bytes::Regex;

use crate::utils::{
    more_colors::{ColorCodeError, Intensity, TermColor, get_256color},
    writer::UnitInfoWriter,
};

//use super::more_colors::{self, ColorCodeError, Intensity, TermColor};

static RE: LazyLock<Regex> = LazyLock::new(|| {
    //https://stackoverflow.com/questions/14693701/how-can-i-remove-the-ansi-escape-sequences-from-a-string-in-python
    match Regex::new(
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
            let error_msg = format!("Regex compile error : {:?}", e);
            log::error!("{error_msg}");
            panic!("{error_msg}")
        }
    }
});

pub fn write(
    writer: &mut UnitInfoWriter,
    text: &str,
    token_list: &mut Vec<Token>,
    added_tokens: &[Token],
) {
    token_list.clear();
    token_list.extend_from_slice(added_tokens);
    get_tokens(token_list, text);
    write_text(token_list, writer, text);
}

pub fn get_tokens(token_list: &mut Vec<Token>, text: &str) {
    let mut last_end: usize = 0;

    for captures in RE.captures_iter(text.as_bytes()) {
        let main_match = captures.get(0).expect("not supose to happen");
        let end = main_match.end();
        let start = main_match.start();

        if start != last_end {
            token_list.push(Token::Text(last_end, start));
        }
        last_end = end;

        if let Some(osc_contol) = captures.get(2) {
            let control = osc_contol.as_bytes();
            if control == b"m"
                && let Some(select_graphic_rendition_match) = captures.get(1)
            {
                let select_graphic_rendition = select_graphic_rendition_match.as_bytes();
                match capture_code(select_graphic_rendition, token_list) {
                    Ok(_) => {
                        continue;
                    }
                    Err(e) => {
                        warn!(
                            "while parsing {} got error {:?}",
                            String::from_utf8_lossy(select_graphic_rendition),
                            e
                        )
                    }
                };
            }
        } else if let Some(link_match) = captures.get(3)
            && let Some(link_text_match) = captures.get(4)
        {
            token_list.push(Token::Hyperlink(
                link_match.start(),
                link_match.end(),
                link_text_match.start(),
                link_text_match.end(),
            ));
            continue;
        }

        let s = bytes_to_string(main_match.as_bytes());
        token_list.push(Token::UnHandled(s));
    }

    if text.len() != last_end {
        token_list.push(Token::Text(last_end, text.len()));
    }
}

fn bytes_to_string(main_match: &[u8]) -> String {
    match String::from_utf8(main_match.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            warn!("while parsing {:?} got error {:?}", main_match, e);
            String::from_utf8_lossy(main_match).to_string()
        }
    }
}

pub(super) fn write_text(tokens: &Vec<Token>, writer: &mut UnitInfoWriter, text: &str) {
    let tag_table = writer.buffer.tag_table();

    let mut select_graphic_rendition = SelectGraphicRendition::default();

    for token in tokens {
        match token {
            Token::Text(start, end) => {
                // !sgr.append_tags(&mut out, first);

                let start_offset = writer.text_iterator.offset();

                //let sub_text = sub_string(text, *start, *end, token);
                let sub_text = &text[*start..*end];

                writer.buffer.insert(&mut writer.text_iterator, sub_text);
                let start_iter = writer.buffer.iter_at_offset(start_offset);

                select_graphic_rendition.apply_tags(
                    &tag_table,
                    &writer.buffer,
                    &start_iter,
                    &writer.text_iterator,
                );
                /*                 for tag in  sgr.tags.iter() {
                    buf.apply_tag(tag, &start_iter, &iter);
                }  */
            }

            Token::Intensity(intensity) => select_graphic_rendition.set_intensity(Some(*intensity)),
            Token::FgColor(term_color) => {
                select_graphic_rendition.set_foreground_color(Some(*term_color))
            }
            Token::BgColor(term_color) => {
                select_graphic_rendition.set_background_color(Some(*term_color))
            }
            Token::Italic => select_graphic_rendition.set_italic(true),
            Token::Underline(underline) => select_graphic_rendition.set_underline(*underline),
            Token::Blink => select_graphic_rendition.set_blink(true),
            Token::Reversed => select_graphic_rendition.set_reversed(true),
            Token::Hidden => select_graphic_rendition.set_hidden(true),
            Token::Strikeout => select_graphic_rendition.set_strikeout(true),
            Token::Hyperlink(link_start, link_end, link_text_stert, link_text_end) => {
                let link = &text[*link_start..*link_end];

                let link_text = &text[*link_text_stert..*link_text_end];
                debug!("Do hyperlink {link} {link_text}");

                //out.push_str("<a href=\"");
                //out.push_str(&link_text);
                //out.push_str("\">");
                //let new_link_text = convert_to_mackup(link_text, &TermColor::Black);

                //out.push_str(&new_link_text); //TODO escape <>
                //out.push_str("</a>");
            }
            Token::UnHandledCode(code) => info!("UnHandledCode {code}"),
            Token::UnHandled(a) => debug!("UnHandled {a}"),

            Token::Reset(reset_type) => match reset_type {
                ResetType::All => select_graphic_rendition.reset(),
                ResetType::FgColor => select_graphic_rendition.set_foreground_color(None),
                ResetType::BgColor => select_graphic_rendition.set_background_color(None),
                ResetType::Intensity => select_graphic_rendition.set_intensity(None),
                ResetType::Hidden => select_graphic_rendition.set_hidden(false),
            },
        }
    }
}

fn bsplit(b: &u8) -> bool {
    matches!(b, b';' | b':')
}

fn capture_code(code_line: &[u8], vec: &mut Vec<Token>) -> Result<(), ColorCodeError> {
    let mut it: std::slice::Split<'_, u8, fn(&u8) -> bool> = code_line.split(bsplit); // insome case they use : as separator

    while let Some(code) = it.next() {
        let token = match code {
            b"0" => Token::Reset(ResetType::All),
            b"1" => Token::Intensity(Intensity::Bold),
            b"2" => Token::Intensity(Intensity::Faint),
            b"3" => Token::Italic,
            b"4" => Token::Underline(Underline::Single),
            b"5" => Token::Blink,
            b"6" => Token::Blink,
            b"7" => Token::Reversed,
            b"8" => Token::Hidden,
            b"9" => Token::Strikeout,
            b"22" => Token::Reset(ResetType::Intensity),
            b"28" => Token::Reset(ResetType::Hidden),
            b"21" => Token::Underline(Underline::Double),
            b"30" => Token::FgColor(TermColor::Black),
            b"31" => Token::FgColor(TermColor::Red),
            b"32" => Token::FgColor(TermColor::Green),
            b"33" => Token::FgColor(TermColor::Yellow),
            b"34" => Token::FgColor(TermColor::Blue),
            b"35" => Token::FgColor(TermColor::Magenta),
            b"36" => Token::FgColor(TermColor::Cyan),
            b"37" => Token::FgColor(TermColor::White),
            b"38" => {
                let color = find_color(&mut it)?;
                Token::FgColor(color)
            }
            b"39" => Token::Reset(ResetType::FgColor),

            b"40" => Token::BgColor(TermColor::Black),
            b"41" => Token::BgColor(TermColor::Red),
            b"42" => Token::BgColor(TermColor::Green),
            b"43" => Token::BgColor(TermColor::Yellow),
            b"44" => Token::BgColor(TermColor::Blue),
            b"45" => Token::BgColor(TermColor::Magenta),
            b"46" => Token::BgColor(TermColor::Cyan),
            b"47" => Token::BgColor(TermColor::White),
            b"48" => {
                let color = find_color(&mut it)?;
                Token::BgColor(color)
            }
            b"49" => Token::Reset(ResetType::BgColor),
            b"90" => Token::FgColor(TermColor::BrightBlack),
            b"91" => Token::FgColor(TermColor::BrightRed),
            b"92" => Token::FgColor(TermColor::BrightGreen),
            b"93" => Token::FgColor(TermColor::BrightYellow),
            b"94" => Token::FgColor(TermColor::BrightBlue),
            b"95" => Token::FgColor(TermColor::BrightMagenta),
            b"96" => Token::FgColor(TermColor::BrightCyan),
            b"97" => Token::FgColor(TermColor::BrightWhite),

            b"100" => Token::BgColor(TermColor::BrightBlack),
            b"101" => Token::BgColor(TermColor::BrightRed),
            b"102" => Token::BgColor(TermColor::BrightGreen),
            b"103" => Token::BgColor(TermColor::BrightYellow),
            b"104" => Token::BgColor(TermColor::BrightBlue),
            b"105" => Token::BgColor(TermColor::BrightMagenta),
            b"106" => Token::BgColor(TermColor::BrightCyan),
            b"107" => Token::BgColor(TermColor::BrightWhite),
            unknown_code => Token::UnHandledCode(bytes_to_string(unknown_code)),
        };

        vec.push(token)
    }
    Ok(())
}

fn find_color(
    it: &mut std::slice::Split<'_, u8, fn(&u8) -> bool>,
) -> Result<TermColor, ColorCodeError> {
    let Some(sub_code) = it.next() else {
        return Err(ColorCodeError::Malformed);
    };
    let color = match sub_code {
        b"5" => {
            if let Some(color_code) = it.next() {
                get_256color(color_code[0])
            } else {
                return Err(ColorCodeError::Malformed);
            }
        }
        b"2" => {
            let Some(r) = it.next() else {
                return Err(ColorCodeError::Malformed);
            };

            let Some(g) = it.next() else {
                return Err(ColorCodeError::Malformed);
            };

            let Some(b) = it.next() else {
                return Err(ColorCodeError::Malformed);
            };
            let r = str::from_utf8(r)?;
            let g = str::from_utf8(g)?;
            let b = str::from_utf8(b)?;

            TermColor::new_vga(r, g, b)?
        }
        unexpected_code => {
            return Err(ColorCodeError::UnexpectedCode(bytes_to_string(
                unexpected_code,
            )));
        }
    };
    Ok(color)
}

#[derive(Debug, Clone)]
pub(super) enum Token {
    FgColor(TermColor),
    BgColor(TermColor),
    Intensity(Intensity),
    Italic,
    Underline(Underline),
    Blink,
    Reversed,
    Hidden,
    Strikeout,
    Text(usize, usize),
    Reset(ResetType),
    Hyperlink(usize, usize, usize, usize),
    UnHandledCode(String),
    UnHandled(String),
}

#[derive(Debug, Clone)]
pub(super) enum ResetType {
    All,
    FgColor,
    BgColor,
    Intensity,
    Hidden,
}

#[derive(Default, PartialEq, Eq)]
pub struct SelectGraphicRendition {
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

/* macro_rules! span {
    (  $first:expr, $out:expr  ) => {{
        if !$first {
            $out.push_str("<span");
            $first = true
        }
    }};
} */

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

    fn apply_tags(
        &mut self,
        tag_table: &gtk::TextTagTable,
        buf: &gtk::TextBuffer,
        start_iter: &gtk::TextIter,
        end_iter: &gtk::TextIter,
    ) {
        if let Some(underline) = self.underline {
            let tt = gtk::TextTag::builder().underline(underline.pango()).build();
            tag_table.add(&tt);
            buf.apply_tag(&tt, start_iter, end_iter);
        }

        if let Some(strikeout) = self.strikeout {
            let tt = gtk::TextTag::builder().strikethrough(strikeout).build();
            tag_table.add(&tt);
            buf.apply_tag(&tt, start_iter, end_iter);
        }

        if let Some(_italic) = self.italic {
            const ITALIC: &str = "italic";
            let tag = if let Some(tag) = tag_table.lookup(ITALIC) {
                tag
            } else {
                let tag = gtk::TextTag::builder()
                    .style(pango::Style::Italic)
                    .name(ITALIC)
                    .build();
                tag_table.add(&tag);
                tag
            };

            buf.apply_tag(&tag, start_iter, end_iter);
        }

        if let Some(intensity) = self.intensity {
            let name = intensity.pango_str();
            let tt: gtk::TextTag = if let Some(tag) = tag_table.lookup(name) {
                tag
            } else {
                let mut tag_builder = gtk::TextTag::builder()
                    .weight(intensity.pango_i32())
                    .name(name);

                if intensity == Intensity::Faint {
                    tag_builder =
                        tag_builder.foreground_rgba(&TermColor::Vga(94, 94, 94).get_rgba());
                }

                let tag = tag_builder.build();
                tag_table.add(&tag);
                tag
            };

            buf.apply_tag(&tt, start_iter, end_iter);
        }

        if let Some(color) = self.foreground_color {
            let tt = gtk::TextTag::builder()
                .foreground_rgba(&color.get_rgba())
                .build();
            tag_table.add(&tt);
            buf.apply_tag(&tt, start_iter, end_iter);
        }

        if let Some(color) = self.background_color {
            let tt = gtk::TextTag::builder()
                .background_rgba(&color.get_rgba())
                .build();
            tag_table.add(&tt);
            buf.apply_tag(&tt, start_iter, end_iter);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Underline {
    Single,

    Double,
}
impl Underline {
    /*     fn pango_str(&self) -> &str {
        match self {
            Underline::Single => "single",
            Underline::Double => "double",
        }
    } */

    fn pango(&self) -> pango::Underline {
        match self {
            Underline::Single => pango::Underline::Single,
            Underline::Double => pango::Underline::Double,
        }
    }
}

#[cfg(test)]
mod tests {

    use gtk::gdk;
    use test_base::init_logs;

    use super::*;

    const TEST_STRS: [&str; 4] = [
        "This is \u{1b}[4mvery\u{1b}[0m\u{1b}[1m\u{1b}[96m Important\u{1b}[0m",
        "asdf \u{1b}[38;2;255;140;0;48;2;255;228;225mExample 24 bit color escape sequence\u{1b}[0m",
        "0:13:37 fedora abrt-server[90694]: \u{1b}[0;1;38;5;185m\u{1b}[0;1;39m\u{1b}[0;1;38;5;185m'post-create' on '/var/spool/abrt/ccpp-2024-10-08-10:13:37.85581-16875' exited with 1\u{1b}[0m",
        "nothing \u{1b}[91mframed\u{1b}[7m test ok\u{1b}[0m",
    ];

    #[test]
    fn test_display() {
        for (line, s) in TEST_STRS.into_iter().enumerate() {
            println!("line {} {}", line, s);
        }
    }

    #[test]
    fn test_tokens() {
        for (line, s) in TEST_STRS.into_iter().enumerate() {
            println!("\nLine {line}");
            println!("{}", s);

            let result = get_tokens_test(s);

            println!("{:?}", result);
        }
    }

    /*     #[test]
    fn test_full() {
        for s in TEST_STRS {
            println!("{}", s);

            let result = convert_to_mackup(s, &TermColor::Black);

            println!("{}", result);
        }
    } */

    /*     #[test]
    fn test_reverse() {
        let s = "reverse test \u{1b}[7m reverse test \u{1b}[0;m test test \u{1b}[97mwhite\u{1b}[0m";
        println!("{}", s);

        let result = convert_to_mackup(s, &TermColor::Black);

        println!("{}", result);
    } */

    #[test]
    fn test_color_regex() {
        let mut results = vec![];

        for (line, haystack) in TEST_STRS.into_iter().enumerate() {
            for capt in RE.captures_iter(haystack.as_bytes()) {
                results.push((line, capt));
            }
        }

        for capt in results {
            println!("line {} capture: {:#?}", capt.0, capt.1)
        }
    }

    #[test]
    fn test_capture_code() {
        let mut vec = Vec::<Token>::new();
        let _res = capture_code(b"0;1;38;5;185", &mut vec);

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

    /*     #[test]
    fn test_make_markup() {
        let text = "this text is in italic not in bold.";
        let vaec = vec![
            Token::Text(0, 16),
            Token::Italic,
            Token::Text(16, 22),
            Token::Reset(ResetType::All),
            Token::Text(22, 30),
            Token::Intensity(Intensity::Bold),
            Token::Text(30, text.len()),
        ];

        let out = make_markup(text, &vaec, &TermColor::Black);

        println!("out: {out}");
    } */

    /*     #[test]
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
    } */

    #[test]
    fn test_link_regex() {
        let test_str = "\x1b[35;47mANSI? \x1b[0m\x1b[1;32mSI\x1b[0m \x1b]8;;man:abrt(1)\x1b\u{07}[🡕]\x1b]8;;\x1b\u{7} test \x1b[0m".as_bytes();

        //let test_str = "begin \x1b]8;;man:abrt(1)\x1b\\[🡕]\x1b]8;;\x1b\\ test";
        //let test_str = "begin \x1b]8;;qwer:\x1b\\[🡕]\x1b]8;;\x1b\\ test";

        for capt in RE.captures_iter(test_str) {
            println!("capture: {:#?}", capt)
        }
    }

    #[test]
    fn test_link_regex2() {
        let test_text = "Oct 15 08:07:19 fedora abrt-notification[160431]: \u{1b}]8;;man:abrt(1)\u{7}[🡕]\u{1b}]8;;\u{7}   end of line".as_bytes();

        for capt in RE.captures_iter(test_text) {
            println!("capture: {:#?}", capt);
            assert_eq!(b"man:abrt(1)", capt.get(3).unwrap().as_bytes());
            assert_eq!(
                "[🡕]",
                str::from_utf8(capt.get(4).unwrap().as_bytes()).unwrap()
            );
        }
    }

    /*     #[test]
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
    } */

    pub fn get_tokens_test(text: &str) -> Vec<Token> {
        let mut token_list = Vec::<Token>::new();
        super::get_tokens(&mut token_list, text);
        token_list
    }

    #[test]
    fn test_tok_amp() {
        let test_text = "Gnome & Co";

        let token_list = get_tokens_test(test_text);

        println!("out {:?}", token_list);
    }

    #[test]
    fn test_rust_log() {
        let logs = r#"
        Oct 10 02:02:44 systemd[1]: tiny_daemon.service: Deactivated successfully.
Oct 10 02:02:44 systemd[1]: Stopped It is tiny, but is not the tiniest.
Oct 10 02:02:44 systemd[1]: Started It is tiny, but is not the tiniest.
Oct 10 02:02:44 tiny_daemon[338370]: [2m2025-10-10T06:02:44.657790Z[0m [32m INFO[0m [2mtiny_daemon[0m[2m:[0m Starting tiny_daemon...
Oct 10 02:02:44 tiny_daemon[338370]: [2m2025-10-10T06:02:44.657890Z[0m [32m INFO[0m [2mtiny_daemon[0m[2m:[0m SIGRTMIN() + 1 = 35!!!
Oct 10 02:02:44 tiny_daemon[338370]: [2m2025-10-10T06:02:44.657898Z[0m [33m WARN[0m [2mtiny_daemon[0m[2m:[0m test warning message
Oct 10 02:02:44 tiny_daemon[338370]: [2m2025-10-10T06:02:44.657903Z[0m [31mERROR[0m [2mtiny_daemon[0m[2m:[0m test error message
Oct 10 02:02:44 tiny_daemon[338370]: [2m2025-10-10T06:02:44.657956Z[0m [32m INFO[0m [2mtiny_daemon[0m[2m:[0m Tiny Daemon listening on 127.0.0.1:33001
"#;

        for (line, s) in logs.lines().enumerate() {
            println!("\nLine {line}");
            println!("{}", s);

            let result = get_tokens_test(s);

            println!("{:?}", result);
        }
    }

    #[test]
    fn test_color_out() {
        let escape = '\u{001b}';
        // Try this one-liner
        for x in 0..5 {
            println!("---");
            for z in [0, 10, 60, 70] {
                for y in 30..37 {
                    let y = y + z;
                    let label = format!("\\e[{x};{y}m");
                    print!("{escape}[{x};{y}m {label: ^10} {escape}[0m");
                }
                println!()
            }
        }
    }

    #[test]
    fn test_color_out2() {
        let escape = '\u{001b}';
        for code in 0..255 {
            println!("{escape}[38;5;{code}m[38;5;'{code}m{escape}[0m");
        }
    }

    #[test]
    fn test_char_boundary() {
        let s = "ab早cd";
        for (index, character) in s.char_indices() {
            println!("Character '{}' starts at byte index {}", character, index);
        }
    }
    /*
       #[test]
       fn test_tok_amp_regex() {
           //let re_amp = Regex::new(r"\&").unwrap();

           let test_text = "Gnome & Co";

           let replaced = RE_AMP.replace_all(test_text, "&amp;");

           println!("replaced {}", replaced);
       }
    */
    /*     #[test]
    fn test_tok_amp_convert() {
        //let re_amp = Regex::new(r"\&").unwrap();

        let test_text = "Gnome & Co";

        let out = convert_to_mackup(test_text, &TermColor::Black);

        println!("replaced {}", out);
    } */
    #[test]
    fn test_char_boundary2() {
        let s = "Löwe 老虎 Léopard";

        for index in 0..s.len() {
            println!(
                "Character at {} is char boundary {}",
                index,
                s.is_char_boundary(index)
            );
        }
    }

    #[test]
    fn test_multiple_escape_sequences() {
        // This test checks parsing of multiple consecutive ANSI escape sequences.
        let test_str =
            "\u{1b}[1mBold\u{1b}[0m and \u{1b}[3mItalic\u{1b}[0m and \u{1b}[4mUnderline\u{1b}[0m";
        let tokens = get_tokens_test(test_str);
        // Should alternate between formatting tokens and text tokens
        let mut found_bold = false;
        let mut found_italic = false;
        let mut found_underline = false;
        for token in &tokens {
            match token {
                Token::Intensity(Intensity::Bold) => found_bold = true,
                Token::Italic => found_italic = true,
                Token::Underline(Underline::Single) => found_underline = true,
                _ => {}
            }
        }
        assert!(found_bold, "Bold token not found");
        assert!(found_italic, "Italic token not found");
        assert!(found_underline, "Underline token not found");
    }

    #[test]
    fn test_unhandled_escape_sequence() {
        // This test checks that an unknown escape code is handled as UnHandledCode
        let test_str = "\u{1b}[999mUnknown\u{1b}[0m";
        let tokens = get_tokens_test(test_str);
        let mut found_unhandled = false;
        for token in &tokens {
            if let Token::UnHandledCode(code) = token {
                assert_eq!(code, "999");
                found_unhandled = true;
            }
        }
        assert!(
            found_unhandled,
            "UnHandledCode token not found for unknown code"
        );
    }

    #[test]
    fn test_reset_types() {
        // This test checks that reset codes are parsed correctly
        let test_str = "\u{1b}[0mResetAll\u{1b}[39mResetFg\u{1b}[49mResetBg\u{1b}[22mResetIntensity\u{1b}[28mResetHidden";
        let tokens = get_tokens_test(test_str);
        let mut found_all = false;
        let mut found_fg = false;
        let mut found_bg = false;
        let mut found_intensity = false;
        let mut found_hidden = false;
        for token in &tokens {
            match token {
                Token::Reset(ResetType::All) => found_all = true,
                Token::Reset(ResetType::FgColor) => found_fg = true,
                Token::Reset(ResetType::BgColor) => found_bg = true,
                Token::Reset(ResetType::Intensity) => found_intensity = true,
                Token::Reset(ResetType::Hidden) => found_hidden = true,
                _ => {}
            }
        }
        assert!(found_all, "ResetType::All not found");
        assert!(found_fg, "ResetType::FgColor not found");
        assert!(found_bg, "ResetType::BgColor not found");
        assert!(found_intensity, "ResetType::Intensity not found");
        assert!(found_hidden, "ResetType::Hidden not found");
    }

    #[test]
    fn test_24bit_color_parsing() {
        // This test checks parsing of 24-bit color escape sequences
        let test_str = "\u{1b}[38;2;12;34;56m24bitFG\u{1b}[48;2;78;90;123m24bitBG\u{1b}[0m";
        let tokens = get_tokens_test(test_str);
        let mut found_fg = false;
        let mut found_bg = false;
        for token in &tokens {
            match token {
                Token::FgColor(TermColor::Vga(r, g, b)) if *r == 12 && *g == 34 && *b == 56 => {
                    found_fg = true
                }
                Token::BgColor(TermColor::Vga(r, g, b)) if *r == 78 && *g == 90 && *b == 123 => {
                    found_bg = true
                }
                _ => {}
            }
        }
        assert!(found_fg, "24-bit foreground color not parsed");
        assert!(found_bg, "24-bit background color not parsed");
    }

    #[test]
    fn test_hyperlink_token() {
        // This test checks that OSC 8 hyperlinks are parsed into Hyperlink tokens
        let test_str = "before \u{1b}]8;;https://example.com\u{7}link\u{1b}]8;;\u{7} after";
        let tokens = get_tokens_test(test_str);
        let mut found_hyperlink = false;
        for token in &tokens {
            if let Token::Hyperlink(link_start, link_end, text_start, text_end) = token {
                let link = &test_str[*link_start..*link_end];
                let link_text = &test_str[*text_start..*text_end];
                assert_eq!(link, "https://example.com");
                assert_eq!(link_text, "link");
                found_hyperlink = true;
            }
        }
        assert!(found_hyperlink, "Hyperlink token not found");
    }

    #[test]
    fn test_multi_lines() {
        init_logs();
        let s = r#"JS ERROR: Error: Impossible to remove untracked message
_removeMessage@resource:///org/gnome/shell/ui/messageList.js:1599:19
_removePlayer@resource:///org/gnome/shell/ui/messageList.js:1773:14
_setupMpris/<@resource:///org/gnome/shell/ui/messageList.js:1757:51
_onNameOwnerChanged@resource:///org/gnome/shell/ui/mpris.js:247:22
_callHandlers@resource:///org/gnome/gjs/modules/core/_signals.js:130:42
_emit@resource:///org/gnome/gjs/modules/core/_signals.js:119:10
_convertToNativeSignal@resource:///org/gnome/gjs/modules/core/overrides/Gio.js:153:19
@resource:///org/gnome/shell/ui/init.js:21:20"#;
        let lines = s.lines();
        let mut token_list = Vec::<Token>::new();
        for line in lines {
            info!("{line}");
            get_tokens(&mut token_list, line);
            info!("{:?} line len {}", token_list, line.len());
        }
    }
}
