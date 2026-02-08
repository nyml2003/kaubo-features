use super::error::BuildMachineError;
use super::machine::Machine;
use super::types::TokenKindTrait;

pub fn build_keyword_machine<Token>(
    keyword: &str,
    token: Token,
) -> Result<Machine<Token>, BuildMachineError>
where
    Token: TokenKindTrait,
{
    let mut machine = Machine::new(token);
    let mut current_state = machine.get_current_state();
    for (i, c) in keyword.chars().enumerate() {
        let is_accepting = i == keyword.len() - 1;
        let next_state = machine.add_state(is_accepting);
        machine.add_transition(current_state, next_state, Box::new(move |event| event == c))?;
        current_state = next_state;
    }

    Ok(machine)
}

pub fn build_string_machine<Token>(token_string: Token) -> Result<Machine<Token>, BuildMachineError>
where
    Token: TokenKindTrait + 'static,
{
    let mut machine = Machine::new(token_string);
    let s0 = machine.get_current_state();
    let s1 = machine.add_state(false);
    let s2 = machine.add_state(true);
    let s3 = machine.add_state(false);
    let s4 = machine.add_state(true);

    machine
        .add_transition(s0, s1, Box::new(|c| c == '"'))
        .unwrap();
    machine
        .add_transition(s1, s2, Box::new(|c| c == '"'))
        .unwrap();
    machine
        .add_transition(s1, s1, Box::new(|c| c != '"'))
        .unwrap();
    machine
        .add_transition(s0, s3, Box::new(|c| c == '\''))
        .unwrap();
    machine
        .add_transition(s3, s4, Box::new(|c| c == '\''))
        .unwrap();
    machine
        .add_transition(s3, s3, Box::new(|c| c != '\''))
        .unwrap();

    Ok(machine)
}

pub fn build_single_char_machine<Token>(
    token: Token,
    c: char,
) -> Result<Machine<Token>, BuildMachineError>
where
    Token: TokenKindTrait + 'static,
{
    let mut machine = Machine::new(token);
    let s0 = machine.get_current_state();
    let s1 = machine.add_state(true);
    machine.add_transition(s0, s1, Box::new(move |event| event == c))?;
    Ok(machine)
}

pub fn build_multi_char_machine<Token>(
    token: Token,
    chars: Vec<char>,
) -> Result<Machine<Token>, BuildMachineError>
where
    Token: TokenKindTrait + 'static,
{
    if chars.is_empty() {
        return Err(BuildMachineError::EmptyCharSequence);
    }
    let mut machine = Machine::new(token);
    let mut current_state = machine.get_current_state();
    let len = chars.len() - 1;
    for (i, c) in chars.into_iter().enumerate() {
        let is_final = i == len; // 最后一个字符对应的状态为终态
        let next_state = machine.add_state(is_final);
        machine.add_transition(current_state, next_state, Box::new(move |event| event == c))?;
        current_state = next_state;
    }

    Ok(machine)
}

pub fn build_integer_machine<Token>(token: Token) -> Result<Machine<Token>, BuildMachineError>
where
    Token: TokenKindTrait + 'static,
{
    let mut machine = Machine::new(token);
    let s0 = machine.get_current_state();
    // 接受无限多个0-9
    let s1 = machine.add_state(true);
    machine.add_transition(s0, s1, Box::new(|c| c.is_ascii_digit()))?;
    machine.add_transition(s1, s1, Box::new(|c| c.is_ascii_digit()))?;
    Ok(machine)
}

pub fn build_newline_machine<Token>(token: Token) -> Result<Machine<Token>, BuildMachineError>
where
    Token: TokenKindTrait + 'static,
{
    let mut machine = Machine::new(token);
    let s0 = machine.get_current_state();
    let s1 = machine.add_state(true);
    machine.add_transition(s0, s1, Box::new(|c| c == '\n'))?;
    Ok(machine)
}

pub fn build_whitespace_machine<Token>(token: Token) -> Result<Machine<Token>, BuildMachineError>
where
    Token: TokenKindTrait + 'static,
{
    let mut machine = Machine::new(token);
    let s0 = machine.get_current_state();
    let s1 = machine.add_state(true);
    // 只匹配空格，不换行或制表符（它们有各自的状态机）
    machine.add_transition(s0, s1, Box::new(|c| c == ' '))?;
    Ok(machine)
}

pub fn build_tab_machine<Token>(token: Token) -> Result<Machine<Token>, BuildMachineError>
where
    Token: TokenKindTrait + 'static,
{
    let mut machine = Machine::new(token);
    let s0 = machine.get_current_state();
    let s1 = machine.add_state(true);
    machine.add_transition(s0, s1, Box::new(|c| c == '\t'))?;
    Ok(machine)
}

