use std::num::ParseIntError;

#[derive(Debug, PartialEq, PartialOrd)]
pub enum TokenType {
    LeftParen,    // x
    RightParen,   // x
    LeftBrace,    // x
    RightBrace,   // x
    RightBracket, // x
    LeftBracket,  // x

    Comma,
    Dot,
    DotDot,
    Minus,
    Plus,
    Semicolon, // x
    Glob,
    Pound, // x
    Ampersand,
    Pipe, // x
    Shebang,
    Backslash, // x
    Forwardslash,
    OutputRedirect, //x
    AppendRedirect, // x
    InputRedirect,  //x

    Bang,  // x
    Equal, // x

    GlobbedWord(String),
    Word(String),
    DoubleQuotedString(String), // x
    SingleQuotedString(String), // x
    VariableExpansion(String),
    SubshellExpansion(Option<Vec<Token>>),
    Integer(i64),
    Float(f32),
    RangeExpressionNumeric(i64, i64, Option<i64>),
    RangeExpressionAlphabetic(char, char, Option<i64>),

    EOF,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub struct Token {
    token_type: TokenType,
}

impl Token {
    fn new(token_type: TokenType) -> Token {
        Token { token_type }
    }
}

pub struct Scanner {
    source: String,
    start: usize,
    current: usize,
    had_error: bool,
    tokens: Vec<Token>,
}

impl Scanner {
    pub fn new(source: String) -> Scanner {
        Scanner {
            source: source.trim().to_string(),
            start: 0,
            current: 0,
            had_error: false,
            tokens: Vec::new(),
        }
    }

