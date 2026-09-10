use std::collections::HashSet;
use std::ops::Range;

const NAME_MAX_LEN: usize = 32;

/// A template reference discovered in a Handlebars-like template string, e.g.
/// `{{NAME}}`, `{{NAME:value}}`, or `{{NAME|command}}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub name: String,
    pub fallback: Fallback,
    pub span: Range<usize>,
}

/// The fallback carried by a template reference, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fallback {
    None,
    Static(String),
    Command(String),
}

/// Scan `template` and return every valid reference in occurrence order,
/// preserving the byte span (relative to `template`) each reference occupies.
///
/// Only well-formed references are yielded. Invalid, unterminated, or
/// non-name constructs (e.g. `{{}}`, `{{ }}`, or a construct missing its
/// closing `}}`) are left untouched as ordinary template text and are not
/// reported.
pub fn scan(template: &str) -> Vec<Reference> {
    let mut refs: Vec<Reference> = Vec::new();
    let mut i: usize = 0;
    let bytes: &[u8] = template.as_bytes();

    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some((reference, next)) = parse_reference(template, i) {
                refs.push(reference);
                i = next;
                continue;
            }
        }
        i += 1;
    }

    refs
}

/// Discover the distinct reference names present in `template`, regardless
/// of fallback form. This is the legacy key-discovery behavior, now backed
/// by [`scan`].
pub fn find_keys(template: &str) -> HashSet<String> {
    scan(template).into_iter().map(|reference: Reference| reference.name).collect()
}

fn parse_reference(template: &str, start: usize) -> Option<(Reference, usize)> {
    let name_start: usize = start + 2;
    let name_end: usize = parse_name_end(template, name_start);

    if name_end == name_start {
        return None;
    }

    let name: String = template.get(name_start..name_end)?.to_string();
    let bytes: &[u8] = template.as_bytes();

    match bytes.get(name_end) {
        Some(b'}') if bytes.get(name_end + 1) == Some(&b'}') => {
            let end: usize = name_end + 2;
            Some((
                Reference {
                    name,
                    fallback: Fallback::None,
                    span: start..end,
                },
                end,
            ))
        }
        Some(b':') => parse_delimited(template, start, name, name_end + 1, Fallback::Static),
        Some(b'|') => parse_delimited(template, start, name, name_end + 1, Fallback::Command),
        _ => None,
    }
}

fn parse_delimited(
    template: &str,
    start: usize,
    name: String,
    content_start: usize,
    ctor: fn(String) -> Fallback,
) -> Option<(Reference, usize)> {
    let rest: &str = template.get(content_start..)?;
    let close: usize = rest.find("}}")?;
    let content: String = rest.get(..close)?.to_string();
    let end: usize = content_start + close + 2;
    Some((
        Reference {
            name,
            fallback: ctor(content),
            span: start..end,
        },
        end,
    ))
}

fn parse_name_end(template: &str, start: usize) -> usize {
    let bytes: &[u8] = template.as_bytes();
    let mut i: usize = start;

    while i < bytes.len() && i - start < NAME_MAX_LEN && is_name_byte(bytes[i]) {
        i += 1;
    }

    i
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::templ::{self, Fallback, Reference};

    #[test]
    fn find_template_keys_legacy_discovery() {
        let template = "{{FOO}} {{}}- {{{}}} {{  }} {{BAR}}";
        let keys: HashSet<String> = templ::find_keys(template);
        let expected: HashSet<String> =
            [String::from("FOO"), String::from("BAR")].into_iter().collect();
        assert_eq!(expected, keys);
    }

    #[test]
    fn scan_plain_reference() {
        let template = "{{FOO}}";
        let refs: Vec<Reference> = templ::scan(template);
        assert_eq!(1, refs.len());
        assert_eq!("FOO", refs[0].name);
        assert_eq!(Fallback::None, refs[0].fallback);
        assert_eq!(0..7, refs[0].span);
    }

    #[test]
    fn scan_static_reference() {
        let template = "{{FOO:bar}}";
        let refs: Vec<Reference> = templ::scan(template);
        assert_eq!(1, refs.len());
        assert_eq!("FOO", refs[0].name);
        assert_eq!(Fallback::Static(String::from("bar")), refs[0].fallback);
        assert_eq!(0..11, refs[0].span);
    }

    #[test]
    fn scan_command_reference() {
        let template = "{{FOO|echo hi}}";
        let refs: Vec<Reference> = templ::scan(template);
        assert_eq!(1, refs.len());
        assert_eq!("FOO", refs[0].name);
        assert_eq!(Fallback::Command(String::from("echo hi")), refs[0].fallback);
        assert_eq!(0..15, refs[0].span);
    }

    #[test]
    fn scan_static_fallback_retains_colon_and_pipe_literally() {
        let template = "{{FOO:a:b|c}}";
        let refs: Vec<Reference> = templ::scan(template);
        assert_eq!(1, refs.len());
        assert_eq!(Fallback::Static(String::from("a:b|c")), refs[0].fallback);
    }

    #[test]
    fn scan_command_fallback_retains_colon_and_pipe_literally() {
        let template = "{{FOO|a:b|c}}";
        let refs: Vec<Reference> = templ::scan(template);
        assert_eq!(1, refs.len());
        assert_eq!(Fallback::Command(String::from("a:b|c")), refs[0].fallback);
    }

    #[test]
    fn scan_repeated_name_with_distinct_fallbacks() {
        let template = "{{FOO}} {{FOO:a}} {{FOO|b}}";
        let refs: Vec<Reference> = templ::scan(template);
        assert_eq!(3, refs.len());
        assert!(refs.iter().all(|r| r.name == "FOO"));
        assert_eq!(Fallback::None, refs[0].fallback);
        assert_eq!(Fallback::Static(String::from("a")), refs[1].fallback);
        assert_eq!(Fallback::Command(String::from("b")), refs[2].fallback);
        assert_eq!(0..7, refs[0].span);
        assert_eq!(8..17, refs[1].span);
        assert_eq!(18..27, refs[2].span);
    }

    #[test]
    fn scan_ignores_invalid_and_unterminated_constructs() {
        let template =
            "{{FOO}} {{}}- {{{}}} {{  }} {{BAR}} {{BAZ:unterminated {{QUX|also unterminated";
        let refs: Vec<Reference> = templ::scan(template);
        let names: HashSet<&str> = refs.iter().map(|r| r.name.as_str()).collect();
        let expected: HashSet<&str> = ["FOO", "BAR"].into_iter().collect();
        assert_eq!(expected, names);
    }

    #[test]
    fn scan_name_length_boundaries() {
        let name_32 = "a".repeat(32);
        let template_32 = format!("{{{{{}}}}}", name_32);
        let refs: Vec<Reference> = templ::scan(&template_32);
        assert_eq!(1, refs.len());
        assert_eq!(name_32, refs[0].name);

        let name_33 = "a".repeat(33);
        let template_33 = format!("{{{{{}}}}}", name_33);
        let refs: Vec<Reference> = templ::scan(&template_33);
        assert!(refs.is_empty());
    }

    #[test]
    fn find_keys_reflects_scanned_reference_names() {
        let template = "{{FOO}} {{FOO:a}} {{BAR|cmd}}";
        let keys: HashSet<String> = templ::find_keys(template);
        let expected: HashSet<String> =
            [String::from("FOO"), String::from("BAR")].into_iter().collect();
        assert_eq!(expected, keys);
    }
}
