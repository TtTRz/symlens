use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

/// Custom tokenizer that splits camelCase, snake_case, and PascalCase identifiers.
///
/// Examples:
///   "processAudioBlock" → ["process", "audio", "block"]
///   "process_audio_block" → ["process", "audio", "block"]
///   "HTTPClient" → ["http", "client"]
///   "AudioEngine" → ["audio", "engine"]
#[derive(Clone)]
pub struct CodeTokenizer;

impl Tokenizer for CodeTokenizer {
    type TokenStream<'a> = CodeTokenStream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        let tokens = tokenize_code(text);
        CodeTokenStream {
            tokens,
            index: 0,
            token: Token::default(),
        }
    }
}

pub struct CodeTokenStream {
    tokens: Vec<(String, usize, usize)>, // (text, start_offset, end_offset)
    index: usize,
    token: Token,
}

impl TokenStream for CodeTokenStream {
    fn advance(&mut self) -> bool {
        if self.index < self.tokens.len() {
            let (ref text, start, end) = self.tokens[self.index];
            self.token.text.clear();
            self.token.text.push_str(text);
            self.token.offset_from = start;
            self.token.offset_to = end;
            self.token.position = self.index;
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn token(&self) -> &Token {
        &self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.token
    }
}

/// Split a code identifier into tokens.
fn tokenize_code(text: &str) -> Vec<(String, usize, usize)> {
    let mut tokens = Vec::new();
    let mut search_from = 0;

    // Split by whitespace first, then process each word
    for word in text.split_whitespace() {
        // Search from current position to handle duplicate words correctly
        let word_start = text[search_from..]
            .find(word)
            .map(|pos| search_from + pos)
            .unwrap_or(search_from);
        split_identifier(word, word_start, &mut tokens);
        search_from = word_start + word.len();
    }

    // Filter single-char tokens to reduce index noise
    tokens.retain(|(t, _, _)| t.len() >= 2);
    tokens
}

fn split_identifier(word: &str, base_offset: usize, tokens: &mut Vec<(String, usize, usize)>) {
    let mut chars = word.chars().peekable();
    if chars.peek().is_none() {
        return;
    }

    let mut current_start: usize = 0;
    let mut char_pos: usize = 0;
    let mut current = String::new();
    let mut prev_was_upper = false;

    // Helper: flush current token if non-empty.
    let flush = |current: &mut String,
                 start: usize,
                 end: usize,
                 tokens: &mut Vec<(String, usize, usize)>| {
        if !current.is_empty() {
            tokens.push((std::mem::take(current), start, end));
        }
    };

    while let Some(ch) = chars.next() {
        if ch == '_' || ch == '-' || ch == '.' || ch == '/' || ch == ':' {
            if !current.is_empty() {
                let start = base_offset + current_start;
                let end = base_offset + char_pos;
                flush(&mut current, start, end, tokens);
            }
            current_start = char_pos + ch.len_utf8();
            prev_was_upper = false;
        } else if ch.is_uppercase() && !current.is_empty() {
            let next_lower = chars.peek().is_some_and(|c| c.is_lowercase());

            if !prev_was_upper || next_lower {
                if !current.is_empty() {
                    let start = base_offset + current_start;
                    let end = base_offset + char_pos;
                    flush(&mut current, start, end, tokens);
                }
                current_start = char_pos;
            }
            current.push(ch.to_ascii_lowercase());
            prev_was_upper = true;
        } else if ch.is_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
            prev_was_upper = ch.is_uppercase();
        } else {
            if !current.is_empty() {
                let start = base_offset + current_start;
                let end = base_offset + char_pos;
                flush(&mut current, start, end, tokens);
            }
            current_start = char_pos + ch.len_utf8();
            prev_was_upper = false;
        }
        char_pos += ch.len_utf8();
    }

    // Flush remaining
    if !current.is_empty() {
        let start = base_offset + current_start;
        let end = base_offset + char_pos;
        flush(&mut current, start, end, tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(input: &str) -> Vec<String> {
        tokenize_code(input)
            .into_iter()
            .map(|(t, _, _)| t)
            .collect()
    }

    #[test]
    fn test_camel_case() {
        assert_eq!(tok("processAudioBlock"), vec!["process", "audio", "block"]);
    }

    #[test]
    fn test_pascal_case() {
        assert_eq!(tok("AudioEngine"), vec!["audio", "engine"]);
    }

    #[test]
    fn test_snake_case() {
        assert_eq!(
            tok("process_audio_block"),
            vec!["process", "audio", "block"]
        );
    }

    #[test]
    fn test_acronym() {
        assert_eq!(tok("HTTPClient"), vec!["http", "client"]);
    }

    #[test]
    fn test_mixed() {
        assert_eq!(tok("getHTTPResponse"), vec!["get", "http", "response"]);
    }

    #[test]
    fn test_query_words() {
        assert_eq!(tok("process audio"), vec!["process", "audio"]);
    }
}
