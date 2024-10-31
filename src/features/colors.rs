use biome_css_parser::CssParse;
use biome_css_syntax::{CssLanguage, CssSyntaxKind};
use biome_rowan::{AstNode, SyntaxNode};
use csscolorparser::{parse as parse_color, NAMED_COLORS};
use lsp_types::{Color, ColorInformation, ColorPresentation, Range, TextDocumentItem, TextEdit};

use crate::{
    converters::{line_index::LineIndex, to_proto::range, PositionEncoding},
    service::LanguageService,
};

fn convert_parsed_color(color: csscolorparser::Color) -> Color {
    Color {
        red: color.r,
        green: color.g,
        blue: color.b,
        alpha: color.a,
    }
}

fn extract_colors_information(
    node: &SyntaxNode<CssLanguage>,
    line_index: &LineIndex,
    encoding: PositionEncoding,
) -> Vec<ColorInformation> {
    let mut colors = Vec::new();

    // PERF: This implementation will traverse the entire tree of the CSS file, matching many unnecessary nodes.
    // A more efficient implementation would instead only look for colors in relevant contexts (e.g. CSS values, function parameters etc.)

    node.children().for_each(|child| {
        match child.kind() {
            CssSyntaxKind::CSS_FUNCTION => {
                // CSS functions come in many forms and shapes, and are sometimes colors themselves or container of other colors
                // In our case, we only care about functions that are colors (rgb, hsl, etc.) as the other branches will cover
                // functions that contain colors (e.g. linear-gradient, light-dark, etc.)
                if let Some(function_name) = child.first_child().map(|n| n.text().to_string()) {
                    if matches!(
                        function_name.as_str(),
                        "rgb"
                            | "rgba"
                            | "hsl"
                            | "hsla"
                            | "hwb"
                            | "lab"
                            | "lch"
                            | "hwba"
                            | "hsv"
                            | "hsva"
                    ) {
                        if let Ok(function_color) = parse_color(&node.text().to_string()) {
                            colors.push(ColorInformation {
                                color: convert_parsed_color(function_color),
                                range: range(line_index, node.text_range(), encoding).unwrap(),
                            });
                        }
                    }
                }
            }
            // Any CSS identifier, such as a property name or basic value (ex: `color: red;` contains two identifiers)
            CssSyntaxKind::CSS_IDENTIFIER => {
                if let Some(color) = NAMED_COLORS.get(&node.text().to_string()).map(|color| {
                    csscolorparser::Color::from_rgba8(color[0], color[1], color[2], 255)
                }) {
                    colors.push(ColorInformation {
                        color: convert_parsed_color(color),
                        range: range(line_index, node.text_range(), encoding).unwrap(),
                    });
                }
            }
            // HEX colors
            CssSyntaxKind::CSS_COLOR => {
                if let Ok(color) = parse_color(&node.text().to_string()) {
                    colors.push(ColorInformation {
                        color: convert_parsed_color(color),
                        range: range(line_index, node.text_range(), encoding).unwrap(),
                    });
                }
            }
            _ => {}
        }
    });

    // TODO: Handle CSS variables

    for child in node.children() {
        colors.extend(extract_colors_information(&child, line_index, encoding));
    }

    colors
}

fn find_document_colors(
    css: &CssParse,
    line_index: &LineIndex,
    encoding: PositionEncoding,
) -> Vec<ColorInformation> {
    let binding = css.tree().rules();
    extract_colors_information(binding.syntax(), line_index, encoding)
}