    pub fn get_tokens(mut self) -> Option<Vec<Token>> {
        self.scan_tokens();
        if self.had_error {
            None
        } else {
            Some(self.tokens)
        }
    }
    fn scan_tokens(&mut self) {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token()
        }
        self.tokens.push(Token::new(TokenType::EOF));
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn scan_token(&mut self) {
        if let Some(c) = self.next_char() {
            match c {
                '(' => self.add_token(TokenType::LeftParen),
                ')' => self.add_token(TokenType::RightParen),
                '[' => self.add_token(TokenType::LeftBracket),
                ']' => self.add_token(TokenType::RightBracket),
                '{' => {
                    self.parse_range_expression();
                }
                '}' => self.add_token(TokenType::RightBrace),
                ',' => self.add_token(TokenType::Comma),
                '+' => self.add_token(TokenType::Plus),
                ';' => self.add_token(TokenType::Semicolon),
                '|' => self.add_token(TokenType::Pipe),
                '$' => {
                    if self.peek().is_some_and(|c| c == '(') {
                        self.parse_subshell_expansion();
                    } else if self.peek().is_some_and(|c| c == '{') {
                        self.increment_current(); // get rid of $
                        self.parse_variable(); // passed will be {something}
                        self.increment_current(); // get rid of trailing }
                    } else if self.peek().is_some_and(|c| allowed_name_char(c)) {
                        self.parse_variable();
                    } else {
                        self.emit_error(" expand what?");
                    }
                }
                '<' => self.add_token(TokenType::InputRedirect),
                '\\' => self.add_token(TokenType::Backslash),
                '/' => self.add_token(TokenType::Forwardslash),
                '\t' | '\n' | 'r' | ' ' => return,
                '"' => {
                    while self.peek().is_some_and(|c| c != '"') {
                        self.increment_current();
                        if self.peek().is_none() {
                            self.emit_error("Unterminated string literal");
                        }
                    }
                    self.add_token(TokenType::DoubleQuotedString(
                        self.source[self.start + 1..self.current].to_string(),
                    ));
                    self.increment_current();
                }
                '\'' => {
                    while self.peek().is_some_and(|c| c != '\'') {
                        self.increment_current();
                        if self.peek().is_none() {
                            self.emit_error("Unterminated string literal");
                        }
                    }
                    self.add_token(TokenType::SingleQuotedString(
                        self.source[self.start + 1..self.current].to_string(),
                    ));
                    self.increment_current();
                }
                '*' => {
                    if self.peek().is_some_and(|c| c.is_whitespace()) {
                        // stand alone glob
                        self.add_token(TokenType::Glob);
                    } else {
                        self.parse_word()
                    }
                }
                '#' => {
                    if self.peek().is_some_and(|c| c == '!') {
                        self.add_token(TokenType::Shebang);
                        self.increment_current();
                    } else {
                        self.add_token(TokenType::Pound);
                    }
                }
                '!' => {
                    if self.peek().is_some_and(|c| c.is_whitespace()) {
                        self.add_token(TokenType::Bang);
                    } else {
                        self.parse_word();
                    }
                }
                '&' => {
                    self.add_token(TokenType::Ampersand);
                }

                '.' => {
                    if self.peek().is_some_and(|c| c == '.') {
                        if self.peek_next().is_some_and(|c| c.is_whitespace())
                            || self.peek_next().is_none()
                        {
                            self.add_token(TokenType::DotDot);
                            self.increment_current();
                        } else {
                            self.parse_word();
                        }
                    } else {
                        if self.peek().is_some_and(|c| c.is_whitespace()) || self.peek().is_none() {
                            self.add_token(TokenType::Dot);
                        } else {
                            self.parse_word();
                        }
                    }
                }
                '=' => {
                    if self.peek().is_some_and(|c| c.is_whitespace())
                        || self.peek_prev().is_some_and(|c| c.is_whitespace())
                    {
                        self.emit_error(" whitespace around equals");
                    } else if self.peek().is_none() {
                        self.emit_error(" equals what?");
                    } else if self.peek_prev().is_none() {
                        self.emit_error(" what equals?");
                    } else {
                        self.add_token(TokenType::Equal);
                    }
                }
                '>' => {
                    if self.peek().is_some_and(|c| c == '>') {
                        self.add_token(TokenType::AppendRedirect);
                        self.increment_current();
                    } else {
                        self.add_token(TokenType::OutputRedirect);
                    }
                }
                '-' => {
                    let next = self.peek();
                    if next.is_some_and(|c| c.is_ascii_alphabetic() || c == '-') {
                        self.parse_word();
                    } else if next.is_some_and(|c| c.is_numeric()) {
                        self.parse_number()
                    } else {
                        self.add_token(TokenType::Minus)
                    }
                }
                default => {
                    if default.is_numeric() {
                        self.parse_number()
                    } else if default.is_ascii_alphabetic() {
                        self.parse_word()
                    } else {
                        self.emit_error(&format!(" invalid character: \'{}\'", default));
                    }
                }
            }
        }
    }
    fn emit_error(&mut self, message: &str) {
        self.had_error = true;
        let space = " ".repeat(self.current - 1);
        eprintln!("{}", self.source);
        eprintln!("{}\x1b[;31m^{}\x1b[;37m", space, message);
    }
    pub fn next_char(&mut self) -> Option<char> {
        let ret = self.source.chars().nth(self.current);
        self.increment_current();
        ret
    }
    pub fn peek(&self) -> Option<char> {
        self.source.chars().nth(self.current)
    }
    pub fn peek_prev(&self) -> Option<char> {
        if self.current == 0 {
            None
        } else {
            self.source.chars().nth(self.current - 1)
        }
    }
    pub fn peek_next(&self) -> Option<char> {
        self.source.chars().nth(self.current + 1)
    }
    fn parse_word(&mut self) {
        while self.peek().is_some_and(|c| {
            !c.is_whitespace() && !is_pair_delimiter(c) && !is_special_character(c)
        }) {
            self.increment_current()
        }
        if self.source[self.start..self.current].contains('*') {
            self.add_token(TokenType::GlobbedWord(
                self.source[self.start..self.current].to_string(),
            ));
        } else {
            self.add_token(TokenType::Word(
                self.source[self.start..self.current].to_string(),
            ));
        }
    }
    fn parse_variable(&mut self) {
        while self.peek().is_some_and(|c| allowed_name_char(c)) {
            self.increment_current();
        }
        self.add_token(TokenType::VariableExpansion(
            self.source[self.start + 1..self.current].to_string(),
        ));
    }

