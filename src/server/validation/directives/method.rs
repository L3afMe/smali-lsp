use lspower::lsp::{Diagnostic, DiagnosticSeverity};

use super::Validator;
use crate::server::{
    helper::tokens_to_diagnostic,
    lexer::{Token, TokenType},
};

#[derive(Debug)]
pub struct MethodValidator {
    method_decl:         Option<MethodDeclaration>,
    constructor_static:  Option<MethodDeclaration>,
    constructor_virtual: Option<MethodDeclaration>,
}

#[derive(Debug, Clone)]
struct MethodDeclaration {
    is_start:     bool,
    found_return: bool,
    tokens:       Vec<Token>,
    return_type:  ReturnType,
}

#[derive(Debug, Clone)]
enum MethodDeclarationStage {
    Modifiers,
    Params,
    ReturnType,
}

#[derive(Debug, Clone)]
enum ReturnType {
    None,
    Void,
    BuiltinType(String),
    Class(String),
}

macro_rules! breakable {
    ($xs:block) => {
        loop {
            let _ = $xs;
            break;
        }
    };
}

impl Default for MethodValidator {
    fn default() -> Self {
        Self {
            method_decl:         None,
            constructor_static:  None,
            constructor_virtual: None,
        }
    }
}

impl Validator for MethodValidator {
    fn validate_token(&mut self, token: &Token) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        #[allow(clippy::single_match)]
        match token.token_type {
            TokenType::Return => {
                diags.append(&mut validate_method_token(token, self));
            },
            _ => {},
        }

        diags
    }

    fn validate_line(&mut self, line: &[Token]) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        #[allow(clippy::single_match)]
        match line[0].token_type {
            TokenType::Method => {
                diags.append(&mut validate_method_declaration(line, self));
            },
            _ => {},
        }

        diags
    }

    fn validate_end(&self) -> Vec<Diagnostic> {
        Vec::new()
    }
}

fn validate_method_token(token: &Token, validator: &mut MethodValidator) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    if let Some(mut method) = validator.method_decl.clone() {
        method.found_return = true;
        validator.method_decl = Some(method.clone());

        match method.return_type {
            ReturnType::None => {
                diags.push(token.to_diagnostic(
                    "Unable to get return type from method declaration.",
                    Some(DiagnosticSeverity::Information),
                ));
            },
            ReturnType::Void => {
                if token.content != "return-void" {
                    diags.push(
                        method
                            .tokens
                            .last()
                            .unwrap()
                            .to_diagnostic("Return type declared here.", Some(DiagnosticSeverity::Hint)),
                    );
                    diags.push(token.to_diagnostic("'return-void' expected.", Some(DiagnosticSeverity::Error)));
                }
            },
            ReturnType::Class(_) => {
                if token.content != "return-object" {
                    diags.push(
                        method
                            .tokens
                            .last()
                            .unwrap()
                            .to_diagnostic("Return type declared here.", Some(DiagnosticSeverity::Hint)),
                    );
                    diags.push(token.to_diagnostic("'return-object' expected.", Some(DiagnosticSeverity::Error)));
                }
            },
            _ => {},
        }
    }

    diags
}

fn validate_method_declaration(line: &[Token], validator: &mut MethodValidator) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    if line[0].content == ".method" {
        let mut method_decl = validate_method_declaration_line(line, validator);

        let mut valid_placement = true;
        if let Some(method) = &validator.method_decl {
            if method.is_start {
                diags.push(tokens_to_diagnostic(
                    &method.tokens,
                    "Method block starts here.",
                    Some(DiagnosticSeverity::Hint),
                ));
                diags.push(tokens_to_diagnostic(
                    line,
                    "'.method' directive cannot be inside a method block.",
                    Some(DiagnosticSeverity::Error),
                ));
                valid_placement = false;
            }
        }

        if valid_placement {
            diags.append(&mut method_decl.0);
        }

        validator.method_decl = Some(MethodDeclaration {
            is_start:     true,
            found_return: false,
            tokens:       line.into(),
            return_type:  method_decl.1,
        });
    } else if let Some(method) = &validator.method_decl {
        if !method.is_start {
            diags.push(tokens_to_diagnostic(
                &method.tokens,
                "Method block ends here.",
                Some(DiagnosticSeverity::Hint),
            ));
            diags.push(tokens_to_diagnostic(
                line,
                "'.end method' directive must be at the end of a method block.",
                Some(DiagnosticSeverity::Error),
            ));
        } else {
            if !method.found_return {
                diags.push(tokens_to_diagnostic(
                    &method.tokens,
                    "No return instruction found in method block.",
                    Some(DiagnosticSeverity::Error),
                ));
            }

            validator.method_decl = Some(MethodDeclaration {
                is_start:     false,
                found_return: false,
                tokens:       line.into(),
                return_type:  ReturnType::None,
            });
        }
    } else {
        diags.push(tokens_to_diagnostic(
            line,
            "'.end method' directive must be at the end of a method block.",
            Some(DiagnosticSeverity::Error),
        ));
    }

    diags
}

