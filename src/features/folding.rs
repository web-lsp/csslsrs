use crate::service::LanguageService;
use lsp_types::{FoldingRange, FoldingRangeKind, TextDocumentItem};

/// Compute the folding ranges for the given CSS source code. It supports CSS blocks enclosed in
/// braces, multi-line comments, and regions marked with `#region` and `#endregion` comments.
///
/// # Arguments
/// `document` - The original CSS source code as a `TextDocumentItem`.
///
/// # Returns
/// A vector of `FoldingRange` indicating the foldable regions in the CSS code.
fn compute_folding_ranges(document: &TextDocumentItem) -> Vec<FoldingRange> {
    let mut folding_ranges = Vec::new();
    let mut brace_stack = Vec::new();
    let mut comment_stack = Vec::new();
    let mut region_stack = Vec::new();

    let source = &document.text;

    // Precompute line start offsets
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(source.match_indices('\n').map(|(idx, _)| idx + 1))
        .collect();

    let mut chars = source.char_indices().peekable();
    while let Some((offset, c)) = chars.next() {
        match c {
            '{' => {
                // Determine line number based on offset
                let line_number = line_starts
                    .partition_point(|&line_start| line_start <= offset)
                    .saturating_sub(1);
                brace_stack.push(line_number);
            }
            '}' => {
                // Pop the last start line number
                if let Some(start_line) = brace_stack.pop() {
                    let end_line = line_starts
                        .partition_point(|&line_start| line_start <= offset)
                        .saturating_sub(1);
                    if start_line != end_line {
                        folding_ranges.push(FoldingRange {
                            start_line: start_line as u32,
                            start_character: None,
                            end_line: end_line as u32,
                            end_character: None,
                            kind: None, // CSS blocks have no specific kind
                            collapsed_text: None,
                        });
                    }
                }
            }
            '/' => {
                // Check for start of multi-line comment
                if let Some(&(_, next_char)) = chars.peek() {
                    if next_char == '*' {
                        // Consume the '*' character
                        chars.next();
                        let line_number = line_starts
                            .partition_point(|&line_start| line_start <= offset)
                            .saturating_sub(1);
                        comment_stack.push(line_number);
                    }
                }
            }
            '*' => {
                // Check for end of multi-line comment
                if let Some(&(_, next_char)) = chars.peek() {
                    if next_char == '/' {
                        // Consume the '/' character
                        chars.next();
                        if let Some(start_line) = comment_stack.pop() {
                            let end_line = line_starts
                                .partition_point(|&line_start| line_start <= offset)
                                .saturating_sub(1);

                            // Determine the end offset safely
                            let end_offset = if end_line + 1 < line_starts.len() {
                                line_starts[end_line + 1]
                            } else {
                                source.len()
                            };

                            // Extract the comment content using the correct byte offsets
                            let comment_content = &source[line_starts[start_line]..end_offset];

                            if comment_content.contains("#region") {
                                // Handle #region
                                region_stack.push(start_line as u32);
                            } else if comment_content.contains("#endregion") {
                                // Handle #endregion
                                if let Some(region_start) = region_stack.pop() {
                                    folding_ranges.push(FoldingRange {
                                        start_line: region_start,
                                        start_character: None,
                                        end_line: end_line as u32,
                                        end_character: None,
                                        kind: Some(FoldingRangeKind::Region),
                                        collapsed_text: None,
                                    });
                                }
                            } else {
                                // Regular multi-line comment
                                if start_line != end_line {
                                    folding_ranges.push(FoldingRange {
                                        start_line: start_line as u32,
                                        start_character: None,
                                        end_line: end_line as u32,
                                        end_character: None,
                                        kind: Some(FoldingRangeKind::Comment),
                                        collapsed_text: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Determine the last line with content
    let mut total_lines = line_starts.len() as u32 - 1;
    if source.ends_with('\n') && total_lines > 0 {
        total_lines -= 1;
    }

    // Handle any unclosed blocks
    while let Some(start_line) = brace_stack.pop() {
        if start_line < total_lines as usize {
            folding_ranges.push(FoldingRange {
                start_line: start_line as u32,
                start_character: None,
                end_line: total_lines,
                end_character: None,
                kind: None, // CSS blocks have no specific kind
                collapsed_text: None,
            });
        }
    }

    // Handle any unclosed comments
    while let Some(start_line) = comment_stack.pop() {
        if start_line < total_lines as usize {
            folding_ranges.push(FoldingRange {
                start_line: start_line as u32,
                start_character: None,
                end_line: total_lines,
                end_character: None,
                kind: Some(FoldingRangeKind::Comment),
                collapsed_text: None,
            });
        }
    }

    // Handle any unclosed regions
    while let Some(region_start) = region_stack.pop() {
        if region_start < total_lines {
            folding_ranges.push(FoldingRange {
                start_line: region_start,
                start_character: None,
                end_line: total_lines,
                end_character: None,
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: None,
            });
        }
    }

    folding_ranges
}

impl LanguageService {
    /// Get the folding ranges for the given CSS source code. It supports CSS blocks enclosed in
    /// braces, multi-line comments, and regions marked with `#region` and `#endregion` comments.
    ///
    /// # Arguments
    /// `document` - The original CSS source code as a `TextDocumentItem`.
    ///
    /// # Returns
    /// A vector of `FoldingRange` indicating the foldable regions in the CSS code.
    pub fn get_folding_ranges(&mut self, document: TextDocumentItem) -> Vec<FoldingRange> {
        let store_document = self.store.get_or_update_document(document);
        compute_folding_ranges(&store_document.document)
    }
}

#[cfg(feature = "wasm")]
mod wasm_bindings {
    use super::compute_folding_ranges;
    use serde_wasm_bindgen;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(typescript_custom_section)]
    const TS_APPEND_CONTENT: &'static str = r#"
export async function get_folding_ranges(source: import("vscode-languageserver-textdocument").TextDocument): Promise<import("vscode-languageserver-types").FoldingRange[]>;
"#;

    #[wasm_bindgen(skip_typescript)]
    pub fn get_folding_ranges(document: JsValue) -> JsValue {
        let parsed_text_document = crate::wasm_text_document::create_text_document(document);
        let folding_ranges = compute_folding_ranges(&parsed_text_document);

        serde_wasm_bindgen::to_value(&folding_ranges).unwrap()
    }
}