    fn parse_subshell_expansion(&mut self) {
        if let Some(pair_close) = get_pair_match(self.peek().unwrap()) {
            let pair_open = self.peek().unwrap();
            let mut paren_stack: Vec<char> = vec![];

            while self.peek().is_some() {
                if self.peek().unwrap() == pair_open {
                    paren_stack.push(pair_open);
                } else if self.peek().unwrap() == pair_close {
                    if let Some(&top) = paren_stack.last() {
                        if top == pair_open {
                            paren_stack.pop();
                        }
                    }
                }
                if paren_stack.is_empty() {
                    break;
                }

                self.increment_current();
            }
            if !paren_stack.is_empty() {
                self.emit_error(" unmatched pair");
            } else {
                println!("{}", self.source[self.start + 2..self.current].to_string());
                let scanner = Scanner::new(self.source[self.start + 2..self.current].to_string());
                self.add_token(TokenType::SubshellExpansion(scanner.get_tokens()));
            }
            self.increment_current();
        }
    }
    fn parse_number(&mut self) {
        while self.peek().is_some_and(|c| c.is_numeric()) {
            self.increment_current()
        }

        if self.peek().is_some_and(|c| c.is_alphabetic()) {
            self.current = self.start;
            self.parse_word();
        } else if self
            .peek()
            .is_some_and(|c| c == '.' && self.peek_next().is_some_and(|c| c.is_numeric()))
        {
            self.increment_current();
            while self.peek().is_some_and(|c| c.is_numeric()) {
                self.increment_current()
            }
            let num = self.source[self.start..self.current]
                .parse::<f32>()
                .unwrap_or(0.0);

            self.add_token(TokenType::Float(num));
            if self.peek().is_some_and(|c| !c.is_whitespace()) {
                self.current = self.start;
                self.parse_word();
            }
        } else {
            let num: i64 = self.source[self.start..self.current].parse().unwrap_or(0);
            self.add_token(TokenType::Integer(num));
        }
    }
    fn parse_and_get_integer(&mut self) -> Result<i64, ParseIntError> {
        self.start = self.current;
        while self.peek().is_some_and(|c| c.is_numeric()) {
            self.increment_current();
        }
        if self.current < self.source.len() {
            self.source[self.start..self.current].parse()
        } else {
            "a".parse()
        }
    }

    fn parse_range_expression(&mut self) {
        if self.peek().is_some_and(|c| c.is_numeric()) {
            // we are parsing a RangeExpressionNumeric
            let start = self.parse_and_get_integer();
            if self.peek().is_some_and(|c| c != '.') && self.peek_next().is_some_and(|c| c != '.') {
                self.emit_error("range expressions can take the form {i..i..i} or {a..a..i} (where \'i\' is an integer, and \'a\' is a character)");
                return;
            } else {
                self.increment_n(2);
            }
            let end = self.parse_and_get_integer();

            if end.is_err() || start.is_err() {
                self.emit_error(" error parsing range expressions");
                return;
            }

            if self.peek().is_some_and(|c| c == '}') {
                self.add_token(TokenType::RangeExpressionNumeric(
                    start.unwrap(),
                    end.unwrap(),
                    None,
                ));
                self.increment_current();
                return;
            }

            if self.peek().is_some_and(|c| c != '.') && self.peek_next().is_some_and(|c| c != '.') {
                self.emit_error("range expressions can take the form {i..i}, {a..a}, {i..i..i} or {a..a..i} (where \'i\' is an integer, and \'a\' is a character)");
                return;
            }
            self.increment_n(2);

            let by;
            if self.peek().is_some_and(|c| c.is_numeric()) {
                by = self.parse_and_get_integer();
                if by.is_err() {
                    self.emit_error(" error parsing range expressions");
                    return;
                } else {
                    self.add_token(TokenType::RangeExpressionNumeric(
                        start.unwrap(),
                        end.unwrap(),
                        Some(by.unwrap()),
                    ));
                    self.increment_current();
                    return;
                }
            } else {
                self.increment_current();
                self.emit_error("range expressions can take the form {i..i..i} or {a..a..i} (where \'i\' is an integer, and \'a\' is a character)");
                return;
            }
        } else if self.peek().is_some_and(|c| c.is_alphabetic()) {
            // we are parsing a RangeExpressionAlphabetic
            let start = self.peek().unwrap();
            if self.peek().is_some_and(|c| c != '.') && self.peek_next().is_some_and(|c| c != '.') {
                self.emit_error("must have \'..\', range expressions can take the form {i..i..i} or {a..a..i} (where \'i\' is an integer, and \'a\' is a character)");
                return;
            }
            self.increment_n(3);
            let end: char;
            if self.peek().is_some_and(|c| c.is_alphabetic()) {
                end = self.peek().unwrap();
            } else {
                self.emit_error("range expressions can take the form {i..i..i} or {a..a..i} (where \'i\' is an integer, and \'a\' is a character)");
                return;
            }
            self.increment_current(); // on second alpha

            if self.peek().is_some_and(|c| c == '}') {
                self.add_token(TokenType::RangeExpressionAlphabetic(start, end, None));
                self.increment_current();
                return;
            }

            if self.peek().is_some_and(|c| c != '.') && self.peek_next().is_some_and(|c| c != '.') {
                self.emit_error("must have \'..\', range expressions can take the form {i..i..i} or {a..a..i} (where \'i\' is an integer, and \'a\' is a character)");
                return;
            }
            self.increment_n(2);

            let by;
            if self.peek().is_some_and(|c| c.is_numeric()) {
                by = self.parse_and_get_integer();
                if by.is_err() {
                    self.emit_error(" error parsing range expressions");
                    return;
                } else {
                    self.add_token(TokenType::RangeExpressionAlphabetic(
                        start,
                        end,
                        Some(by.unwrap()),
                    ));
                    self.increment_current();
                    return;
                }
            } else {
                self.increment_current();
                self.emit_error("range expressions can take the form {i..i..i} or {a..a..i} (where \'i\' is an integer, and \'a\' is a character)");
                return;
            }
        } else {
            self.emit_error("range expressions can take the form {i..i..i} or {a..a..i} (where \'i\' is an integer, and \'a\' is a character)");
        }
    }

