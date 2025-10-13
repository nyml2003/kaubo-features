// 测试泛化后的状态机
#[cfg(test)]
mod tests {
    use crate::kit::lexer::state_machine::builder::{build_keyword_machine, build_string_machine};
    #[test]
    fn test_with_token1() {
        /// 示例Token枚举1
        #[derive(Debug, Clone, PartialEq)]
        enum Token1 {
            Null,
        }

        // 使用Token1作为令牌类型
        let mut machine = build_keyword_machine("null", Token1::Null).unwrap();

        // 测试有效输入
        machine.reset();
        assert!(machine.process_event('n').is_none());
        assert!(machine.process_event('u').is_none());
        assert!(machine.process_event('l').is_none());
        assert!(machine.process_event('l').is_none());
        assert!(machine.is_in_accepting_state());
        assert_eq!(machine.get_token_type(), Token1::Null);

        // 测试无效输入
        machine.reset();
        assert!(machine.process_event('n').is_none());
        assert!(machine.process_event('u').is_none());
        assert!(machine.process_event('x').is_some()); // 无效字符
        assert!(!machine.is_in_accepting_state());
    }

    #[test]
    fn test_with_token2() {
        #[derive(Debug, Clone, PartialEq)]
        enum Token2 {
            String,
        }
        let mut machine = build_string_machine(Token2::String).unwrap();

        machine.reset();
        assert!(machine.process_event('"').is_none());
        assert!(machine.process_event('a').is_none());
        assert!(machine.process_event('b').is_none());
        assert!(machine.process_event('"').is_none());
        assert!(machine.is_in_accepting_state());
        assert_eq!(machine.get_token_type(), Token2::String);

        machine.reset();
        assert!(machine.process_event('\'').is_none());
        assert!(machine.process_event('a').is_none());
        assert!(machine.process_event('b').is_none());
        assert!(machine.process_event('\'').is_none());
        assert!(machine.is_in_accepting_state());
        assert_eq!(machine.get_token_type(), Token2::String);
    }
}
