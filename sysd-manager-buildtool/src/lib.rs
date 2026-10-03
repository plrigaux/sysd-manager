pub mod errors;

use std::{
    borrow::Cow,
    fmt::Display,
    fs::File,
    io::{self, BufRead, Cursor},
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex, OnceLock},
};

use quick_xml::{
    Writer,
    events::{BytesDecl, BytesText, Event},
};
use regex::Regex;
use tracing::{debug, info, warn};

use crate::errors::ToolError;

// use crate::errors::ToolError;

// use crate::errors::ToolError;

pub fn find_change_log_file(dir_path: &Path, file_name: &str) -> Result<PathBuf, ToolError> {
    let path = dir_path.join(file_name);

    if !path.exists() {
        if let Some(parent_dir) = dir_path.parent() {
            find_change_log_file(parent_dir, file_name)
        } else {
            Err(ToolError::FileNotFound)
        }
    } else {
        Ok(path)
    }
}

#[derive(Debug, Default)]
pub struct Version {
    major: u16,
    minor: u16,
    patch: u16,
}

impl Version {
    fn new(major: &str, minor: &str, patch: &str) -> Self {
        Self {
            major: major.parse::<u16>().unwrap(),
            minor: minor.parse::<u16>().unwrap(),
            patch: patch.parse::<u16>().unwrap(),
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Default)]
pub struct Release {
    pub date: String,
    pub version: Version,
    logs: Vec<LogToken>,
}

impl Release {
    fn new(version: Version, date: &str) -> Self {
        Self {
            version,
            date: date.to_owned(),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
enum LogToken {
    Item(String),
    Section(String),
}

pub fn extract_changelog(file_path: &Path) -> Result<Vec<Release>, ToolError> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    let re_str = r#"## \[([0-9\.]+)\] - ([0-9\-]+)"#;
    let versionline_re = Regex::new(re_str).expect("Valid RegEx");
    let version_re = Regex::new(r"(\d+)\.(\d+)\.(\d+)").expect("Valid RegEx");
    let date_re = Regex::new(r"\d{4}-\d{2}-\d{2}").expect("Valid RegEx");

    let title_re = Regex::new(r"###\s*(.+)").expect("Valid RegEx");
    let item_re = Regex::new(r"^- (.+)\s*").expect("Valid RegEx");
    let follow_line_re = Regex::new(r"^\s*\S+").expect("Valid RegEx");

    let mut vlogs = Vec::new();

    let mut item_row = false;
    for line in reader.lines() {
        let line = line?;

        if let Some(cap) = versionline_re.captures(&line) {
            let date = &cap[2];
            let version = &cap[1];
            info!("version {} date {}", version, date);

            let Some(vcap) = version_re.captures(version) else {
                warn!("Invalid version {}", version);
                continue;
            };

            let ver = Version::new(&vcap[1], &vcap[2], &vcap[3]);

            let Some(_dcap) = date_re.captures(date) else {
                warn!("Invalid date {}", date);
                continue;
            };

            let vlog = Release::new(ver, date);
            vlogs.push(vlog);
        } else if let Some(tcap) = title_re.captures(&line) {
            info!("Section {}", &tcap[1]);
            if let Some(vlog) = vlogs.last_mut() {
                vlog.logs.push(LogToken::Section(tcap[1].to_owned()))
            }
            item_row = false;
        } else if let Some(capture) = item_re.captures(&line) {
            debug!("Item {}", &capture[1]);
            if let Some(vlog) = vlogs.last_mut() {
                vlog.logs.push(LogToken::Item(capture[1].to_owned()))
            }
            item_row = true;
        } else if item_row && follow_line_re.is_match(&line) {
            warn!("LINE:{}", line);
            if let Some(vlog) = vlogs.last_mut()
                && let Some(LogToken::Item(pizza)) = vlog.logs.last_mut()
            {
                pizza.push(' ');
                pizza.push_str(line.trim_ascii());
            };
        }
    }

    // write_xml(file_path.parent().unwrap(), &vlogs)?;
    Ok(vlogs)
}

pub fn write_releases_to_xml(file_path: &Path, logs: &[Release]) -> Result<(), ToolError> {
    // let vec = Vec::new();

    let file = File::create(file_path)?;

    let mut writer = Writer::new_with_indent(file, b'\t', 1);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    writer.write_event(Event::Comment(BytesText::new(
        "File generated from CHANGELOG.md",
    )))?;
    writer
        .create_element("releases")
        .write_inner_content(|writer| {
            for release in logs {
                writer
                    .create_element("release")
                    .with_attributes([
                        ("version", release.version.to_string().as_str()),
                        ("date", release.date.as_str()),
                    ])
                    // .write_empty()?;
                    // .write_text_content(BytesText::new("test test"))?
                    .write_inner_content(|writer| {
                        writer
                            .create_element("description")
                            // .with_attribute(("translate", "no"))
                            .write_inner_content(|writer| {
                                // inner_release(writer, release, "heading")
                                inner_release(writer, release, "p")
                            })?;

                        if let Ok(mut issues) = ISSUES.lock()
                            && !issues.is_empty()
                        {
                            writer
                                .create_element("issues")
                                .write_inner_content(|writer| {
                                    for (label, url) in issues.iter() {
                                        writer
                                            .create_element("issue")
                                            .with_attribute(("url", url.as_str()))
                                            .write_text_content(BytesText::new(label))?;
                                    }
                                    Ok(())
                                })?;
                            issues.clear();
                        }
                        Ok(())
                    })?;
            }
            Ok(())
        })?;
    Ok(())
}

static LI_RE: OnceLock<Regex> = OnceLock::new();

fn get_config() -> &'static Regex {
    LI_RE.get_or_init(|| Regex::new(r#"\[(.*?)\]\((.*?)\)"#).expect("Valid RegEx"))
}

static ISSUES: LazyLock<Mutex<Vec<(String, String)>>> = LazyLock::new(|| Mutex::new(Vec::new()));

fn add_issues(label: &str, url: &str) {
    if let Ok(mut issues) = ISSUES.lock() {
        issues.push((label.to_owned(), url.to_owned()))
    }
}

fn clean_li<'a>(li: &'a str) -> Cow<'a, str> {
    let mut it = get_config().captures_iter(li).peekable();

    if it.peek().is_none() {
        return Cow::Borrowed(li);
    }

    let mut out_string = String::with_capacity(li.len());
    let mut start = 0;
    for capture in it {
        let m = capture.get_match();
        let m1 = capture.get(1).unwrap();
        out_string.push_str(&li[start..m1.end() + 1]);
        start = m.end();

        let label = &capture[1];
        let url = &capture[2];

        add_issues(label, url);
    }
    out_string.push_str(&li[start..]);

    debug!("-- {}", out_string);
    Cow::Owned(out_string)
}

pub fn get_about_what_change(last_release: &Release) -> Result<String, ToolError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    inner_release(&mut writer, last_release, "p")?;

