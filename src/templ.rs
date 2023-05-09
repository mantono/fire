use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;

lazy_static! {
    static ref VAR_REGEX: Regex = Regex::new(r"\{\{[A-Za-z0-9_-]{1,32}\}\}").unwrap();
}

pub fn find_keys(template: &str) -> HashSet<String> {
    VAR_REGEX
        .find_iter(template)
        .map(|m: regex::Match| trim_braces(m.as_str()).to_string())
        .collect()
}

fn trim_braces(input: &str) -> &str {
    let end: usize = input.len() - 2;
    input.get(2..end).unwrap()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::templ;

    #[test]
    fn find_template_keys() {
        let template = "{{FOO}} {{}}- {{{}}} {{  }} {{BAR}}";
        let keys: HashSet<String> = templ::find_keys(&template);
        let expected: HashSet<String> =
            [String::from("FOO"), String::from("BAR")].into_iter().collect();
        assert_eq!(expected, keys);
    }
}
