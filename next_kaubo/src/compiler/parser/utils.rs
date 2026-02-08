use super::super::lexer::token_kind::KauboTokenKind;

pub fn get_precedence(op: KauboTokenKind) -> i32 {
    match op {
        KauboTokenKind::Equal => 50,
        KauboTokenKind::Or => 60,
        KauboTokenKind::Pipe => 70,
        KauboTokenKind::And => 80,
        KauboTokenKind::DoubleEqual
        | KauboTokenKind::ExclamationEqual
        | KauboTokenKind::GreaterThan
        | KauboTokenKind::LessThan
        | KauboTokenKind::GreaterThanEqual
        | KauboTokenKind::LessThanEqual => 100,
        KauboTokenKind::Plus | KauboTokenKind::Minus => 200,
        KauboTokenKind::Asterisk | KauboTokenKind::Slash => 300,
        KauboTokenKind::Dot => 400,
        KauboTokenKind::Not => 450,
        _ => 0,
    }
}

/// 获取运算符结合性
/// 
/// Returns: `true` 表示左结合，`false` 表示右结合
pub fn get_associativity(op: KauboTokenKind) -> bool {
    match op {
        // 右结合：赋值运算符
        KauboTokenKind::Equal => false,
        // 其他都是左结合
        _ => true,
    }
}
