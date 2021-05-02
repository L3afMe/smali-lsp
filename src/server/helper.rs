use std::ops::Range;

use lspower::lsp::{Diagnostic, DiagnosticSeverity, Position, Range as LspRange};

use super::lexer::{Token, TokenType};

pub fn trim_space_tokens(tokens: Vec<Token>) -> Vec<Token> {
    let mut output = Vec::new();
    let mut space_buffer = Vec::new();

    for token in tokens {
        if token.token_type == TokenType::Space {
            // Ignore spaces at the start
            if !output.is_empty() {
                space_buffer.push(token);
            }
        } else {
            // Only push spaces when there is a non-space token
            output.append(&mut space_buffer);
            output.push(token);
            space_buffer.clear();
        }
    }

    output
}

pub fn tokens_to_diagnostic(
    tokens: &[Token],
    message: impl ToString,
    severity: Option<DiagnosticSeverity>,
) -> Diagnostic {
    let range = LspRange {
        start: tokens.first().unwrap().range.start,
        end:   tokens.last().unwrap().range.end,
    };

    Diagnostic {
        range,
        severity,
        message: message.to_string(),
        code: None,
        code_description: None,
        data: None,
        related_information: None,
        source: None,
        tags: None,
    }
}

pub fn pos_to_lsp_pos(input: usize, content: &str) -> Position {
    let line = content.split_at(input).0.split('\n').count() as u32 - 1;
    let character = content.split_at(input).0.split('\n').last().unwrap_or("").len() as u32;

    Position {
        line,
        character,
    }
}

pub fn lsp_pos_to_pos(input: Position, content: &str) -> usize {
    let lines: Vec<&str> = content.split('\n').collect();
    let line = match lines.get(input.line as usize) {
        Some(line) => line,
        None => {
            return content.len();
        },
    };

    let up_to = format!(
        "{}{}{}",
        lines.split_at(input.line as usize).0.join("\n"),
        if input.line > 0 { "\n" } else { "" },
        line.split_at(input.character as usize).0
    );

    up_to.len()
}

pub fn range_to_lsp_range(range: Range<usize>, content: &str) -> LspRange {
    LspRange {
        start: pos_to_lsp_pos(range.start, content),
        end:   pos_to_lsp_pos(range.end, content),
    }
}

pub fn lsp_range_to_range(range: LspRange, content: &str) -> Range<usize> {
    lsp_pos_to_pos(range.start, content)..lsp_pos_to_pos(range.end, content)
}

#[cfg(test)]
mod test {
    use lspower::lsp::{Position, Range};

    use crate::server::{helper::trim_space_tokens, lexer::{TokenType, lex_str}};

    use super::{lsp_pos_to_pos, lsp_range_to_range, pos_to_lsp_pos, range_to_lsp_range};

    #[test]
    fn pos_to_lsp_pos_single_line() {
        let input = "test string";
        let pos = 5;
        let expected = Position {
            line:      0,
            character: 5,
        };

        assert_eq!(expected, pos_to_lsp_pos(pos, input));

        let input = "a longer test string";
        let pos = 10;
        let expected = Position {
            line:      0,
            character: 10,
        };
        assert_eq!(expected, pos_to_lsp_pos(pos, input));
    }

    #[test]
    fn lsp_pos_to_pos_single_line() {
        let input = "test string";
        let pos = Position {
            line:      0,
            character: 5,
        };
        let expected = 5;

        assert_eq!(expected, lsp_pos_to_pos(pos, input));

        let input = "a longer test string";
        let pos = Position {
            line:      0,
            character: 10,
        };
        let expected = 10;
        assert_eq!(expected, lsp_pos_to_pos(pos, input));
    }

    #[test]
    fn pos_to_lsp_pos_multi_line() {
        let input = "test\nstring";
        let pos = Position {
            line:      1,
            character: 2,
        };
        let expected = 7;
        assert_eq!(expected, lsp_pos_to_pos(pos, input));

        let input = "test\nseveral\nline\nstring";
        let pos = Position {
            line:      2,
            character: 2,
        };
        let expected = 15;
        assert_eq!(expected, lsp_pos_to_pos(pos, input));
    }

    #[test]
    fn lsp_pos_to_pos_multi_line() {
        let input = "test\nstring";
        let pos = 7;
        let expected = Position {
            line:      1,
            character: 2,
        };
        assert_eq!(expected, pos_to_lsp_pos(pos, input));

        let input = "test\nseveral\nline\nstring";
        let pos = 15;
        let expected = Position {
            line:      2,
            character: 2,
        };

        assert_eq!(expected, pos_to_lsp_pos(pos, input));
    }

