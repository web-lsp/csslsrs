use crate::service::LanguageService;
use lsp_types::{FoldingRange, FoldingRangeKind, TextDocumentItem};

/// Computes the folding ranges for the given CSS source code. It supports CSS blocks enclosed in
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
    /// Computes the folding ranges for the given CSS source code.
    ///
    /// # Arguments
    ///
    /// * `document` - The original CSS source code as a `TextDocumentItem`.
    ///
    /// # Returns
    ///
    /// * A vector of `FoldingRange` indicating the foldable regions in the CSS code.
    pub fn get_folding_ranges(mut self, document: TextDocumentItem) -> Vec<FoldingRange> {
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use lsp_types::Uri;

    use super::*;

    #[test]
    fn test_compute_folding_ranges_empty() {
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test.css").unwrap(),
            "css".to_string(),
            1,
            "".to_string(),
        );

        let folding_ranges = compute_folding_ranges(&document);

        assert!(
            folding_ranges.is_empty(),
            "No folding ranges should be returned for empty input"
        );
    }

    #[test]
    fn test_compute_folding_ranges_single_rule() {
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test.css").unwrap(),
            "css".to_string(),
            1,
            "body {\n    margin: 0;\n    padding: 0;\n}\n".to_string(),
        );

        let folding_ranges = compute_folding_ranges(&document);

        assert_eq!(folding_ranges.len(), 1, "Expected one folding range");
        let range = &folding_ranges[0];
        assert_eq!(range.start_line, 0, "Folding should start at line 0");
        assert_eq!(range.end_line, 3, "Folding should end at line 3");
    }

    #[test]
    fn test_compute_folding_ranges_multiple_rules() {
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test.css").unwrap(),
            "css".to_string(),
            1,
            "body {\n    margin: 0;\n}\n\nh1 {\n    color: red;\n}\n".to_string(),
        );
        let mut folding_ranges = compute_folding_ranges(&document);

        assert_eq!(folding_ranges.len(), 2, "Expected two folding ranges");

        folding_ranges.sort_by_key(|fr| fr.start_line);

        let range1 = &folding_ranges[0];
        assert_eq!(range1.start_line, 0, "First folding should start at line 0");
        assert_eq!(range1.end_line, 2, "First folding should end at line 2");

        let range2 = &folding_ranges[1];
        assert_eq!(
            range2.start_line, 4,
            "Second folding should start at line 4"
        );
        assert_eq!(range2.end_line, 6, "Second folding should end at line 6");
    }

    #[test]
    fn test_compute_folding_ranges_nested_rules() {
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test.css").unwrap(),
            "css".to_string(),
            1,
            "@media screen {\n    body {\n        margin: 0;\n    }\n}\n".to_string(),
        );
        let mut folding_ranges = compute_folding_ranges(&document);

        assert_eq!(folding_ranges.len(), 2, "Expected two folding ranges");

        // Sort folding ranges by start_line
        folding_ranges.sort_by_key(|fr| fr.start_line);

        let outer_range = &folding_ranges[0];
        assert_eq!(
            outer_range.start_line, 0,
            "Outer folding should start at line 0"
        );
        assert_eq!(
            outer_range.end_line, 4,
            "Outer folding should end at line 4"
        );

        let inner_range = &folding_ranges[1];
        assert_eq!(
            inner_range.start_line, 1,
            "Inner folding should start at line 1"
        );
        assert_eq!(
            inner_range.end_line, 3,
            "Inner folding should end at line 3"
        );
    }

    #[test]
    fn test_compute_folding_ranges_single_line_rule() {
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test.css").unwrap(),
            "css".to_string(),
            1,
            "h1 { color: blue; }\n".to_string(),
        );
        let folding_ranges = compute_folding_ranges(&document);

        // Since the rule is on a single line, there should be no folding range
        assert!(
            folding_ranges.is_empty(),
            "No folding ranges expected for single-line rules"
        );
    }

    #[test]
    fn test_compute_folding_ranges_unmatched_braces() {
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test.css").unwrap(),
            "css".to_string(),
            1,
            "body {\n    margin: 0;\n    padding: 0;\n\n".to_string(),
        );
        let folding_ranges = compute_folding_ranges(&document);

        // The opening brace does not have a matching closing brace
        // So the folding range should not be added
        assert!(
            folding_ranges.is_empty(),
            "No folding ranges expected when braces are unmatched"
        );
    }

    #[test]
    fn test_compute_folding_ranges_with_comments() {
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test.css").unwrap(),
            "css".to_string(),
            1,
            "/* Comment block\nspanning multiple lines\n*/\nbody {\n    margin: 0;\n}\n"
                .to_string(),
        );
        let folding_ranges = compute_folding_ranges(&document);

        assert_eq!(
            folding_ranges.len(),
            2,
            "Expected two folding ranges: one for the comment and one for the body block"
        );

        // Check the comment folding range
        let comment_range = folding_ranges
            .iter()
            .find(|r| r.kind == Some(FoldingRangeKind::Comment))
            .expect("Comment folding range not found");
        assert_eq!(
            comment_range.start_line, 0,
            "Comment should start at line 0"
        );
        assert_eq!(comment_range.end_line, 2, "Comment should end at line 2");

        // Check the body block folding range
        let body_range = folding_ranges
            .iter()
            .find(|r| r.kind == None)
            .expect("Body block folding range not found");
        assert_eq!(
            body_range.start_line, 3,
            "Body block should start at line 3"
        );
        assert_eq!(body_range.end_line, 5, "Body block should end at line 5");
    }

    #[test]
    fn test_compute_folding_ranges_complex() {
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test.css").unwrap(),
            "css".to_string(),
            1,
            "@media screen {\n    @supports (display: grid) {\n        .container {\n            display: grid;\n        }\n    }\n}\n".to_string(),
        );
        let mut folding_ranges = compute_folding_ranges(&document);

        assert_eq!(folding_ranges.len(), 3, "Expected three folding ranges");

        // Sort folding ranges by start_line
        folding_ranges.sort_by_key(|fr| fr.start_line);

        let range1 = &folding_ranges[0];
        assert_eq!(range1.start_line, 0, "First folding should start at line 0");
        assert_eq!(range1.end_line, 6, "First folding should end at line 6");

        let range2 = &folding_ranges[1];
        assert_eq!(
            range2.start_line, 1,
            "Second folding should start at line 1"
        );
        assert_eq!(range2.end_line, 5, "Second folding should end at line 5");

        let range3 = &folding_ranges[2];
        assert_eq!(range3.start_line, 2, "Third folding should start at line 2");
        assert_eq!(range3.end_line, 4, "Third folding should end at line 4");
    }

    #[test]
    fn test_compute_folding_ranges_with_region_comments() {
        use lsp_types::FoldingRangeKind;
        use std::str::FromStr;

        // Create a CSS document with #region and #endregion markers in comments
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test_with_region.css").unwrap(),
            "css".to_string(),
            1,
            "/* #region Header */\n.header {\n    background: blue;\n}\n/* #endregion */\n\n/* #region Footer */\n.footer {\n    background: green;\n}\n/* #endregion */\n".to_string(),
        );

        // Compute folding ranges
        let folding_ranges = compute_folding_ranges(&document);

        // Expecting four folding ranges: two for regions and two for CSS blocks
        assert_eq!(
            folding_ranges.len(),
            4,
            "Expected four folding ranges: two for regions and two for CSS blocks"
        );

        // Sort folding ranges by start_line to ensure consistent order
        let mut sorted_ranges = folding_ranges.clone();
        sorted_ranges.sort_by_key(|fr| fr.start_line);

        // Verify the first folding range corresponds to the Header region
        let header_region = &sorted_ranges[0];
        assert_eq!(
            header_region.start_line, 0,
            "Header region should start at line 0"
        );
        assert_eq!(
            header_region.end_line, 4,
            "Header region should end at line 4"
        );
        assert_eq!(
            header_region.kind,
            Some(FoldingRangeKind::Region),
            "Header region should have kind 'Region'"
        );

        // Verify the second folding range corresponds to the Header block
        let header_block = &sorted_ranges[1];
        assert_eq!(
            header_block.start_line, 1,
            "Header block should start at line 1"
        );
        assert_eq!(
            header_block.end_line, 3,
            "Header block should end at line 3"
        );
        assert_eq!(
            header_block.kind, None,
            "Header block should have no specific kind"
        );

        // Verify the third folding range corresponds to the Footer region
        let footer_region = &sorted_ranges[2];
        assert_eq!(
            footer_region.start_line, 6,
            "Footer region should start at line 6"
        );
        assert_eq!(
            footer_region.end_line, 10,
            "Footer region should end at line 10"
        );
        assert_eq!(
            footer_region.kind,
            Some(FoldingRangeKind::Region),
            "Footer region should have kind 'Region'"
        );

        // Verify the fourth folding range corresponds to the Footer block
        let footer_block = &sorted_ranges[3];
        assert_eq!(
            footer_block.start_line, 7,
            "Footer block should start at line 7"
        );
        assert_eq!(
            footer_block.end_line, 9,
            "Footer block should end at line 9"
        );
        assert_eq!(
            footer_block.kind, None,
            "Footer block should have no specific kind"
        );
    }

    #[test]
    fn test_compute_folding_ranges_ignores_non_foldable_tokens() {
        // Create a CSS document with non-foldable tokens (properties without braces)
        let document = TextDocumentItem::new(
        Uri::from_str("file:///test_non_foldable.css").unwrap(),
        "css".to_string(),
        1,
        "body {\n    margin: 0;\n    padding: 0;\n}\n\n/* Single-line comment */\n\nh1 { color: blue; }\n\np {\n    font-size: 16px;\n}\n".to_string(),
    );

        // Compute folding ranges
        let folding_ranges = compute_folding_ranges(&document);

        // Expecting two folding ranges: one for the body block and one for the p block
        // The single-line comment and the h1 rule should not generate folding ranges
        assert_eq!(
            folding_ranges.len(),
            2,
            "Expected two folding ranges for foldable blocks only"
        );

        // Sort folding ranges by start_line to ensure consistent order
        let mut sorted_ranges = folding_ranges.clone();
        sorted_ranges.sort_by_key(|fr| fr.start_line);

        // Verify the first folding range corresponds to the body block
        let body_block = &sorted_ranges[0];
        assert_eq!(
            body_block.start_line, 0,
            "Body block should start at line 0"
        );
        assert_eq!(body_block.end_line, 3, "Body block should end at line 3");
        assert_eq!(
            body_block.kind, None,
            "Body block should have no specific kind"
        );

        // Verify the second folding range corresponds to the p block
        let p_block = &sorted_ranges[1];
        assert_eq!(p_block.start_line, 6, "p block should start at line 6");
        assert_eq!(p_block.end_line, 8, "p block should end at line 8");
        assert_eq!(p_block.kind, None, "p block should have no specific kind");
    }
    #[test]
    fn test_compute_folding_ranges_closing_brace_same_line() {
        // Create a CSS document where closing brace is on the same line as code
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test_closing_brace_same_line.css").unwrap(),
            "css".to_string(),
            1,
            "body {\n    margin: 0;\n    padding: 0; }\n".to_string(),
        );

        // Compute folding ranges
        let folding_ranges = compute_folding_ranges(&document);

        // Expecting one folding range for the body block
        assert_eq!(
            folding_ranges.len(),
            1,
            "Expected one folding range for the body block"
        );

        let range = &folding_ranges[0];
        assert_eq!(range.start_line, 0, "Body block should start at line 0");
        assert_eq!(
            range.end_line, 2,
            "Body block should end at line 2 (adjusted)"
        );
    }
    #[test]
    fn test_compute_folding_ranges_sorted_order() {
        // Create a CSS document with unordered folding ranges
        let document = TextDocumentItem::new(
        Uri::from_str("file:///test_sorted_order.css").unwrap(),
        "css".to_string(),
        1,
        "@media screen and (max-width: 600px) {\n    .container {\n        display: flex;\n    }\n}\n\n.body {\n    background: white;\n}\n".to_string(),
    );

        // Compute folding ranges
        let folding_ranges = compute_folding_ranges(&document);

        // Expecting two folding ranges: one for @media and one for .container
        assert_eq!(folding_ranges.len(), 2, "Expected two folding ranges");

        // Verify that the ranges are sorted by start_line then end_line
        assert!(
            folding_ranges[0].start_line < folding_ranges[1].start_line,
            "Ranges should be sorted by start_line"
        );
    }
    #[test]
    fn test_compute_folding_ranges_no_overlapping() {
        // Create a CSS document with potential overlapping folding ranges
        let document = TextDocumentItem::new(
        Uri::from_str("file:///test_no_overlapping.css").unwrap(),
        "css".to_string(),
        1,
        "/* #region Outer */\n.container {\n    /* #region Inner */\n    display: grid;\n    /* #endregion */\n}\n/* #endregion */\n".to_string(),
    );

        // Compute folding ranges
        let folding_ranges = compute_folding_ranges(&document);

        // Expecting two folding ranges: Outer and Inner regions
        assert_eq!(
            folding_ranges.len(),
            2,
            "Expected two non-overlapping folding ranges"
        );

        // Sort folding ranges by start_line
        let mut sorted_ranges = folding_ranges.clone();
        sorted_ranges.sort_by_key(|fr| fr.start_line);

        // Verify the Outer region
        let outer_region = &sorted_ranges[0];
        assert_eq!(
            outer_region.start_line, 0,
            "Outer region should start at line 0"
        );
        assert_eq!(
            outer_region.end_line, 6,
            "Outer region should end at line 6"
        );
        assert_eq!(
            outer_region.kind,
            Some(FoldingRangeKind::Region),
            "Outer region should have kind 'region'"
        );

        // Verify the Inner region
        let inner_region = &sorted_ranges[1];
        assert_eq!(
            inner_region.start_line, 2,
            "Inner region should start at line 2"
        );
        assert_eq!(
            inner_region.end_line, 4,
            "Inner region should end at line 4"
        );
        assert_eq!(
            inner_region.kind,
            Some(FoldingRangeKind::Region),
            "Inner region should have kind 'region'"
        );
    }

    #[test]
    fn test_compute_folding_ranges_handles_eof_correctly() {
        // A CSS document ending abruptly with an unclosed comment
        let document = TextDocumentItem::new(
            Uri::from_str("file:///test_eof.css").unwrap(),
            "css".to_string(),
            1,
            "body {\n    margin: 0;\n    padding: 0;\n/* Unclosed comment\n continuation\n"
                .to_string(),
        );
        let folding_ranges = compute_folding_ranges(&document);

        assert_eq!(
            folding_ranges.len(),
            2,
            "Expected two folding ranges: one for the body block and one for the unclosed comment"
        );

        // Check the body block folding range
        let body_range = folding_ranges
            .iter()
            .find(|r| r.kind == None)
            .expect("Body block folding range not found");
        assert_eq!(
            body_range.start_line, 0,
            "Body block should start at line 0"
        );
        assert_eq!(body_range.end_line, 4, "Body block should end at line 4");

        // Find the comment folding range
        let comment_range = folding_ranges
            .iter()
            .find(|r| r.kind == Some(FoldingRangeKind::Comment))
            .expect("Comment folding range not found");
        assert_eq!(
            comment_range.start_line, 3,
            "Comment should start at line 3"
        );
        assert_eq!(comment_range.end_line, 4, "Comment should end at line 4");
    }

    #[test]
    fn test_compute_folding_ranges_mixed_delimiters() {
        // Create a CSS document with mixed delimiters: comments inside braces and braces inside comments
        let document = TextDocumentItem::new(
        Uri::from_str("file:///test_mixed_delimiters.css").unwrap(),
        "css".to_string(),
        1,
        "/* #region Styles */\nbody {\n    /* Comment inside body */\n    margin: 0;\n}\n/* #endregion */\n\n/* #region Utilities */\n.utility {\n    padding: 10px;\n}\n/* #endregion */\n".to_string(),
    );

        // Compute folding ranges
        let folding_ranges = compute_folding_ranges(&document);

        // Expecting four folding ranges:
        // 1. Styles region
        // 2. body block
        // 3. Utilities region
        // 4. utility block
        assert_eq!(
            folding_ranges.len(),
            4,
            "Expected four folding ranges for mixed delimiters"
        );

        // Sort folding ranges by start_line
        let mut sorted_ranges = folding_ranges.clone();
        sorted_ranges.sort_by_key(|fr| fr.start_line);

        // Verify Styles region
        let styles_region = &sorted_ranges[0];
        assert_eq!(
            styles_region.start_line, 0,
            "Styles region should start at line 0"
        );
        assert_eq!(
            styles_region.end_line, 4,
            "Styles region should end at line 4"
        );
        assert_eq!(
            styles_region.kind,
            Some(FoldingRangeKind::Region),
            "Styles region should have kind 'region'"
        );

        // Verify body block
        let body_block = &sorted_ranges[1];
        assert_eq!(
            body_block.start_line, 1,
            "Body block should start at line 1"
        );
        assert_eq!(body_block.end_line, 3, "Body block should end at line 3");
        assert_eq!(
            body_block.kind, None,
            "Body block should have no specific kind"
        );

        // Verify Utilities region
        let utilities_region = &sorted_ranges[2];
        assert_eq!(
            utilities_region.start_line, 6,
            "Utilities region should start at line 6"
        );
        assert_eq!(
            utilities_region.end_line, 9,
            "Utilities region should end at line 9"
        );
        assert_eq!(
            utilities_region.kind,
            Some(FoldingRangeKind::Region),
            "Utilities region should have kind 'region'"
        );

        // Verify utility block
        let utility_block = &sorted_ranges[3];
        assert_eq!(
            utility_block.start_line, 7,
            "Utility block should start at line 7"
        );
        assert_eq!(
            utility_block.end_line, 8,
            "Utility block should end at line 8"
        );
        assert_eq!(
            utility_block.kind, None,
            "Utility block should have no specific kind"
        );
    }
    #[test]
    fn test_compute_folding_ranges_single_line_comments_without_region() {
        // Create a CSS document with single-line comments without region markers
        let document = TextDocumentItem::new(
        Uri::from_str("file:///test_single_line_comments.css").unwrap(),
        "css".to_string(),
        1,
        "/* This is a single-line comment */\nbody {\n    margin: 0;\n}\n/* Another single-line comment */\n".to_string(),
    );

        // Compute folding ranges
        let folding_ranges = compute_folding_ranges(&document);

        // Expecting one folding range for the body block only
        assert_eq!(
            folding_ranges.len(),
            1,
            "Expected one folding range for the body block only"
        );

        let range = &folding_ranges[0];
        assert_eq!(range.start_line, 1, "Body block should start at line 1");
        assert_eq!(range.end_line, 3, "Body block should end at line 3");
        assert_eq!(range.kind, None, "Body block should have no specific kind");
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use lsp_types::{FoldingRangeKind, Uri};
        use std::str::FromStr;

        #[test]
        fn test_compute_folding_ranges_nested_regions() {
            // Create a CSS document with nested #region markers
            let document = TextDocumentItem::new(
                Uri::from_str("file:///test_nested_regions.css").unwrap(),
                "css".to_string(),
                1,
                "/* #region Outer Region */\nbody {\n    /* #region Inner Region */\n    margin: 0;\n    /* #endregion */\n}\n/* #endregion */\n".to_string(),
            );

            // Compute folding ranges
            let folding_ranges = compute_folding_ranges(&document);

            // Expecting three folding ranges: two for regions and one for CSS block
            assert_eq!(
                folding_ranges.len(),
                3,
                "Expected three folding ranges: two for regions and one for CSS block"
            );

            // Sort folding ranges by start_line to ensure consistent order
            let mut sorted_ranges = folding_ranges.clone();
            sorted_ranges.sort_by_key(|fr| fr.start_line);

            // Verify Outer region
            let outer_region = &sorted_ranges[0];
            assert_eq!(
                outer_region.start_line, 0,
                "Outer region should start at line 0"
            );
            assert_eq!(
                outer_region.end_line, 6,
                "Outer region should end at line 6"
            );
            assert_eq!(
                outer_region.kind,
                Some(FoldingRangeKind::Region),
                "Outer region should have kind 'Region'"
            );

            // Verify CSS block
            let css_block = &sorted_ranges[1];
            assert_eq!(css_block.start_line, 1, "CSS block should start at line 1");
            assert_eq!(css_block.end_line, 5, "CSS block should end at line 5");
            assert_eq!(
                css_block.kind, None,
                "CSS block should have no specific kind"
            );

            // Verify Inner region
            let inner_region = &sorted_ranges[2];
            assert_eq!(
                inner_region.start_line, 2,
                "Inner region should start at line 2"
            );
            assert_eq!(
                inner_region.end_line, 4,
                "Inner region should end at line 4"
            );
            assert_eq!(
                inner_region.kind,
                Some(FoldingRangeKind::Region),
                "Inner region should have kind 'Region'"
            );
        }
    }

    #[test]
    fn test_compute_folding_ranges_comments_with_mixed_content() {
        // Create a CSS document with comments that include both region markers and regular content
        let document = TextDocumentItem::new(
        Uri::from_str("file:///test_mixed_comments.css").unwrap(),
        "css".to_string(),
        1,
        "/* #region Header */\n/* This is the header section */\n.header {\n    background: blue;\n}\n/* #endregion */\n\n/* Regular comment without region */\n.footer {\n    background: green;\n}\n".to_string(),
    );

        // Compute folding ranges
        let folding_ranges = compute_folding_ranges(&document);

        // Expecting two folding ranges: Header region and footer block
        assert_eq!(
            folding_ranges.len(),
            2,
            "Expected two folding ranges: one for Header region and one for footer block"
        );

        // Sort folding ranges by start_line
        let mut sorted_ranges = folding_ranges.clone();
        sorted_ranges.sort_by_key(|fr| fr.start_line);

        // Verify Header region
        let header_region = &sorted_ranges[0];
        assert_eq!(
            header_region.start_line, 0,
            "Header region should start at line 0"
        );
        assert_eq!(
            header_region.end_line, 4,
            "Header region should end at line 4"
        );
        assert_eq!(
            header_region.kind,
            Some(FoldingRangeKind::Region),
            "Header region should have kind 'region'"
        );

        // Verify footer block
        let footer_block = &sorted_ranges[1];
        assert_eq!(
            footer_block.start_line, 6,
            "Footer block should start at line 6"
        );
        assert_eq!(
            footer_block.end_line, 8,
            "Footer block should end at line 8"
        );
        assert_eq!(
            footer_block.kind, None,
            "Footer block should have no specific kind"
        );
    }
}