    let bytes = writer.into_inner().into_inner();

    let string = String::from_utf8(bytes)?;
    Ok(string)
}

fn inner_release<T>(
    writer: &mut Writer<T>,
    release: &Release,
    header_tag: &str,
) -> Result<(), io::Error>
where
    T: std::io::Write,
{
    let mut it = release.logs.iter().peekable();
    while let Some(token) = it.next() {
        match token {
            LogToken::Section(title) => {
                writer
                    .create_element(header_tag)
                    .write_text_content(BytesText::new(title))?;
            }
            LogToken::Item(li) => {
                let mut items = Vec::new();
                items.push(li);
                while let Some(LogToken::Item(li)) = it.peek() {
                    items.push(li);
                    it.next();
                }

                writer
                    .create_element("ul")
                    .write_inner_content(move |writer| {
                        for li in items {
                            let li = clean_li(li);
                            let li = collapse_line(&li);
                            writer
                                .create_element("li")
                                .write_text_content(BytesText::new(&li))?;
                        }

                        Ok(())
                    })
                    .ok();
            }
        };
    }

    Ok(())
}

fn collapse_line<'a>(line: &'a str) -> Cow<'a, str> {
    const LINE_SIZE: usize = 80;

    if line.len() <= LINE_SIZE {
        return Cow::Borrowed(line);
    }

    let mut out = String::from(line);

    let mut last_wsb = 0;
    let b = unsafe { out.as_bytes_mut() };

    let mut i = 0;
    let mut offset = 0;
    while i < b.len() {
        let bc = b[i];
        if matches!(bc, b' ' | b'\t') {
            last_wsb = i;
        }

        if offset > LINE_SIZE {
            b[last_wsb] = b'\n';
            offset = 0;
        }

        i += 1;
        offset += 1;
    }

    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use crate::collapse_line;

    use super::{clean_li, get_config};

    #[test]
    fn li_re_matches_markdown_links() {
        let re = get_config();
        let text = "some text [Fix crash](https://example.com/issues/1) otehr text";
        let caps = re.captures(text).expect("regex should match markdown link");

        assert_eq!(&caps[1], "Fix crash");
        assert_eq!(&caps[2], "https://example.com/issues/1");
    }

    #[test]
    fn clean_li_replaces_each_markdown_link_with_label() {
        let input = "Fix [crash](https://example.com/1) and [ui](https://example.com/2)";

        assert_eq!(clean_li(input), "Fix crash and ui");
    }

    #[test]
    fn test_callapse_line() {
        let line = "[32] Sed ut perspiciatis, unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam eaque ipsa, quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt, explicabo. Nemo enim ipsam voluptatem, quia voluptas sit, aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos, qui ratione voluptatem sequi nesciunt, neque porro quisquam est, qui dolorem ipsum, quia dolor sit amet consectetur adipisci[ng] velit, sed quia non numquam [do] eius modi tempora inci[di]dunt, ut labore et dolore magnam aliquam quaerat voluptatem. Ut enim ad minima veniam, quis nostrum[d] exercitationem ullam corporis suscipit laboriosam, nisi ut aliquid ex ea commodi consequatur? [D]Quis autem vel eum i[r]ure reprehenderit, qui in ea voluptate velit esse, quam nihil molestiae consequatur, vel illum, qui dolorem eum fugiat, quo voluptas nulla pariatur?";
        let out = collapse_line(line);

        println!("{}", out);
    }
}