    #[test]
    fn range_to_lsp_range_single_line() {
        let input = "test";
        let rng = 0..3;
        let expected = Range {
            start: Position {
                line:      0,
                character: 0,
            },
            end:   Position {
                line:      0,
                character: 3,
            },
        };
        assert_eq!(expected, range_to_lsp_range(rng, input));

        let input = "test";
        let rng = 1..3;
        let expected = Range {
            start: Position {
                line:      0,
                character: 1,
            },
            end:   Position {
                line:      0,
                character: 3,
            },
        };
        assert_eq!(expected, range_to_lsp_range(rng, input));
    }

    #[test]
    fn lsp_range_to_range_single_line() {
        let input = "test";
        let rng = Range {
            start: Position {
                line:      0,
                character: 0,
            },
            end:   Position {
                line:      0,
                character: 3,
            },
        };
        let expected = 0..3;
        assert_eq!(expected, lsp_range_to_range(rng, input));

        let input = "test";
        let rng = Range {
            start: Position {
                line:      0,
                character: 1,
            },
            end:   Position {
                line:      0,
                character: 3,
            },
        };
        let expected = 1..3;
        assert_eq!(expected, lsp_range_to_range(rng, input));
    }

    #[test]
    fn range_to_lsp_range_multi_line_span_single() {
        let input = "test\nstring";
        let rng = 1..3;
        let expected = Range {
            start: Position {
                line:      0,
                character: 1,
            },
            end:   Position {
                line:      0,
                character: 3,
            },
        };
        assert_eq!(expected, range_to_lsp_range(rng, input));

        let input = "test\nstring";
        let rng = 7..9;
        let expected = Range {
            start: Position {
                line:      1,
                character: 2,
            },
            end:   Position {
                line:      1,
                character: 4,
            },
        };
        assert_eq!(expected, range_to_lsp_range(rng, input));

        let input = "test\nmultiline\n\nstring";
        let rng = 6..8;
        let expected = Range {
            start: Position {
                line:      1,
                character: 1,
            },
            end:   Position {
                line:      1,
                character: 3,
            },
        };
        assert_eq!(expected, range_to_lsp_range(rng, input));
    }

    #[test]
    fn lsp_range_to_range_multi_line_span_single() {
        let input = "test\nstring";
        let rng = Range {
            start: Position {
                line:      0,
                character: 1,
            },
            end:   Position {
                line:      0,
                character: 3,
            },
        };
        let expected = 1..3;
        assert_eq!(expected, lsp_range_to_range(rng, input));

        let input = "test\nstring";
        let rng = Range {
            start: Position {
                line:      1,
                character: 2,
            },
            end:   Position {
                line:      1,
                character: 4,
            },
        };
        let expected = 7..9;
        assert_eq!(expected, lsp_range_to_range(rng, input));

        let input = "test\nmultiline\n\nstring";
        let rng = 6..8;
        let expected = Range {
            start: Position {
                line:      1,
                character: 1,
            },
            end:   Position {
                line:      1,
                character: 3,
            },
        };
        assert_eq!(expected, range_to_lsp_range(rng, input));
    }

    #[test]
    fn range_to_lsp_range_multi_line_span_multiple() {
        let input = "test\nstring";
        let rng = 3..7;
        let expected = Range {
            start: Position {
                line:      0,
                character: 3,
            },
            end:   Position {
                line:      1,
                character: 2,
            },
        };
        assert_eq!(expected, range_to_lsp_range(rng, input));

        let input = "test\nmultiline\n\nstring\n";
        let rng = 3..18;
        let expected = Range {
            start: Position {
                line:      0,
                character: 3,
            },
            end:   Position {
                line:      3,
                character: 2,
            },
        };
        assert_eq!(expected, range_to_lsp_range(rng, input));
    }

    #[test]
    fn lsp_range_to_range_multi_line_span_multiple() {
        let input = "test\nstring";
        let rng = Range {
            start: Position {
                line:      0,
                character: 3,
            },
            end:   Position {
                line:      1,
                character: 2,
            },
        };
        let expected = 3..7;
        assert_eq!(expected, lsp_range_to_range(rng, input));

        let input = "test\nmultiline\n\nstring\n";
        let rng = Range {
            start: Position {
                line:      0,
                character: 3,
            },
            end:   Position {
                line:      3,
                character: 2,
            },
        };
        let expected = 3..18;
        assert_eq!(expected, lsp_range_to_range(rng, input));
    }

    #[test]
    fn trim_spaces() {
        let mut tokens = trim_space_tokens(lex_str("    .locals 1  ")).into_iter();

        let mut token = tokens.next().unwrap();
        assert_eq!(token.token_type, TokenType::Directive);
        assert_eq!(token.content, ".locals");

        token = tokens.next().unwrap();
        assert_eq!(token.token_type, TokenType::Space);
        assert_eq!(token.content, " ");

        token = tokens.next().unwrap();
        assert_eq!(token.token_type, TokenType::Number);
        assert_eq!(token.content, "1");
    }
}
