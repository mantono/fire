use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

pub trait ContentFormatter {
    fn accept(&self, content_type: Option<&str>) -> bool;
    fn format(&self, content: String) -> Result<String, String>;
}

pub fn formatters() -> Vec<Box<dyn ContentFormatter>> {
    vec![Box::new(JsonPretty::new()), Box::new(JsonSyntax::new())]
}

pub struct JsonSyntax {
    syntax_set: SyntaxSet,
    theme: ThemeSet,
}

impl JsonSyntax {
    pub fn new() -> JsonSyntax {
        JsonSyntax {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme: ThemeSet::load_defaults(),
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
        let mut high = HighlightLines::new(&syntax, &self.theme.themes["base16-ocean.dark"]);
        let mut out: Vec<String> = Vec::with_capacity(512);
        for line in LinesWithEndings::from(&content) {
            let ranges: Vec<(Style, &str)> = high.highlight_line(line, &self.syntax_set).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
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
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        Ok(serde_json::to_string_pretty(&json).unwrap())
    }
}
