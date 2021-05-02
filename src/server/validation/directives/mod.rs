mod method;
mod header;

use lspower::lsp::Diagnostic;

use crate::server::lexer::Token;

use self::{header::HeaderValidator, method::MethodValidator};

use super::Validator;

#[derive(Debug, Default)]
pub struct DirectivesValidator {
    header_validator: HeaderValidator,
    method_validator: MethodValidator,
}

impl Validator for DirectivesValidator {
    fn validate_token(&mut self, token: &Token) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        diags.append(&mut self.header_validator.validate_token(token));
        diags.append(&mut self.method_validator.validate_token(token));

        diags
    }

    fn validate_line(&mut self, line: &[Token]) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        diags.append(&mut self.header_validator.validate_line(line));
        diags.append(&mut self.method_validator.validate_line(line));

        diags
    }

    fn validate_end(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        diags.append(&mut self.header_validator.validate_end());
        diags.append(&mut self.method_validator.validate_end());

        diags
    }
}