    fn add_token(&mut self, tok_type: TokenType) {
        self.tokens.push(Token::new(tok_type));
    }

    fn increment_current(&mut self) {
        self.current = self.current + 1;
    }

    fn increment_n(&mut self, n: usize) {
        self.current = self.current + n;
    }
}

pub fn is_pair_delimiter(c: char) -> bool {
    match c {
        ')' | '}' | ']' | '(' | '{' | '[' | '"' | '\'' => true,
        _ => false,
    }
}
pub fn is_group_opener(c: char) -> bool {
    match c {
        '(' | '{' | '[' => true,
        _ => false,
    }
}
pub fn get_pair_match(c: char) -> Option<char> {
    match c {
        '(' => Some(')'),
        '{' => Some('}'),
        '[' => Some(']'),
        ')' => Some('('),
        '}' => Some('{'),
        ']' => Some('{'),
        _ => None,
    }
}
pub fn is_group_closer(c: char) -> bool {
    match c {
        ')' | '}' | ']' => true,
        _ => false,
    }
}
pub fn is_special_character(c: char) -> bool {
    match c {
        '$' | '&' | '!' | ';' | '=' => true,
        _ => false,
    }
}
pub fn allowed_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn get_char() {
        let mut scan = Scanner::new("abcdef".to_string());
        assert_eq!(scan.next_char(), Some('a'));
        assert_eq!(scan.next_char(), Some('b'));
        assert_eq!(scan.next_char(), Some('c'));
        assert_eq!(scan.next_char(), Some('d'));
        assert_eq!(scan.next_char(), Some('e'));
        assert_eq!(scan.next_char(), Some('f'));
    }
    #[test]
    fn single_char_tokens() {
        let scan = Scanner::new("(".to_string());
        let expected = vec![Token::new(TokenType::LeftParen), Token::new(TokenType::EOF)];
        assert_eq!(Some(expected), scan.get_tokens());
    }
    #[test]
    fn two_char_tokens() {
        let scan = Scanner::new("==".to_string());
        assert_eq!(None, scan.get_tokens());
    }