pub fn build_comment_machine<Token>(token: Token) -> Result<Machine<Token>, BuildMachineError>
where
    Token: TokenKindTrait + 'static,
{
    // 创建状态机，使用注释对应的Token类型
    let mut machine = Machine::new(token);

    // 获取初始状态（s0）
    let s0 = machine.get_current_state();

    // 定义所有状态
    let s1 = machine.add_state(false); // 识别到第一个 '/'
    let s2 = machine.add_state(true); // 单行注释状态（接受状态）
    let s3 = machine.add_state(false); // 多行注释的 '*' 状态
    let s4 = machine.add_state(false); // 多行注释内容状态
    let s5 = machine.add_state(false); // 多行注释中遇到 '*'
    let s6 = machine.add_state(true); // 多行注释结束（接受状态）

    // 初始状态转换：遇到 '/' 进入 s1
    machine.add_transition(s0, s1, Box::new(|c| c == '/'))?;

    // 单行注释路径：s1 -> s2（识别到第二个 '/'）
    machine.add_transition(s1, s2, Box::new(|c| c == '/'))?;

    // 单行注释内容：保持在s2直到换行符
    machine.add_transition(s2, s2, Box::new(|c| c != '\n' && c != '\r'))?;

    // 多行注释路径：s1 -> s3（识别到 '*'）
    machine.add_transition(s1, s3, Box::new(|c| c == '*'))?;

    // 多行注释进入内容状态：s3 -> s4（任意字符）
    machine.add_transition(s3, s4, Box::new(|_| true))?;

    // 多行注释内容处理：
    // 非 '*' 字符保持在s4
    machine.add_transition(s4, s4, Box::new(|c| c != '*'))?;
    // 遇到 '*' 进入s5
    machine.add_transition(s4, s5, Box::new(|c| c == '*'))?;

    // 多行注释中已遇到 '*' 的处理（s5状态）：
    // 遇到 '/' 结束注释（进入s6）
    machine.add_transition(s5, s6, Box::new(|c| c == '/'))?;
    // 遇到 '*' 保持在s5
    machine.add_transition(s5, s5, Box::new(|c| c == '*'))?;
    // 遇到其他字符回到s4
    machine.add_transition(s5, s4, Box::new(|c| c != '*' && c != '/'))?;

    // 注释结束状态：不再接受任何字符
    machine.add_transition(s6, s6, Box::new(|_| false))?;

    Ok(machine)
}

pub fn build_identifier_machine<Token>(token: Token) -> Result<Machine<Token>, BuildMachineError>
where
    Token: TokenKindTrait + 'static,
{
    let mut machine = Machine::new(token);
    let s0 = machine.get_current_state();
    let s1 = machine.add_state(true);
    machine.add_transition(s0, s1, Box::new(|c| c.is_ascii_alphabetic() || c == '_'))?;
    machine.add_transition(s1, s1, Box::new(|c| c.is_ascii_alphanumeric() || c == '_'))?;
    Ok(machine)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_with_token1() {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(u8)]
        enum TokenKind {
            Null = 0,
        }

        impl From<TokenKind> for u8 {
            fn from(token: TokenKind) -> u8 {
                token as u8
            }
        }

        impl TokenKindTrait for TokenKind {}

        let mut machine = build_keyword_machine("null", TokenKind::Null).unwrap();

        machine.reset();
        assert!(machine.process_event('n').is_none());
        assert!(machine.process_event('u').is_none());
        assert!(machine.process_event('l').is_none());
        assert!(machine.process_event('l').is_none());
        assert!(machine.is_in_accepting_state());
        assert_eq!(machine.get_token_kind(), TokenKind::Null);

        machine.reset();
        assert!(machine.process_event('n').is_none());
        assert!(machine.process_event('u').is_none());
        assert!(machine.process_event('x').is_some());
        assert!(!machine.is_in_accepting_state());
    }

    #[test]
    fn test_with_token2() {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(u8)]
        enum TokenKind {
            String = 1,
        }

        impl From<TokenKind> for u8 {
            fn from(token: TokenKind) -> u8 {
                token as u8
            }
        }

        impl TokenKindTrait for TokenKind {}
        let mut machine = build_string_machine(TokenKind::String).unwrap();

        machine.reset();
        assert!(machine.process_event('"').is_none());
        assert!(machine.process_event('a').is_none());
        assert!(machine.process_event('b').is_none());
        assert!(machine.process_event('"').is_none());
        assert!(machine.is_in_accepting_state());
        assert_eq!(machine.get_token_kind(), TokenKind::String);

        machine.reset();
        assert!(machine.process_event('\'').is_none());
        assert!(machine.process_event('a').is_none());
        assert!(machine.process_event('b').is_none());
        assert!(machine.process_event('\'').is_none());
        assert!(machine.is_in_accepting_state());
        assert_eq!(machine.get_token_kind(), TokenKind::String);
    }

    #[test]
    fn test_multi() {
        #[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq)]
        #[repr(u8)]
        enum TokenKind {
            DoubleEqual = 0,
        }

        impl From<TokenKind> for u8 {
            fn from(token: TokenKind) -> u8 {
                token as u8
            }
        }

        impl TokenKindTrait for TokenKind {}

        let mut machine =
            build_multi_char_machine(TokenKind::DoubleEqual, "==".chars().collect()).unwrap();

        machine.reset();
        assert!(machine.process_event('=').is_none());
        assert!(machine.process_event('=').is_none());
        assert!(machine.is_in_accepting_state());
        assert!(machine.process_event('=').is_some());
    }

    #[test]
    fn test_integer() {
        #[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq)]
        #[repr(u8)]
        enum TokenKind {
            Integer = 0,
        }

        impl From<TokenKind> for u8 {
            fn from(token: TokenKind) -> u8 {
                token as u8
            }
        }

        impl TokenKindTrait for TokenKind {}

        let mut machine = build_integer_machine(TokenKind::Integer).unwrap();

        machine.reset();
        assert!(machine.process_event('0').is_none());
        assert!(machine.process_event('1').is_none());
        assert!(machine.process_event('2').is_none());
        assert!(machine.process_event('3').is_none());
        assert!(machine.process_event('4').is_none());
        assert!(machine.process_event('5').is_none());
        assert!(machine.process_event(' ').is_some());
        assert!(!machine.is_in_accepting_state());
    }
}
