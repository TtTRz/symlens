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

    // Split by whitespace first, then process each word
    for word in text.split_whitespace() {
        let word_start = text.find(word).unwrap_or(0);
        split_identifier(word, word_start, &mut tokens);
    }

    tokens
}

fn split_identifier(word: &str, base_offset: usize, tokens: &mut Vec<(String, usize, usize)>) {
    let chars: Vec<char> = word.chars().collect();
    if chars.is_empty() {
        return;
    }

    let mut current_start = 0;
    let mut current = String::new();

    for (i, &ch) in chars.iter().enumerate() {
        if ch == '_' || ch == '-' || ch == '.' || ch == '/' || ch == ':' {
            // Separator — flush current token
            if !current.is_empty() {
                let start = base_offset + current_start;
                let end = base_offset + i;
                tokens.push((current.to_lowercase(), start, end));
                current.clear();
            }
            current_start = i + 1;
        } else if ch.is_uppercase() && !current.is_empty() {
            // camelCase boundary
            // Check if this is the start of a new word or a multi-uppercase sequence
            let prev_upper = i > 0 && chars[i - 1].is_uppercase();
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();

            if !prev_upper || (prev_upper && next_lower) {
                // Flush: "process" before "Audio", or "HTT" before "P" in "HTTPClient"→"http","client"
                if !current.is_empty() {
                    let start = base_offset + current_start;
                    let end = base_offset + i;
                    tokens.push((current.to_lowercase(), start, end));
                    current.clear();
                }
                current_start = i;
            }
            current.push(ch);
        } else if ch.is_alphanumeric() {
            current.push(ch);
        } else {
            // Other characters — flush
            if !current.is_empty() {
                let start = base_offset + current_start;
                let end = base_offset + i;
                tokens.push((current.to_lowercase(), start, end));
                current.clear();
            }
            current_start = i + 1;
        }
    }

    // Flush remaining
    if !current.is_empty() {
        let start = base_offset + current_start;
        let end = base_offset + chars.len();
        tokens.push((current.to_lowercase(), start, end));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(input: &str) -> Vec<String> {
        tokenize_code(input).into_iter().map(|(t, _, _)| t).collect()
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
        assert_eq!(tok("process_audio_block"), vec!["process", "audio", "block"]);
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