fn validate_method_declaration_line(line: &[Token], validator: &mut MethodValidator) -> (Vec<Diagnostic>, ReturnType) {
    let mut diags = Vec::new();
    let mut return_type = ReturnType::None;

    let mut vsblty_decl: Option<Token> = None;
    let mut static_decl: Option<Token> = None;
    let mut final_decl: Option<Token> = None;
    let mut const_decl: Option<Token> = None;
    let mut stage = MethodDeclarationStage::Modifiers;
    let mut has_return_type = false;
    let mut was_space = false;

    for (idx, token) in line.iter().enumerate() {
        if idx == 0 {
            // Skip directive
            continue;
        }

        match stage {
            MethodDeclarationStage::Modifiers => breakable!({
                if !was_space && token.token_type != TokenType::Space {
                    diags.push(token.to_diagnostic("Space expected.", Some(DiagnosticSeverity::Error)));
                    break;
                }

                match token.token_type {
                    TokenType::Visibility => {
                        if let Some(visibility_token) = &vsblty_decl {
                            diags.push(
                                visibility_token.to_diagnostic(
                                    "Visibility modifier declared here.",
                                    Some(DiagnosticSeverity::Hint),
                                ),
                            );
                            diags.push(token.to_diagnostic(
                                "Visibility modifier already declared.",
                                Some(DiagnosticSeverity::Error),
                            ));
                            break;
                        }
                        
                        vsblty_decl = Some(token.clone());
                    },
                    TokenType::Modifier => {
                        match token.content.as_ref() {
                            "constructor" => {
                                if let Some(constructor_token) = &const_decl {
                                    diags.push(constructor_token.to_diagnostic(
                                        "Constuctor modifier declared here.",
                                        Some(DiagnosticSeverity::Hint),
                                    ));
                                    diags.push(token.to_diagnostic(
                                        "Constuctor modifier already declared.",
                                        Some(DiagnosticSeverity::Error),
                                    ));
                                    break;
                                }

                                const_decl = Some(token.clone());
                            },
                            "final" => {
                                if let Some(final_token) = &final_decl {
                                    diags.push(final_token.to_diagnostic(
                                        "Final modifier declared here.",
                                        Some(DiagnosticSeverity::Hint),
                                    ));
                                    diags.push(token.to_diagnostic(
                                        "Final modifier already declared.",
                                        Some(DiagnosticSeverity::Error),
                                    ));
                                    break;
                                }

                                final_decl = Some(token.clone());
                            },
                            "static" => {
                                if let Some(static_token) = &static_decl {
                                    diags.push(static_token.to_diagnostic(
                                        "Static modifier declared here.",
                                        Some(DiagnosticSeverity::Hint),
                                    ));
                                    diags.push(token.to_diagnostic(
                                        "Static modifier already declared.",
                                        Some(DiagnosticSeverity::Error),
                                    ));
                                    break;
                                }

                                static_decl = Some(token.clone());
                            },
                            _ => {},
                        }
                    },
                    TokenType::MethodName => {
                        if let Some(constructor_token) = &const_decl {
                            if let Some(static_token) = &static_decl {
                                if token.content != "<clinit>(" {
                                    diags.push(constructor_token.to_diagnostic(
                                        "Constuctor modifier declared here.",
                                        Some(DiagnosticSeverity::Error),
                                    ));
                                    diags.push(static_token.to_diagnostic(
                                        "Static modifier declared here.",
                                        Some(DiagnosticSeverity::Error),
                                    ));
                                    diags.push(token.to_diagnostic(
                                        "Static constuctor must be named '<clinit>'.",
                                        Some(DiagnosticSeverity::Error),
                                    ));
                                }
                            } else if token.content != "<init>(" {
                                diags.push(constructor_token.to_diagnostic(
                                    "Constuctor modifier declared here.",
                                    Some(DiagnosticSeverity::Error),
                                ));
                                diags.push(token.to_diagnostic(
                                    "Non-static constuctor must be named '<init>'.",
                                    Some(DiagnosticSeverity::Error),
                                ));
                            }
                        } else if token.content == "<init>(" {
                            diags.push(token.to_diagnostic(
                                "'<init>' is reserved for nonstatic constructors.",
                                Some(DiagnosticSeverity::Error),
                            ));
                        } else if token.content == "<clinit>(" {
                            diags.push(token.to_diagnostic(
                                "'<clinit>' is reserved for static constructors.",
                                Some(DiagnosticSeverity::Error),
                            ));
                        }
                        stage = MethodDeclarationStage::Params;
                    },
                    TokenType::Space => {},
                    _ => {
                        diags.push(token.to_diagnostic("Method modifier expected.", Some(DiagnosticSeverity::Error)));
                    },
                }
            }),
            MethodDeclarationStage::Params => breakable!({match token.token_type {
                TokenType::BuiltinType | TokenType::Class => {},
                _ => {
                    if token.content == ")" {
                        stage = MethodDeclarationStage::ReturnType;
                        break;
                    }

                    diags.push(token.to_diagnostic("')' expected.", Some(DiagnosticSeverity::Error)));
                },
            }}),
            MethodDeclarationStage::ReturnType => breakable!({
                if has_return_type {
                    if token.token_type != TokenType::Space {
                        diags.push(token.to_diagnostic("New line expected.", Some(DiagnosticSeverity::Error)));
                    }
                    break;
                }

                match token.token_type {
                    TokenType::BuiltinType => {
                        has_return_type = true;

                        return_type = if token.content == "V" {
                            ReturnType::Void
                        } else {
                            ReturnType::BuiltinType(token.content.clone())
                        };
                    },
                    TokenType::Class => {
                        has_return_type = true;
                        return_type = ReturnType::Class(token.content.clone());
                    },
                    _ => {
                        diags.push(
                            token
                                .to_diagnostic("Return type expected.\n'V' for void.", Some(DiagnosticSeverity::Error)),
                        );
                    },
                }
            }),
        }

        was_space = token.token_type == TokenType::Space;
    }

    if const_decl.is_some() {
        if static_decl.is_some() {
            if let Some(constructor_static) = &validator.constructor_static {
                diags.push(tokens_to_diagnostic(
                    &constructor_static.tokens,
                    "Static constuctor defined here.",
                    Some(DiagnosticSeverity::Hint),
                ));
                diags.push(tokens_to_diagnostic(
                    line,
                    "Static constuctor already defined.",
                    Some(DiagnosticSeverity::Error),
                ));
            } else {
                validator.constructor_static = Some(MethodDeclaration {
                    is_start:     true,
                    found_return: true,
                    tokens:       line.into(),
                    return_type:  ReturnType::Void,
                });
            }
        } else if let Some(constructor_virtual) = &validator.constructor_virtual {
            diags.push(tokens_to_diagnostic(
                &constructor_virtual.tokens,
                "Constuctor defined here.",
                Some(DiagnosticSeverity::Hint),
            ));
            diags.push(tokens_to_diagnostic(
                line,
                "Constuctor already defined.",
                Some(DiagnosticSeverity::Error),
            ));
        } else {
            validator.constructor_virtual = Some(MethodDeclaration {
                is_start:     true,
                found_return: true,
                tokens:       line.into(),
                return_type:  ReturnType::Void,
            });
        }
    }

    (diags, return_type)
}
