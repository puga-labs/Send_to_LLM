use thiserror::Error;

#[derive(Debug, Clone)]
pub struct TextValidator {
    max_length: usize,
    max_tokens_estimate: usize,
    min_length: usize,
    allow_only_whitespace: bool,
    detect_binary_data: bool,
    trim_before_validate: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextValidationResult {
    Valid,
    TooShort { length: usize, min: usize },
    TooLong { length: usize, max: usize },
    TooManyTokens { estimated: usize, max: usize },
    OnlyWhitespace,
    ContainsBinary,
    Empty,
}

#[derive(Error, Debug)]
pub enum TextValidationError {
    #[error("Text is empty")]
    Empty,
    
    #[error("Text is too short: {length} characters, minimum: {min}")]
    TooShort { length: usize, min: usize },
    
    #[error("Text is too long: {length} characters, maximum: {max}")]
    TooLong { length: usize, max: usize },
    
    #[error("Text has too many tokens: ~{estimated}, maximum: {max}")]
    TooManyTokens { estimated: usize, max: usize },
    
    #[error("Text contains only whitespace")]
    OnlyWhitespace,
    
    #[error("Text contains binary data")]
    ContainsBinary,
}

impl TextValidator {
    pub fn new(
        max_length: usize,
        max_tokens_estimate: usize,
        min_length: usize,
    ) -> Self {
        Self {
            max_length,
            max_tokens_estimate,
            min_length,
            allow_only_whitespace: false,
            detect_binary_data: true,
            trim_before_validate: true,
        }
    }

    pub fn with_whitespace_allowed(mut self, allowed: bool) -> Self {
        self.allow_only_whitespace = allowed;
        self
    }

    pub fn with_binary_detection(mut self, detect: bool) -> Self {
        self.detect_binary_data = detect;
        self
    }

    pub fn with_trim(mut self, trim: bool) -> Self {
        self.trim_before_validate = trim;
        self
    }

    pub fn validate(&self, text: &str) -> Result<String, TextValidationError> {
        let processed = if self.trim_before_validate {
            text.trim()
        } else {
            text
        };

        // Check empty
        if processed.is_empty() {
            return Err(TextValidationError::Empty);
        }

        // Check whitespace only
        if !self.allow_only_whitespace && processed.chars().all(|c| c.is_whitespace()) {
            return Err(TextValidationError::OnlyWhitespace);
        }

        // Check length constraints (use char count, not byte count)
        let char_count = processed.chars().count();
        if char_count < self.min_length {
            return Err(TextValidationError::TooShort {
                length: char_count,
                min: self.min_length,
            });
        }

        if char_count > self.max_length {
            return Err(TextValidationError::TooLong {
                length: char_count,
                max: self.max_length,
            });
        }

        // Estimate tokens (roughly 4 chars = 1 token for most languages)
        let estimated_tokens = (char_count + 3) / 4; // Round up
        if estimated_tokens > self.max_tokens_estimate {
            return Err(TextValidationError::TooManyTokens {
                estimated: estimated_tokens,
                max: self.max_tokens_estimate,
            });
        }

        // Check for binary data
        if self.detect_binary_data {
            let has_binary = processed.chars().any(|c| {
                c.is_control() && c != '\n' && c != '\r' && c != '\t'
            });
            
            if has_binary {
                return Err(TextValidationError::ContainsBinary);
            }
        }

        Ok(processed.to_string())
    }

