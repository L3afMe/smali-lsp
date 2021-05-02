use lspower::lsp::{Diagnostic, DiagnosticSeverity};

use super::Validator;
use crate::server::{
    helper::tokens_to_diagnostic,
    lexer::{Token, TokenType},
};

#[derive(Debug)]
pub struct HeaderValidator {
    top_line:           Option<Vec<Token>>,
    super_declaration:  Option<Vec<Token>>,
    class_declaration:  Option<Vec<Token>>,
    source_declaration: Option<Vec<Token>>,
    blank_line:         bool,
    last_token:         Option<Token>,
}

impl Default for HeaderValidator {
    fn default() -> Self {
        Self {
            top_line:           None,
            super_declaration:  None,
            class_declaration:  None,
            source_declaration: None,
            blank_line:         false,
            last_token:         None,
        }
    }
}

impl Validator for HeaderValidator {
    fn validate_token(&mut self, token: &Token) -> Vec<Diagnostic> {
        if token.token_type == TokenType::NewLine {
            if let Some(tkn) = &self.last_token {
                if tkn.token_type == TokenType::NewLine {
                    self.blank_line = true;
                }
            }
        }

        if token.token_type != TokenType::Space {
            self.last_token = Some(token.clone());
        }

        Vec::new()
    }

    fn validate_line(&mut self, line: &[Token]) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        if line[0].token_type == TokenType::Directive {
            match line[0].content.as_ref() {
                ".class" => {
                    if let Some(tokens) = &self.class_declaration {
                        diags.push(tokens_to_diagnostic(
                            tokens,
                            "Class declared here.",
                            Some(DiagnosticSeverity::Hint),
                        ));
                        diags.push(tokens_to_diagnostic(
                            &line,
                            "Class already declared.",
                            Some(DiagnosticSeverity::Error),
                        ));
                    } else {
                        diags.append(&mut validate_class(line.into()));
                        self.class_declaration = Some(line.into());
                    }
                },
                ".super" => {
                    if let Some(tokens) = &self.super_declaration {
                        diags.push(tokens_to_diagnostic(
                            tokens,
                            "Super declared here.",
                            Some(DiagnosticSeverity::Hint),
                        ));
                        diags.push(tokens_to_diagnostic(
                            &line,
                            "Super already declared.",
                            Some(DiagnosticSeverity::Error),
                        ));
                    } else {
                        diags.append(&mut validate_simple(line.into()));
                        self.super_declaration = Some(line.into());
                    }
                },
                ".implements" => {
                    diags.append(&mut validate_simple(line.into()));
                },
                ".source" => {
                    if let Some(tokens) = &self.source_declaration {
                        diags.push(tokens_to_diagnostic(
                            tokens,
                            "Source declared here.",
                            Some(DiagnosticSeverity::Hint),
                        ));
                        diags.push(tokens_to_diagnostic(
                            &line,
                            "Source already declared.",
                            Some(DiagnosticSeverity::Error),
                        ));
                    } else {
                        diags.append(&mut validate_simple(line.into()));
                        self.source_declaration = Some(line.into());
                    }
                },
                _ => {},
            }
        }

        if self.top_line.is_none() {
            self.top_line = Some(line.into());
        }

        diags
    }

    fn validate_end(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        if let Some(top_line) = &self.top_line {
            if self.class_declaration.is_none() {
                diags.push(tokens_to_diagnostic(
                    top_line,
                    "Missing class directive.",
                    Some(DiagnosticSeverity::Error),
                ));
            }

            if self.super_declaration.is_none() {
                diags.push(tokens_to_diagnostic(
                    top_line,
                    "Missing super directive.\nExtend 'Ljava/lang/Object;' by default",
                    Some(DiagnosticSeverity::Error),
                ));
            }
        }

        diags
    }
}

#[derive(Debug, PartialEq)]
enum Stage {
    Modifier,
    Other,
}

fn validate_class(line: Vec<Token>) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    let mut vsblty_decl: Option<Token> = None;
    let mut final_decl: Option<Token> = None;
    let mut synthc_decl: Option<Token> = None;
    let mut stage = Stage::Modifier;

    for (idx, token) in line.iter().enumerate() {
        if idx == 0 {
            // Skip directive
            continue;
        }

        if idx == 1 {
            if token.token_type != TokenType::Space {
                diags.push(token.to_diagnostic("Space expected.", Some(DiagnosticSeverity::Error)));
            }

            continue;
        }

        if stage == Stage::Modifier {
            match token.token_type {
                TokenType::Visibility => {
                    if let Some(vsblty_token) = &vsblty_decl {
                        diags.push(
                            vsblty_token
                                .to_diagnostic("Visibility modifier defined here.", Some(DiagnosticSeverity::Hint)),
                        );
                        diags.push(
                            token
                                .to_diagnostic("Visibility modifier already defined.", Some(DiagnosticSeverity::Error)),
                        );

                        continue;
                    }

                    vsblty_decl = Some(token.clone());
                },
                TokenType::Modifier => match token.content.as_ref() {
                    "static" => {
                        diags.push(
                            token.to_diagnostic("Class cannot be defined as static.", Some(DiagnosticSeverity::Error)),
                        );
                    },
                    "final" => {
                        if let Some(final_token) = &final_decl {
                            diags.push(
                                final_token
                                    .to_diagnostic("Final modifier defined here.", Some(DiagnosticSeverity::Hint)),
                            );
                            diags.push(
                                token.to_diagnostic("Final modifier already defined.", Some(DiagnosticSeverity::Error)),
                            );

                            continue;
                        }

                        final_decl = Some(token.clone());
                    },
                    "synthetic" => {
                        if let Some(synthc_token) = &synthc_decl {
                            diags.push(
                                synthc_token
                                    .to_diagnostic("Synthetic modifier defined here.", Some(DiagnosticSeverity::Hint)),
                            );
                            diags.push(
                                token.to_diagnostic(
                                    "Synthetic modifier already defined.",
                                    Some(DiagnosticSeverity::Error),
                                ),
                            );

                            continue;
                        }

                        synthc_decl = Some(token.clone());
                    },
                    _ => {},
                },
                TokenType::Class => {
                    stage = Stage::Other;
                },
                _ => {},
            }
        } else if token.token_type != TokenType::Space {
            diags.push(token.to_diagnostic("New line expected.", Some(DiagnosticSeverity::Error)));
        }
    }

    diags
}

fn validate_simple(line: Vec<Token>) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    if line.len() < 3 {
        diags.push(tokens_to_diagnostic(
            &line,
            format!(
                "'{} {}'",
                line[0].content,
                if line[0].content == ".source" {
                    "\"FileName\""
                } else {
                    "Lclass/Name;"
                }
            ),
            Some(DiagnosticSeverity::Error),
        ));

        return diags;
    }

    for (idx, token) in line.iter().enumerate() {
        match idx {
            0 => {},
            1 => {
                if token.token_type != TokenType::Space {
                    diags.push(token.to_diagnostic("Space expected.", Some(DiagnosticSeverity::Error)));
                }
            },
            2 => {
                if token.token_type != TokenType::Class && line[0].content != ".source" {
                    diags.push(token.to_diagnostic("Class expected.", Some(DiagnosticSeverity::Error)));
                } else if token.token_type != TokenType::String && line[0].content == ".source" {
                    diags.push(token.to_diagnostic("String expected.", Some(DiagnosticSeverity::Error)));
                }
            },
            _ => {
                diags.push(token.to_diagnostic("New line expected.", Some(DiagnosticSeverity::Error)));
            },
        }
    }

    diags
}