fn compute_color_presentations(color: ColorInformation, range: Range) -> Vec<ColorPresentation> {
    let parsed_color = csscolorparser::Color::from_rgba8(
        (color.color.red * 255.0).round() as u8,
        (color.color.green * 255.0).round() as u8,
        (color.color.blue * 255.0).round() as u8,
        (color.color.alpha * 255.0).round() as u8,
    );

    let rgb_color = parsed_color.to_rgba8();
    let rgb_string = if parsed_color.a == 1.0 {
        format!("rgb({} {} {})", rgb_color[0], rgb_color[1], rgb_color[2])
    } else {
        format!(
            "rgb({} {} {} / {}%)",
            rgb_color[0],
            rgb_color[1],
            rgb_color[2],
            (parsed_color.a * 100.0).round()
        )
    };

    let hsl_color = parsed_color.to_hsla();
    let hsl_string = if hsl_color[3] == 1.0 {
        format!(
            "hsl({} {}% {}%)",
            hsl_color[0].round(),
            (hsl_color[1] * 100.0).round(),
            (hsl_color[2] * 100.0).round()
        )
    } else {
        format!(
            "hsl({} {}% {}% / {}%)",
            hsl_color[0].round(),
            (hsl_color[1] * 100.0).round(),
            (hsl_color[2] * 100.0).round(),
            (hsl_color[3] * 100.0).round()
        )
    };

    let hwb_color = parsed_color.to_hwba();
    let hwb_string = if hwb_color[3] == 1.0 {
        format!(
            "hwb({} {}% {}%)",
            hwb_color[0].round(),
            (hwb_color[1] * 100.0).round(),
            (hwb_color[2] * 100.0).round()
        )
    } else {
        format!(
            "hwb({} {}% {}% / {}%)",
            hwb_color[0].round(),
            (hwb_color[1] * 100.0).round(),
            (hwb_color[2] * 100.0).round(),
            (hwb_color[3] * 100.0).round()
        )
    };

    let color_texts = vec![
        rgb_string,
        parsed_color.to_hex_string(),
        hsl_string,
        hwb_string,
    ];

    color_texts
        .into_iter()
        .map(|text| ColorPresentation {
            label: text.clone(),
            text_edit: Some(TextEdit {
                range,
                new_text: text,
            }),
            additional_text_edits: None,
        })
        .collect()
}

impl LanguageService {
    pub fn get_document_colors(&mut self, document: TextDocumentItem) -> Vec<ColorInformation> {
        let store_entry = self.store.get_or_update_document(document);

        find_document_colors(
            &store_entry.css_tree,
            &store_entry.line_index,
            self.encoding,
        )
    }

    pub fn get_color_presentations(
        &self,
        color: ColorInformation,
        range: Range,
    ) -> Vec<ColorPresentation> {
        compute_color_presentations(color, range)
    }
}

#[cfg(feature = "wasm")]
mod wasm_bindings {
    use crate::{
        converters::{line_index::LineIndex, PositionEncoding},
        parser::parse_css,
    };

    use super::{compute_color_presentations, find_document_colors};
    use serde_wasm_bindgen;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(typescript_custom_section)]
    const TS_APPEND_CONTENT: &'static str = r#"export async function get_document_colors(source: import("vscode-languageserver-textdocument").TextDocument): Promise<import("vscode-languageserver-types").ColorInformation[]>;"#;

    #[wasm_bindgen(skip_typescript)]
    pub fn get_document_colors(document: JsValue) -> JsValue {
        let parsed_text_document = crate::wasm_text_document::create_text_document(document);
        let document_colors = find_document_colors(
            &parse_css(&parsed_text_document.text),
            &LineIndex::new(&parsed_text_document.text),
            PositionEncoding::Wide(crate::converters::WideEncoding::Utf16),
        );

        serde_wasm_bindgen::to_value(&document_colors).unwrap()
    }

    #[wasm_bindgen(typescript_custom_section)]
    const TS_APPEND_CONTENT: &'static str = r#"export async function get_color_presentations(color: import("vscode-languageserver-types").ColorInformation, range: import("vscode-languageserver-types").Range): Promise<import("vscode-languageserver-types").ColorPresentation[]>;"#;

    #[wasm_bindgen(skip_typescript)]
    pub fn get_color_presentations(
        _document: JsValue,
        _color: JsValue,
        _range: JsValue,
    ) -> JsValue {
        let color = serde_wasm_bindgen::from_value(_color).unwrap();
        let range = serde_wasm_bindgen::from_value(_range).unwrap();
        let color_presentations = compute_color_presentations(color, range);

        serde_wasm_bindgen::to_value(&color_presentations).unwrap()
    }
}
