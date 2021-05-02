use logos::Logos;
use lspower::lsp::{Diagnostic, DiagnosticSeverity, Range};

use super::helper::range_to_lsp_range;

#[derive(Logos, Debug, Clone, PartialEq)]
pub enum TokenType {
    #[token("\n")]
    NewLine,

    #[regex(r"#.*")]
    Comment,

    #[regex(r"public|private|protected")]
    Visibility,

    #[regex(r"static|constructor|final|synthetic")]
    Modifier,

    #[regex(r"( |\t)+")]
    Space,

    #[regex(r"L[a-zA-Z0-9\$_\-\./]*;")]
    Class,

    #[regex(r"(p|v)\d+")]
    Register,

    #[regex(r"\.(method|end method)")]
    Method,

    #[regex(r"\.(field|end field)")]
    Field,

    #[regex(r":(goto|cond)_\d+")]
    Label,

    #[regex(r"\.(class|source|super|implements|locals|local|registers|line|prologue|goto)")]
    Directive,

    #[regex(r"invoke-(direct|static|virtual|interface)(/range)?")]
    Invoke,

    #[token("check-cast")]
    CheckCast,

    #[token("new-instance")]
    NewInstance,

    #[regex(r"const-string(/jumbo|)")]
    ConstString,

    #[regex(r"const/(4|16)")]
    ConstInt,

    #[regex(r"const(-(class|class)|)")]
    Const,

    #[regex(r"if-(lt|le|gt|ge|eq|eq|ne|ne)(z|)")]
    If,

    #[regex(r"iget(-(object|string|wide)|)")]
    IGet,

    #[regex(r"sget(-(object|string|wide)|)")]
    SGet,

    #[regex(r"iput(-(object|string|wide)|)")]
    IPut,

    #[regex(r"sput(-(object|string|wide)|)")]
    SPut,

    #[regex(r"move(-(result(-object|)|)|)")]
    Move,

    #[regex(r"return(-(void|object|wide)|)")]
    Return,

    #[regex("\"[^\"]*\"")]
    String,

    #[regex(r"(-|)(0x|)\d+")]
    Number,

    #[regex(r"\{\{[a-z/a-zA-Z0-9_]*\}\}")]
    TreecordMacro,

    #[regex(r"(\{|\})")]
    Brace,

    #[regex(r"(\(|\))")]
    Paren,

    #[regex(r"(V|Z|B|S|C|I|J|F|D)")]
    BuiltinType,

    #[regex(r"->[a-zA-Z0-9\$<>]+\(")]
    MethodCall,

    #[regex(r"[a-zA-Z0-9\$<>]+\(")]
    MethodName,

    #[regex(r"[a-zA-Z0-9\$]+:")]
    FieldName,

    #[token("[")]
    ArrayOp,

    #[token("..")]
    RangeOp,

    #[token(",")]
    CommaOp,

    #[error]
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub range:      Range,
    pub content:    String,
    pub token_type: TokenType,
}

impl Token {
    pub fn to_diagnostic(
        &self,
        message: impl ToString,
        severity: Option<DiagnosticSeverity>,
    ) -> Diagnostic {
        Diagnostic {
            message: message.to_string(),
            severity,
            range: self.range,
            code: None,
            code_description: None,
            data: None,
            related_information: None,
            source: None,
            tags: None,
        }
    }
}

pub fn lex_str(content: &str) -> Vec<Token> {
    let mut lex = TokenType::lexer(content);
    let mut output = Vec::new();

    while let Some(token_type) = lex.next() {
        output.push(Token {
            token_type,
            content: lex.slice().to_string(),
            range: range_to_lsp_range(lex.span(), content),
        });
    }

    output
}

#[cfg(test)]
mod test {
    use logos::Logos;

    use super::{lex_str, Token, TokenType};
    use crate::server::helper::range_to_lsp_range;

