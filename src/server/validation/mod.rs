mod directives;

use lspower::lsp::Diagnostic;

use self::directives::DirectivesValidator;
use super::{helper::trim_space_tokens, lexer::{lex_str, Token, TokenType}};

pub fn validate(content: String) -> Result<Vec<Diagnostic>, String> {
    let tokens = lex_str(&content);
    let mut diags = Vec::new();

    let mut directives_validator = DirectivesValidator::default();

    let mut current_line = Vec::new();
    for token in tokens {
        if token.token_type == TokenType::NewLine {
            let line = trim_space_tokens(current_line);
            if !line.is_empty() {
                diags.append(&mut directives_validator.validate_line(&line));
            }

            current_line = Vec::new();
        } else {
            current_line.push(token.clone())
        }

        diags.append(&mut directives_validator.validate_token(&token));
    }

    diags.append(&mut directives_validator.validate_end());

    Ok(diags)
}

trait Validator {
    fn validate_token(&mut self, token: &Token) -> Vec<Diagnostic>;
    fn validate_line(&mut self, line: &[Token]) -> Vec<Diagnostic>;
    fn validate_end(&self) -> Vec<Diagnostic>;
}
