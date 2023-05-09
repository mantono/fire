use core::panic;
use std::collections::{HashMap, HashSet, VecDeque};

pub fn find_keys(template: &str) -> HashSet<String> {
    let mut keys: HashSet<String> = HashSet::with_capacity(8);
    let mut braces: (u8, u8) = (0, 0);
    let mut state: Vec<char> = Vec::with_capacity(32);

    for char in template.chars() {
        let token: Token = char.into();
        match braces {
            (0, 0) => match token {
                Token::LeftBrace => braces = (1, 0),
                _ => state.clear(),
            },
            (1, 0) => match token {
                Token::LeftBrace => braces = (2, 0),
                _ => state.clear(),
            },
            (2, 0) => match token {
                Token::LeftBrace => (),
                Token::RightBrace => {
                    if state.is_empty() {
                        braces = (0, 0)
                    } else {
                        braces = (2, 1)
                    }
                }
                Token::Space => braces = (0, 0),
                Token::IdenChar(c) => state.push(c),
                Token::OtherChar(_) => braces = (0, 0),
            },
            (2, 1) => match token {
                Token::LeftBrace => braces = (0, 0),
                Token::RightBrace => {
                    braces = (0, 0);
                    if !state.is_empty() {
                        let value: String = state.iter().collect();
                        keys.insert(value);
                        state.clear();
                    }
                }
                Token::Space => braces = (0, 0),
                Token::IdenChar(c) => state.push(c),
                Token::OtherChar(_) => braces = (0, 0),
            },
            (_, _) => panic!("Braces ran out of control"),
        }
    }

    keys
}

//pub fn substitute(templ: &str, vars: &HashMap<String, String>) -> Result<String, Error> {
//    let mut tokens: Vec<Token> = Vec::with_capacity(templ.len());
//    for x in templ.chars() {
//        tokens.push(dbg!(x.into()))
//    }
//    let mut parser = Parser::new();
//    for t in tokens {
//        if let Err(e) = parser.push(t) {
//            return Err(Error::Syntax(e));
//        }
//    }
//    Ok(String::from("foo"))
//}
//
//const BRACE_LEFT: char = '{';
//const BRACE_RIGHT: char = '}';

//enum Token {
//    Other(char),
//    Ident(String)
//}

//struct State {
//    left: u8,
//    right: u8,
//    ident: VecDeque<char>,
//}
//
//impl State {
//    pub fn push(input: char) -> Result<Option<Token>, String> {
//        match (left, right) {
//            (0, 0) => {}
//        }
//    }
//}

#[derive(Debug, Clone, Copy)]
enum Token {
    /// {
    LeftBrace,
    /// }
    RightBrace,
    /// ' '
    Space,
    /// a-z, A-Z, 0-9, _, -
    IdenChar(char),
    /// Everything else
    OtherChar(char),
}

impl From<char> for Token {
    fn from(c: char) -> Token {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => Token::IdenChar(c),
            ' ' => Token::Space,
            '{' => Token::LeftBrace,
            '}' => Token::RightBrace,
            _ => Token::OtherChar(c),
        }
    }
}

//struct Parser {
//    state: State,
//    stack: VecDeque<char>,
//    completed: VecDeque<Content>,
//}
//
//impl Parser {
//    pub fn new() -> Parser {
//        Self {
//            state: State::Empty,
//            stack: VecDeque::new(),
//            completed: VecDeque::new(),
//        }
//    }
//
//    pub fn push(&mut self, token: Token) -> Result<(), String> {
//        match self.state {
//            State::Empty => match token {
//                Token::LeftBrace => {
//                    self.state = State::LeftBraceFirst;
//                    Ok(())
//                }
//                Token::Space => {
//                    self.stack.push_back(' ');
//                    Ok(())
//                }
//                Token::IdenChar(c) | Token::OtherChar(c) => {
//                    self.stack.push_back(c);
//                    Ok(())
//                }
//                Token::RightBrace => {
//                    self.stack.push_back('}');
//                    Ok(())
//                }
//            },
//            State::LeftBraceFirst =>
//        }
//    }
//
//    pub fn content(self) -> VecDeque<Content> {
//        self.completed
//    }
//}
//
//#[derive(Debug, Clone)]
//enum Content {
//    Text(String),
//    Variable(String),
//}
//
//enum State {
//    // State is empty
//    Empty,
//    // {
//    LeftBraceFirst,
//    // {{
//    LeftBraceSecond,
//    // }
//    RightBraceFirst,
//    // FOO in {{FOO}}
//    Ident,
//}

#[derive(Debug)]
pub enum Error {
    Syntax(String),
    MissingKey(String),
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::templ;

    #[test]
    fn find_template_keys() {
        let template = "{{FOO}} {{}}- {{{}}} {{  }} {{BAR}}";
        let keys: std::collections::HashSet<String> = templ::find_keys(&template);
        let expected: std::collections::HashSet<String> =
            [String::from("FOO"), String::from("BAR")].into_iter().collect();
        assert_eq!(expected, keys);
    }
}