    #[test]
    fn test_lex_str() {
        let content = ".class public Ltest/Test;";
        let mut lex = lex_str(content).into_iter();
        assert_eq!(
            lex.next(),
            Some(Token {
                range:      range_to_lsp_range(0..6, content),
                token_type: TokenType::Directive,
                content:    ".class".to_string(),
            })
        );
        assert_eq!(
            lex.next(),
            Some(Token {
                range:      range_to_lsp_range(6..7, content),
                token_type: TokenType::Space,
                content:    " ".to_string(),
            })
        );
        assert_eq!(
            lex.next(),
            Some(Token {
                range:      range_to_lsp_range(7..13, content),
                token_type: TokenType::Visibility,
                content:    "public".to_string(),
            })
        );
        assert_eq!(
            lex.next(),
            Some(Token {
                range:      range_to_lsp_range(13..14, content),
                token_type: TokenType::Space,
                content:    " ".to_string(),
            })
        );
        assert_eq!(
            lex.next(),
            Some(Token {
                range:      range_to_lsp_range(14..25, content),
                token_type: TokenType::Class,
                content:    "Ltest/Test;".to_string(),
            })
        );
    }

    #[test]
    fn test_comment() {
        let mut lex = TokenType::lexer("# Test");
        assert_eq!(lex.next(), Some(TokenType::Comment));
        assert_eq!(lex.slice(), "# Test");

        let mut lex = TokenType::lexer("# Test\n");
        assert_eq!(lex.next(), Some(TokenType::Comment));
        assert_eq!(lex.slice(), "# Test");
        assert_eq!(lex.next(), Some(TokenType::NewLine));
        assert_eq!(lex.slice(), "\n");
    }

    #[test]
    fn test_method_field_name() {
        let mut lex = TokenType::lexer(".field private bool:Z\n.method public getBool()Z");
        assert_eq!(lex.next(), Some(TokenType::Field));
        assert_eq!(lex.slice(), ".field");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Visibility));
        assert_eq!(lex.slice(), "private");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::FieldName));
        assert_eq!(lex.slice(), "bool:");
        assert_eq!(lex.next(), Some(TokenType::BuiltinType));
        assert_eq!(lex.slice(), "Z");
        assert_eq!(lex.next(), Some(TokenType::NewLine));
        assert_eq!(lex.next(), Some(TokenType::Method));
        assert_eq!(lex.slice(), ".method");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Visibility));
        assert_eq!(lex.slice(), "public");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::MethodName));
        assert_eq!(lex.slice(), "getBool(");
        assert_eq!(lex.next(), Some(TokenType::Paren));
        assert_eq!(lex.next(), Some(TokenType::BuiltinType));
        assert_eq!(lex.slice(), "Z");

        let mut lex = TokenType::lexer(".method public getBool()Z\n.field private bool:Z");
        assert_eq!(lex.next(), Some(TokenType::Method));
        assert_eq!(lex.slice(), ".method");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Visibility));
        assert_eq!(lex.slice(), "public");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::MethodName));
        assert_eq!(lex.slice(), "getBool(");
        assert_eq!(lex.next(), Some(TokenType::Paren));
        assert_eq!(lex.next(), Some(TokenType::BuiltinType));
        assert_eq!(lex.slice(), "Z");
        assert_eq!(lex.next(), Some(TokenType::NewLine));
        assert_eq!(lex.next(), Some(TokenType::Field));
        assert_eq!(lex.slice(), ".field");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Visibility));
        assert_eq!(lex.slice(), "private");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::FieldName));
        assert_eq!(lex.slice(), "bool:");
        assert_eq!(lex.next(), Some(TokenType::BuiltinType));
        assert_eq!(lex.slice(), "Z");
    }
}

#[cfg(test)]
mod test_instructions {
    use logos::Logos;

    use super::TokenType;

