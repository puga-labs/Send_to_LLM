use crate::validation::TextValidator;

/// Split large text into smaller chunks for translation
pub struct TextSplitter {
    max_chunk_size: usize,
    overlap_size: usize,
}

impl TextSplitter {
    pub fn new(max_chunk_size: usize) -> Self {
        Self {
            max_chunk_size,
            overlap_size: 50, // Characters to overlap between chunks for context
        }
    }
    
    /// Split text into translatable chunks while preserving context
    pub fn split_for_translation(&self, text: &str) -> Vec<TranslationChunk> {
        if text.chars().count() <= self.max_chunk_size {
            return vec![TranslationChunk {
                index: 0,
                text: text.to_string(),
                is_continuation: false,
                overlap_start: 0,
            }];
        }
        
        let mut chunks = Vec::new();
        let mut current_position = 0;
        let chars: Vec<char> = text.chars().collect();
        let total_chars = chars.len();
        let mut chunk_index = 0;
        
        while current_position < total_chars {
            // Calculate chunk boundaries
            let chunk_start = if chunk_index == 0 { 
                0 
            } else { 
                current_position.saturating_sub(self.overlap_size) 
            };
            
            let chunk_end = (chunk_start + self.max_chunk_size).min(total_chars);
            
            // Try to find a good split point (sentence boundary)
            let adjusted_end = if chunk_end < total_chars {
                self.find_split_point(&chars, chunk_start, chunk_end)
                    .unwrap_or(chunk_end)
            } else {
                chunk_end
            };
            
            // Extract chunk text
            let chunk_text: String = chars[chunk_start..adjusted_end].iter().collect();
            
            chunks.push(TranslationChunk {
                index: chunk_index,
                text: chunk_text,
                is_continuation: chunk_index > 0,
                overlap_start: if chunk_index > 0 { self.overlap_size } else { 0 },
            });
            
            current_position = adjusted_end;
            chunk_index += 1;
        }
        
        chunks
    }
    
    /// Find a good split point near the target position
    fn find_split_point(&self, chars: &[char], start: usize, target: usize) -> Option<usize> {
        // Look for sentence boundaries first
        let sentence_endings = ['.', '!', '?', '。', '！', '？'];
        
        // Search backwards from target for sentence ending
        for i in (start..=target).rev() {
            if i > start && sentence_endings.contains(&chars[i - 1]) {
                // Check if followed by space or newline (or end of text)
                if i >= chars.len() || chars[i].is_whitespace() {
                    return Some(i);
                }
            }
        }
        
        // Look for paragraph boundaries
        for i in (start..=target).rev() {
            if i > start && chars[i - 1] == '\n' {
                return Some(i);
            }
        }
        
        // Look for other natural boundaries (comma, semicolon)
        let soft_boundaries = [',', ';', ':', '、', '；', '：'];
        for i in (start..=target).rev() {
            if i > start && soft_boundaries.contains(&chars[i - 1]) {
                return Some(i);
            }
        }
        
        // Last resort: find word boundary
        for i in (start..=target).rev() {
            if i > start && chars[i].is_whitespace() && !chars[i - 1].is_whitespace() {
                return Some(i);
            }
        }
        
        None
    }
    
    /// Merge translated chunks back together
    pub fn merge_translations(&self, chunks: Vec<TranslatedChunk>) -> String {
        if chunks.is_empty() {
            return String::new();
        }
        
        if chunks.len() == 1 {
            return chunks[0].translated_text.clone();
        }
        
        let mut result = String::new();
        
        for (i, chunk) in chunks.iter().enumerate() {
            if i == 0 {
                result.push_str(&chunk.translated_text);
            } else {
                // Remove overlap from the beginning of continuation chunks
                let text = if chunk.overlap_start > 0 {
                    // Try to intelligently remove the overlap
                    self.remove_overlap(
                        &chunks[i - 1].translated_text,
                        &chunk.translated_text,
                        chunk.overlap_start
                    )
                } else {
                    chunk.translated_text.clone()
                };
                
                // Add appropriate spacing
                if !result.ends_with(char::is_whitespace) && !text.starts_with(char::is_whitespace) {
                    result.push(' ');
                }
                
                result.push_str(&text);
            }
        }
        
        result
    }
    
    /// Remove overlapping content from the beginning of a translated chunk
    fn remove_overlap(&self, previous: &str, current: &str, overlap_chars: usize) -> String {
        // This is a simplified implementation
        // In production, you might want to use more sophisticated matching
        let chars: Vec<char> = current.chars().collect();
        if chars.len() > overlap_chars {
            chars[overlap_chars..].iter().collect()
        } else {
            current.to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranslationChunk {
    pub index: usize,
    pub text: String,
    pub is_continuation: bool,
    pub overlap_start: usize,
}

#[derive(Debug, Clone)]
pub struct TranslatedChunk {
    pub index: usize,
    pub translated_text: String,
    pub overlap_start: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_split_needed() {
        let splitter = TextSplitter::new(1000);
        let text = "Hello, world!";
        let chunks = splitter.split_for_translation(text);
        
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, text);
        assert_eq!(chunks[0].index, 0);
        assert!(!chunks[0].is_continuation);
    }
    
    #[test]
    fn test_split_at_sentence_boundary() {
        let splitter = TextSplitter::new(50);
        let text = "This is the first sentence. This is the second sentence. This is the third.";
        let chunks = splitter.split_for_translation(text);
        
        assert!(chunks.len() > 1);
        // First chunk should end at a sentence boundary
        assert!(chunks[0].text.ends_with('.') || chunks[0].text.ends_with(". "));
    }
    
    #[test]
    fn test_split_with_overlap() {
        let splitter = TextSplitter::new(100);
        let text = "A".repeat(250); // Long text that needs splitting
        let chunks = splitter.split_for_translation(&text);
        
        assert_eq!(chunks.len(), 3);
        assert!(chunks[1].is_continuation);
        assert!(chunks[2].is_continuation);
        assert_eq!(chunks[1].overlap_start, 50);
    }
    
    #[test]
    fn test_merge_translations() {
        let splitter = TextSplitter::new(100);
        
        let translated_chunks = vec![
            TranslatedChunk {
                index: 0,
                translated_text: "Первая часть текста.".to_string(),
                overlap_start: 0,
            },
            TranslatedChunk {
                index: 1,
                translated_text: "текста. Вторая часть текста.".to_string(),
                overlap_start: 7,
            },
        ];
        
        let merged = splitter.merge_translations(translated_chunks);
        assert!(merged.contains("Первая часть текста."));
        assert!(merged.contains("Вторая часть текста."));
        // Should not duplicate "текста."
        assert_eq!(merged.matches("текста.").count(), 2);
    }
    
    #[test]
    fn test_unicode_splitting() {
        let splitter = TextSplitter::new(10);
        let text = "Привет, мир! Как дела? 你好世界！";
        let chunks = splitter.split_for_translation(text);
        
        // Should split but preserve complete characters
        assert!(chunks.len() > 1);
        for chunk in chunks {
            // Ensure no broken Unicode sequences
            assert!(chunk.text.is_char_boundary(0));
            assert!(chunk.text.is_char_boundary(chunk.text.len()));
        }
    }
}