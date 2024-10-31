use csslsrs::service::LanguageService;
use lsp_types::{Color, ColorInformation, Position, Range, TextDocumentItem, Uri};
use std::str::FromStr;

#[test]
fn test_hex_color() {
    let mut ls = LanguageService::default();

    assert_color_symbols(
        &mut ls,
        "body { backgroundColor: #ff9977; }",
        vec![ColorInformation {
            color: csscolorparser::parse("#ff9977")
                .map(convert_parsed_color)
                .unwrap(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 24,
                },
                end: Position {
                    line: 0,
                    character: 31,
                },
            },
        }],
    );
}

#[test]
fn test_hsl_color() {
    let mut ls = LanguageService::default();

    assert_color_symbols(
        &mut ls,
        "body { backgroundColor: hsl(0, 0%, 100%); }",
        vec![ColorInformation {
            color: csscolorparser::parse("hsl(0, 0%, 100%)")
                .map(convert_parsed_color)
                .unwrap(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 24,
                },
                end: Position {
                    line: 0,
                    character: 40,
                },
            },
        }],
    );
}

#[test]
fn test_rgb_and_hsl_colors() {
    let mut ls = LanguageService::default();

    assert_color_symbols(
        &mut ls,
        ".oo { color: rgb(1,40,1); borderColor: hsl(120, 75%, 85%) }",
        vec![
            ColorInformation {
                color: csscolorparser::parse("rgb(1,40,1)")
                    .map(convert_parsed_color)
                    .unwrap(),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 13,
                    },
                    end: Position {
                        line: 0,
                        character: 24,
                    },
                },
            },
            ColorInformation {
                color: csscolorparser::parse("hsl(120, 75%, 85%)")
                    .map(convert_parsed_color)
                    .unwrap(),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 39,
                    },
                    end: Position {
                        line: 0,
                        character: 58,
                    },
                },
            },
        ],
    );
}

#[test]
fn test_rgba_color() {
    let mut ls = LanguageService::default();

    assert_color_symbols(
        &mut ls,
        "body { backgroundColor: rgba(1, 40, 1, 0.3); }",
        vec![ColorInformation {
            color: csscolorparser::parse("rgba(1, 40, 1, 0.3)")
                .map(convert_parsed_color)
                .unwrap(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 24,
                },
                end: Position {
                    line: 0,
                    character: 43,
                },
            },
        }],
    );
}

#[test]
fn test_hwb_color() {
    let mut ls = LanguageService::default();

    assert_color_symbols(
        &mut ls,
        "body { backgroundColor: hwb(194 0% 0% / .5); }",
        vec![ColorInformation {
            color: csscolorparser::parse("hwb(194 0% 0% / .5)")
                .map(convert_parsed_color)
                .unwrap(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 24,
                },
                end: Position {
                    line: 0,
                    character: 43,
                },
            },
        }],
    );
}

#[test]
fn test_named_color() {
    let mut ls = LanguageService::default();

    assert_color_symbols(
        &mut ls,
        "body { backgroundColor: red; }",
        vec![ColorInformation {
            color: csscolorparser::parse("red")
                .map(convert_parsed_color)
                .unwrap(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 24,
                },
                end: Position {
                    line: 0,
                    character: 27,
                },
            },
        }],
    );
}

#[test]
fn test_functions_color() {
    let mut ls = LanguageService::default();

    assert_color_symbols(
        &mut ls,
        "body { color: linear-gradient(to right, red, blue); }",
        vec![
            ColorInformation {
                color: csscolorparser::parse("red")
                    .map(convert_parsed_color)
                    .unwrap(),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 40,
                    },
                    end: Position {
                        line: 0,
                        character: 43,
                    },
                },
            },
            ColorInformation {
                color: csscolorparser::parse("blue")
                    .map(convert_parsed_color)
                    .unwrap(),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 45,
                    },
                    end: Position {
                        line: 0,
                        character: 49,
                    },
                },
            },
        ],
    );
}

#[test]
fn test_color_presentations() {
    let mut ls = LanguageService::default();

    assert_color_presentations(
        &mut ls,
        ColorInformation {
            color: csscolorparser::parse("rgb(255, 0, 0)")
                .map(convert_parsed_color)
                .unwrap(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
        },
        vec!["rgb(255 0 0)", "#ff0000", "hsl(0 100% 50%)", "hwb(0 0% 0%)"],
    );

    assert_color_presentations(
        &mut ls,
        ColorInformation {
            color: csscolorparser::parse("rgba(77, 33, 111, 0.5)")
                .map(convert_parsed_color)
                .unwrap(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
        },
        vec![
            "rgb(77 33 111 / 50%)",
            "#4d216f80",
            "hsl(274 54% 28% / 50%)",
            "hwb(274 13% 56% / 50%)",
        ],
    );
}

fn convert_parsed_color(color: csscolorparser::Color) -> Color {
    Color {
        red: color.r,
        green: color.g,
        blue: color.b,
        alpha: color.a,
    }
}

fn assert_color_presentations(
    ls: &mut LanguageService,
    color: ColorInformation,
    expected_presentations_texts: Vec<&str>,
) {
    let range = color.range;
    let presentations = ls.get_color_presentations(color, range);

    assert_eq!(
        presentations.len(),
        expected_presentations_texts.len(),
        "Unexpected number of color presentations"
    );

    for (presentation, expected_text) in presentations
        .iter()
        .zip(expected_presentations_texts.iter())
    {
        assert_eq!(
            presentation.label, *expected_text,
            "Unexpected color presentation text"
        );
        assert_eq!(
            presentation.text_edit.as_ref().unwrap().new_text,
            *expected_text,
            "Unexpected color presentation text edit"
        );
        assert!(presentation.text_edit.as_ref().unwrap().range == range);
    }
}

fn assert_color_symbols(
    ls: &mut LanguageService,
    text: &str,
    expected_colors: Vec<ColorInformation>,
) {
    let document = TextDocumentItem {
        uri: Uri::from_str("file:///test.css").unwrap(),
        language_id: "css".to_string(),
        version: 1,
        text: text.to_string(),
    };

    let colors = ls.get_document_colors(document);

    assert_eq!(
        colors.len(),
        expected_colors.len(),
        "Unexpected number of colors"
    );

    for (color, expected) in colors.iter().zip(expected_colors.iter()) {
        assert_eq!(color, expected, "Unexpected color information");
    }
}