    #[test]
    fn float() {
        let scan = Scanner::new("1.23".to_string());
        let expected = vec![
            Token::new(TokenType::Float(1.23)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("1.23 0.5 0.75 0.111".to_string());
        let expected = vec![
            Token::new(TokenType::Float(1.23)),
            Token::new(TokenType::Float(0.5)),
            Token::new(TokenType::Float(0.75)),
            Token::new(TokenType::Float(0.111)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());
    }

    #[test]
    fn integer() {
        let scan = Scanner::new("123".to_string());
        let expected = vec![
            Token::new(TokenType::Integer(123)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("1 2".to_string());
        let expected = vec![
            Token::new(TokenType::Integer(1)),
            Token::new(TokenType::Integer(2)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("1 2 3 4 567".to_string());
        let expected = vec![
            Token::new(TokenType::Integer(1)),
            Token::new(TokenType::Integer(2)),
            Token::new(TokenType::Integer(3)),
            Token::new(TokenType::Integer(4)),
            Token::new(TokenType::Integer(567)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());
    }
    #[test]
    fn commands() {
        let scan = Scanner::new("cd ..".to_string());
        let expected = vec![
            Token::new(TokenType::Word("cd".to_string())),
            Token::new(TokenType::DotDot),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("ls -a | grep file.txt".to_string());
        let expected = vec![
            Token::new(TokenType::Word("ls".to_string())),
            Token::new(TokenType::Word("-a".to_string())),
            Token::new(TokenType::Pipe),
            Token::new(TokenType::Word("grep".to_string())),
            Token::new(TokenType::Word("file.txt".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("ls *.csv | grep mnist".to_string());
        let expected = vec![
            Token::new(TokenType::Word("ls".to_string())),
            Token::new(TokenType::GlobbedWord("*.csv".to_string())),
            Token::new(TokenType::Pipe),
            Token::new(TokenType::Word("grep".to_string())),
            Token::new(TokenType::Word("mnist".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());
    }
    #[test]
    fn mixed_numeric_alpha() {
        let scan = Scanner::new("23aa < 10".to_string());
        let expected = vec![
            Token::new(TokenType::Word("23aa".to_string())),
            Token::new(TokenType::InputRedirect),
            Token::new(TokenType::Integer(10)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("123 < 10 20 < 30".to_string());
        let expected = vec![
            Token::new(TokenType::Integer(123)),
            Token::new(TokenType::InputRedirect),
            Token::new(TokenType::Integer(10)),
            Token::new(TokenType::Integer(20)),
            Token::new(TokenType::InputRedirect),
            Token::new(TokenType::Integer(30)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("123 < 10 20 < 30".to_string());
        let expected = vec![
            Token::new(TokenType::Integer(123)),
            Token::new(TokenType::InputRedirect),
            Token::new(TokenType::Integer(10)),
            Token::new(TokenType::Integer(20)),
            Token::new(TokenType::InputRedirect),
            Token::new(TokenType::Integer(30)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());
    }

    #[test]
    fn expression() {
        let scan = Scanner::new("123 < 10".to_string());
        let expected = vec![
            Token::new(TokenType::Integer(123)),
            Token::new(TokenType::InputRedirect),
            Token::new(TokenType::Integer(10)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("123 < 10  20 < 30".to_string());
        let expected = vec![
            Token::new(TokenType::Integer(123)),
            Token::new(TokenType::InputRedirect),
            Token::new(TokenType::Integer(10)),
            Token::new(TokenType::Integer(20)),
            Token::new(TokenType::InputRedirect),
            Token::new(TokenType::Integer(30)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());
    }
    #[test]
    fn words() {
        let scan = Scanner::new("abc abc abc".to_string());
        let expected = vec![
            Token::new(TokenType::Word("abc".to_string())),
            Token::new(TokenType::Word("abc".to_string())),
            Token::new(TokenType::Word("abc".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());
        let scan = Scanner::new("a_bc abc abc".to_string());
        let expected = vec![
            Token::new(TokenType::Word("a_bc".to_string())),
            Token::new(TokenType::Word("abc".to_string())),
            Token::new(TokenType::Word("abc".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("a_bc a123_bc abc".to_string());
        let expected = vec![
            Token::new(TokenType::Word("a_bc".to_string())),
            Token::new(TokenType::Word("a123_bc".to_string())),
            Token::new(TokenType::Word("abc".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("x y z a b c".to_string());
        let expected = vec![
            Token::new(TokenType::Word("x".to_string())),
            Token::new(TokenType::Word("y".to_string())),
            Token::new(TokenType::Word("z".to_string())),
            Token::new(TokenType::Word("a".to_string())),
            Token::new(TokenType::Word("b".to_string())),
            Token::new(TokenType::Word("c".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("x y z a b c".to_string());
        let expected = vec![
            Token::new(TokenType::Word("x".to_string())),
            Token::new(TokenType::Word("y".to_string())),
            Token::new(TokenType::Word("z".to_string())),
            Token::new(TokenType::Word("a".to_string())),
            Token::new(TokenType::Word("b".to_string())),
            Token::new(TokenType::Word("c".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("\"a\"".to_string());
        let expected = vec![
            Token::new(TokenType::DoubleQuotedString("a".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("\"\"".to_string());
        let expected = vec![
            Token::new(TokenType::DoubleQuotedString("".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("\"a big boy\"".to_string());
        let expected = vec![
            Token::new(TokenType::DoubleQuotedString("a big boy".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("'a big boy".to_string());
        assert_eq!(None, scan.get_tokens());
    }
    #[test]
    fn range_expression() {
        let scan = Scanner::new("{1..2}".to_string());
        let expected = vec![
            Token::new(TokenType::RangeExpressionNumeric(1, 2, None)),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());
    }

    #[test]
    fn commands_2() {
        let scan = Scanner::new("echo \"Hello, world!\"".to_string());
        let expected = vec![
            Token::new(TokenType::Word("echo".to_string())),
            Token::new(TokenType::DoubleQuotedString("Hello, world!".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("ls -l | grep \".txt\"".to_string());
        let expected = vec![
            Token::new(TokenType::Word("ls".to_string())),
            Token::new(TokenType::Word("-l".to_string())),
            Token::new(TokenType::Pipe),
            Token::new(TokenType::Word("grep".to_string())),
            Token::new(TokenType::DoubleQuotedString(".txt".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("find . -name \"*.rs\" -type f".to_string());
        let expected = vec![
            Token::new(TokenType::Word("find".to_string())),
            Token::new(TokenType::Dot),
            Token::new(TokenType::Word("-name".to_string())),
            Token::new(TokenType::DoubleQuotedString("*.rs".to_string())),
            Token::new(TokenType::Word("-type".to_string())),
            Token::new(TokenType::Word("f".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());
        //
        let scan = Scanner::new("for i in {1..5}; do echo $i; done".to_string());
        let expected = vec![
            Token::new(TokenType::Word("for".to_string())),
            Token::new(TokenType::Word("i".to_string())),
            Token::new(TokenType::Word("in".to_string())),
            Token::new(TokenType::RangeExpressionNumeric(1, 5, None)),
            Token::new(TokenType::Semicolon),
            Token::new(TokenType::Word("do".to_string())),
            Token::new(TokenType::Word("echo".to_string())),
            Token::new(TokenType::VariableExpansion("i".to_string())),
            Token::new(TokenType::Semicolon),
            Token::new(TokenType::Word("done".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());

        let scan = Scanner::new("cat file.txt | sed 's/old/new/g' > newfile.txt".to_string());
        let expected = vec![
            Token::new(TokenType::Word("cat".to_string())),
            Token::new(TokenType::Word("file.txt".to_string())),
            Token::new(TokenType::Pipe),
            Token::new(TokenType::Word("sed".to_string())),
            Token::new(TokenType::SingleQuotedString("s/old/new/g".to_string())),
            Token::new(TokenType::OutputRedirect),
            Token::new(TokenType::Word("newfile.txt".to_string())),
            Token::new(TokenType::EOF),
        ];
        assert_eq!(Some(expected), scan.get_tokens());
    }

    /*
     * awk '{print $1}' data.txt | sort | uniq -c
     * ps aux | grep "[n]ginx"
     * (cd /tmp && touch test.txt && echo "Created file")
     * echo $((2 + 3 * 4))
     * tar -czvf archive.tar.gz /path/to/directory
     * while read line; do echo "Processing: $line"; done < input.txt
     * curl -s https://api.example.com | jq '.data[]'
     * find . -type f -exec file {} \; | grep "ASCII text"
     * echo "Line 1" && echo "Line 2" || echo "Failed"
     * export VAR="value" && echo $VAR
     * cut -d',' -f1,3 data.csv | tail -n +2
     * case "$1" in start) echo "Starting";; stop) echo "Stopping";; *) echo "Unknown";; esac
     * grep -oP '(?<=name=")[^"]*' config.xml
     * [ -z "$VAR" ] && echo "VAR is empty" || echo "VAR is set"
     */
}