    pub fn split_text(&self, text: &str) -> Vec<String> {
        let processed = if self.trim_before_validate {
            text.trim()
        } else {
            text
        };

        if processed.chars().count() <= self.max_length {
            return vec![processed.to_string()];
        }

        // Smart splitting by sentences or paragraphs
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        
        // Try to split by paragraphs first
        let paragraphs: Vec<&str> = processed.split("\n\n").collect();
        
        for paragraph in paragraphs {
            let chunk_chars = current_chunk.chars().count();
            let para_chars = paragraph.chars().count();
            if chunk_chars + para_chars + 2 <= self.max_length {
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(paragraph);
            } else {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.clone());
                    current_chunk.clear();
                }
                
                // If paragraph itself is too long, split by sentences
                if paragraph.chars().count() > self.max_length {
                    let sentences = self.split_by_sentences(paragraph);
                    for sentence in sentences {
                        chunks.push(sentence);
                    }
                } else {
                    current_chunk = paragraph.to_string();
                }
            }
        }
        
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }
        
        chunks
    }

    fn split_by_sentences(&self, text: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current = String::new();
        
        // Simple sentence splitting (can be improved with proper NLP)
        let sentences = text.split_inclusive(|c| c == '.' || c == '!' || c == '?');
        
        for sentence in sentences {
            let current_chars = current.chars().count();
            let sentence_chars = sentence.chars().count();
            if current_chars + sentence_chars <= self.max_length {
                current.push_str(sentence);
            } else {
                if !current.is_empty() {
                    chunks.push(current.clone());
                    current.clear();
                }
                
                // If sentence is still too long, hard split
                if sentence.chars().count() > self.max_length {
                    chunks.extend(self.hard_split(sentence));
                } else {
                    current = sentence.to_string();
                }
            }
        }
        
        if !current.is_empty() {
            chunks.push(current);
        }
        
        chunks
    }

    fn hard_split(&self, text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        chars.chunks(self.max_length)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        let validator = TextValidator::new(1000, 250, 1);
        assert!(matches!(
            validator.validate(""),
            Err(TextValidationError::Empty)
        ));
    }

    #[test]
    fn test_whitespace_only() {
        let validator = TextValidator::new(1000, 250, 1);
        assert!(matches!(
            validator.validate("   \n\t  "),
            Err(TextValidationError::OnlyWhitespace)
        ));
    }

    #[test]
    fn test_too_short() {
        let validator = TextValidator::new(1000, 250, 10);
        assert!(matches!(
            validator.validate("hello"),
            Err(TextValidationError::TooShort { length: 5, min: 10 })
        ));
    }

    #[test]
    fn test_too_long() {
        let validator = TextValidator::new(10, 5, 1);
        let long_text = "a".repeat(20);
        assert!(matches!(
            validator.validate(&long_text),
            Err(TextValidationError::TooLong { .. })
        ));
    }

    #[test]
    fn test_binary_data() {
        let validator = TextValidator::new(1000, 250, 1);
        let text_with_null = "Hello\0World";
        assert!(matches!(
            validator.validate(text_with_null),
            Err(TextValidationError::ContainsBinary)
        ));
    }

    #[test]
    fn test_valid_text() {
        let validator = TextValidator::new(1000, 250, 1);
        let result = validator.validate("Hello, world!");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");
    }

    #[test]
    fn test_text_splitting() {
        let validator = TextValidator::new(50, 20, 1);
        let long_text = "This is a long text. It has multiple sentences. Each sentence should be split properly.";
        let chunks = validator.split_text(long_text);
        
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= 50);
        }
    }

    #[test]
    fn test_unicode_handling() {
        let validator = TextValidator::new(1000, 250, 1);
        let unicode_text = "Привет, мир! 你好世界 🌍";
        let result = validator.validate(unicode_text);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_unicode_length_counting() {
        let validator = TextValidator::new(10, 10, 1);
        
        // This string has 10 characters but many more bytes
        let unicode_text = "你好世界🌍🌏🌎❤️👨‍👩‍👧‍👦";
        let result = validator.validate(unicode_text);
        
        // Should pass because it's exactly 10 characters
        assert!(result.is_ok());
        
        // Now test with 11 characters
        let too_long = "你好世界🌍🌏🌎❤️👨‍👩‍👧‍👦!";
        let result = validator.validate(too_long);
        
        // Should fail as too long
        assert!(matches!(result, Err(TextValidationError::TooLong { .. })));
    }
    
    #[test]
    fn test_unicode_splitting() {
        let validator = TextValidator::new(5, 5, 1);
        
        // Test splitting with Unicode characters
        let text = "你好世界🌍 Hello!";
        let chunks = validator.split_text(text);
        
        // Each chunk should have at most 5 characters
        for chunk in &chunks {
            assert!(chunk.chars().count() <= 5);
        }
        
        // Ensure no characters were lost
        let combined: String = chunks.join("");
        assert_eq!(combined.chars().count(), text.chars().count());
    }
}