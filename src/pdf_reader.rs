use anyhow::Result;
use std::path::Path;

pub struct PdfReader;

impl PdfReader {
    pub fn extract_text(path: &Path) -> Result<String> {
        tracing::info!("Extracting text from PDF: {}", path.display());
        pdf_extract::extract_text(path)
            .map_err(|e| anyhow::anyhow!("PDF extraction failed: {}", e))
    }

    pub fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
        if text.is_empty() {
            return vec![];
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for para in text.split("\n\n") {
            if current_chunk.len() + para.len() + 2 > max_chars {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.clone());
                    current_chunk.clear();
                }
            }
            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(para);
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    pub fn estimate_tokens(text: &str) -> usize {
        text.len() / 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_respects_max_chars() {
        let text = "Hello world.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = PdfReader::chunk_text(text, 30);
        for chunk in &chunks {
            assert!(chunk.len() <= 30, "chunk too large: {}", chunk.len());
        }
    }

    #[test]
    fn test_estimate_tokens() {
        let text = "This is a test.";
        let tokens = PdfReader::estimate_tokens(text);
        assert!(tokens > 0);
    }
}
