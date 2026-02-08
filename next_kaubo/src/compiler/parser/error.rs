#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
    UnexpectedToken,
    InvalidNumberFormat,
    MissingRightParen,
    UnexpectedEndOfInput,
    ExpectedCommaOrPipeInLambda,
    ExpectedIdentifierAfterDot,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::UnexpectedToken => write!(f, "Unexpected token"),
            ParserError::InvalidNumberFormat => write!(f, "Invalid number format"),
            ParserError::MissingRightParen => write!(f, "Missing right parenthesis"),
            ParserError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            ParserError::ExpectedCommaOrPipeInLambda => {
                write!(f, "Expected ',' or '|' in lambda parameters")
            }
            ParserError::ExpectedIdentifierAfterDot => write!(f, "Expected identifier after '.'"),
        }
    }
}

impl std::error::Error for ParserError {}

pub type ParseResult<T> = Result<T, ParserError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let errors = vec![
            (ParserError::UnexpectedToken, "Unexpected token"),
            (ParserError::InvalidNumberFormat, "Invalid number format"),
            (ParserError::MissingRightParen, "Missing right parenthesis"),
            (ParserError::UnexpectedEndOfInput, "Unexpected end of input"),
            (ParserError::ExpectedCommaOrPipeInLambda, "Expected ',' or '|' in lambda parameters"),
            (ParserError::ExpectedIdentifierAfterDot, "Expected identifier after '.'"),
        ];

        for (error, expected) in errors {
            assert_eq!(format!("{}", error), expected);
        }
    }

    #[test]
    fn test_error_clone() {
        let err = ParserError::UnexpectedToken;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_error_equality() {
        assert_eq!(ParserError::UnexpectedToken, ParserError::UnexpectedToken);
        assert_ne!(ParserError::UnexpectedToken, ParserError::MissingRightParen);
    }
}
