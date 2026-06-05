use regex::Regex;
use std::sync::LazyLock;

static ABBREVIATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:Dr|Mr|Mrs|Ms|Prof|Sr|Jr|Inc|Ltd|Corp|vs|etc|approx|dept|univ|assn|No|Vol|Fig|Eq|al|e\.g|i\.e|a\.m|p\.m)\.\s").unwrap()
});

static SENTENCE_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?]+\s+").unwrap());

pub fn split_sentences(text: &str) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let protected = ABBREVIATION_RE.replace_all(text, |caps: &regex::Captures| {
        caps[0].replace('.', "\x00")
    });

    let mut sentences = Vec::new();
    let mut last = 0;

    for mat in SENTENCE_END_RE.find_iter(&protected) {
        let boundary = mat.start() + 1;
        let end_of_punct = mat.end() - mat.as_str().trim_start().len();
        let split_at = if end_of_punct > mat.start() {
            mat.start() + end_of_punct
        } else {
            boundary
        };

        let sentence = protected[last..split_at].replace('\x00', ".");
        if !sentence.trim().is_empty() {
            sentences.push(sentence.trim().to_string());
        }
        last = split_at;
    }

    if last < protected.len() {
        let remainder = protected[last..].replace('\x00', ".");
        if !remainder.trim().is_empty() {
            sentences.push(remainder.trim().to_string());
        }
    }

    let mut result = Vec::new();
    for s in sentences {
        for paragraph in s.split('\n') {
            let trimmed = paragraph.trim();
            if !trimmed.is_empty() {
                result.push(trimmed.to_string());
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_sentences() {
        let result = split_sentences("Hello world. How are you?");
        assert_eq!(result, vec!["Hello world.", "How are you?"]);
    }

    #[test]
    fn exclamation_and_question() {
        let result = split_sentences("Wow! Really? Yes.");
        assert_eq!(result, vec!["Wow!", "Really?", "Yes."]);
    }

    #[test]
    fn abbreviation_protection() {
        let result = split_sentences("Dr. Smith went home. He was tired.");
        assert_eq!(result.len(), 2);
        assert!(result[0].contains("Dr. Smith"));
    }

    #[test]
    fn newline_splitting() {
        let result = split_sentences("First line\nSecond line");
        assert_eq!(result, vec!["First line", "Second line"]);
    }

    #[test]
    fn empty_input() {
        let result = split_sentences("");
        assert!(result.is_empty());
    }

    #[test]
    fn single_sentence() {
        let result = split_sentences("Just one sentence");
        assert_eq!(result, vec!["Just one sentence"]);
    }

    #[test]
    fn multiple_abbreviations() {
        let result = split_sentences("Mr. and Mrs. Smith went to Dr. Jones. They were happy.");
        assert!(result.len() >= 2);
    }
}