    #[test]
    fn test_invoke() {
        let mut lex = TokenType::lexer("    invoke-direct {p0}, Ljava/lang/Object;-><init>()V");

        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Invoke));
        assert_eq!(lex.slice(), "invoke-direct");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Brace));
        assert_eq!(lex.next(), Some(TokenType::Register));
        assert_eq!(lex.slice(), "p0");
        assert_eq!(lex.next(), Some(TokenType::Brace));
        assert_eq!(lex.next(), Some(TokenType::CommaOp));
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Class));
        assert_eq!(lex.slice(), "Ljava/lang/Object;");
        assert_eq!(lex.next(), Some(TokenType::MethodCall));
        assert_eq!(lex.slice(), "-><init>(");
        assert_eq!(lex.next(), Some(TokenType::Paren));
        assert_eq!(lex.next(), Some(TokenType::BuiltinType));
        assert_eq!(lex.slice(), "V");
    }
}

#[cfg(test)]
mod test_directives {
    use logos::Logos;

    use super::TokenType;

    #[test]
    fn test_class() {
        let mut lex = TokenType::lexer(".class public final Lme/l3af/Test;");

        assert_eq!(lex.next(), Some(TokenType::Directive));
        assert_eq!(lex.slice(), ".class");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Visibility));
        assert_eq!(lex.slice(), "public");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Modifier));
        assert_eq!(lex.slice(), "final");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Class));
        assert_eq!(lex.slice(), "Lme/l3af/Test;");
    }

    #[test]
    fn test_source() {
        let mut lex = TokenType::lexer(".source \"TreecordCommands.smali\"");

        assert_eq!(lex.next(), Some(TokenType::Directive));
        assert_eq!(lex.slice(), ".source");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::String));
        assert_eq!(lex.slice(), "\"TreecordCommands.smali\"");
    }

    #[test]
    fn test_super() {
        let mut lex = TokenType::lexer(".super Ljava/lang/Object;");

        assert_eq!(lex.next(), Some(TokenType::Directive));
        assert_eq!(lex.slice(), ".super");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Class));
        assert_eq!(lex.slice(), "Ljava/lang/Object;");
    }

    #[test]
    fn test_field() {
        let mut lex = TokenType::lexer(".field private static final Obj:Ljava/lang/Object;");

        assert_eq!(lex.next(), Some(TokenType::Field));
        assert_eq!(lex.slice(), ".field");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Visibility));
        assert_eq!(lex.slice(), "private");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Modifier));
        assert_eq!(lex.slice(), "static");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Modifier));
        assert_eq!(lex.slice(), "final");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::FieldName));
        assert_eq!(lex.slice(), "Obj:");
        assert_eq!(lex.next(), Some(TokenType::Class));
        assert_eq!(lex.slice(), "Ljava/lang/Object;");
    }

    #[test]
    fn test_method_start() {
        let mut lex = TokenType::lexer(".method public static constructor <clinit>()V");

        assert_eq!(lex.next(), Some(TokenType::Method));
        assert_eq!(lex.slice(), ".method");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Visibility));
        assert_eq!(lex.slice(), "public");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Modifier));
        assert_eq!(lex.slice(), "static");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Modifier));
        assert_eq!(lex.slice(), "constructor");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::MethodName));
        assert_eq!(lex.slice(), "<clinit>(");
        assert_eq!(lex.next(), Some(TokenType::Paren));
        assert_eq!(lex.next(), Some(TokenType::BuiltinType));
        assert_eq!(lex.slice(), "V");
    }

    #[test]
    fn test_method_end() {
        let mut lex = TokenType::lexer(".end method");

        assert_eq!(lex.next(), Some(TokenType::Method));
        assert_eq!(lex.slice(), ".end method");
    }

    #[test]
    fn test_goto() {
        let mut lex = TokenType::lexer(".goto :goto_12");

        assert_eq!(lex.next(), Some(TokenType::Directive));
        assert_eq!(lex.slice(), ".goto");
        assert_eq!(lex.next(), Some(TokenType::Space));
        assert_eq!(lex.next(), Some(TokenType::Label));
        assert_eq!(lex.slice(), ":goto_12");
    }
}
