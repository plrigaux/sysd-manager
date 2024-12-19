
#[derive(Debug, Clone)]
pub(super) enum InfoToken<'a> {
    Char(char),
    Text(String),
    TextStr(&'a str),
    NewLine,
    HyperLink(String, String),
    InfoActive(String),
    InfoDisable(String),
}