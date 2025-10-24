#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
    UnexpectedToken,
    InvalidNumberFormat,
    MissingRightParen,
    UnexpectedEndOfInput,
    ExpectedPipe,
    ExpectedIdentifierInLambdaParams,
    ExpectedCommaOrPipeInLambda,
    ExpectedLeftBraceInLambdaBody,
    ExpectedIdentifierAfterDot,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::UnexpectedToken => write!(f, "Unexpected token"),
            ParserError::InvalidNumberFormat => write!(f, "Invalid number format"),
            ParserError::MissingRightParen => write!(f, "Missing right parenthesis"),
            ParserError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            ParserError::ExpectedPipe => write!(f, "Expected '|' in lambda"),
            ParserError::ExpectedIdentifierInLambdaParams => {
                write!(f, "Expected identifier in lambda parameters")
            }
            ParserError::ExpectedCommaOrPipeInLambda => {
                write!(f, "Expected ',' or '|' in lambda parameters")
            }
            ParserError::ExpectedLeftBraceInLambdaBody => write!(f, "Expected {{ for lambda body"),
            ParserError::ExpectedIdentifierAfterDot => write!(f, "Expected identifier after '.'"),
        }
    }
}

impl std::error::Error for ParserError {}

pub type ParseResult<T> = Result<T, ParserError>;
