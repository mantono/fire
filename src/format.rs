use std::fmt::format;

use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

pub trait ContentFormatter {
    fn accept(&self, content_type: Option<&str>) -> bool;
    fn format(&self, content: String) -> Result<String, String>;
}

pub fn formatters() -> Vec<Box<dyn ContentFormatter>> {
    let theme_set = ThemeSet::load_defaults();
    let theme: Theme = theme_set.themes["base16-mocha.dark"].clone();
    vec![
        Box::new(JsonPretty::new()),
        Box::new(JsonSyntax::new(theme.clone())),
        Box::new(XmlSyntax::new(theme.clone())),
    ]
}

pub struct JsonSyntax {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl JsonSyntax {
    pub fn new(theme: Theme) -> JsonSyntax {
        JsonSyntax {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme,
        }
    }
}

impl ContentFormatter for JsonSyntax {
    fn accept(&self, content_type: Option<&str>) -> bool {
        match content_type {
            Some(ct) => ct.starts_with("application/json"),
            None => false,
        }
    }

    fn format(&self, content: String) -> Result<String, String> {
        let syntax = self.syntax_set.find_syntax_by_extension("json").unwrap();
        let mut high = HighlightLines::new(&syntax, &self.theme);
        let mut out: Vec<String> = Vec::with_capacity(512);
        for line in LinesWithEndings::from(&content) {
            let ranges: Vec<(Style, &str)> = high.highlight_line(line, &self.syntax_set).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            out.push(escaped);
        }
        Ok(out.as_slice().join(""))
    }
}

pub struct JsonPretty;

impl JsonPretty {
    pub fn new() -> JsonPretty {
        JsonPretty
    }
}

impl ContentFormatter for JsonPretty {
    fn accept(&self, content_type: Option<&str>) -> bool {
        match content_type {
            Some(ct) => ct.starts_with("application/json"),
            None => false,
        }
    }

    fn format(&self, content: String) -> Result<String, String> {
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Unable to parse body as JSON: {:?}", e))?;
        Ok(serde_json::to_string_pretty(&json).unwrap())
    }
}

pub struct XmlSyntax {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl XmlSyntax {
    pub fn new(theme: Theme) -> XmlSyntax {
        XmlSyntax {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme,
        }
    }
}

impl ContentFormatter for XmlSyntax {
    fn accept(&self, content_type: Option<&str>) -> bool {
        match content_type {
            Some(ct) => ct.starts_with("text/html") || ct.starts_with("text/xml"),
            None => false,
        }
    }

    fn format(&self, content: String) -> Result<String, String> {
        let syntax = self.syntax_set.find_syntax_by_extension("xml").unwrap();
        let mut high = HighlightLines::new(&syntax, &self.theme);
        let mut out: Vec<String> = Vec::with_capacity(512);
        for line in LinesWithEndings::from(&content) {
            let ranges: Vec<(Style, &str)> = high.highlight_line(line, &self.syntax_set).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            out.push(escaped);
        }
        Ok(out.as_slice().join(""))
    }
}
