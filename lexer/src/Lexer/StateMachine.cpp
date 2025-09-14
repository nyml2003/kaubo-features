#include "Lexer/StateMachine.h"

namespace Lexer {

// 构建关键字状态机
void LexerBuilders::build_keyword_machine(
  StateMachine::Builder& builder,
  const std::vector<std::string>& keywords
) {
  // 为每个关键字创建状态路径
  for (const std::string& keyword : keywords) {
    StateMachine::StateId current_state = StateMachine::StartState;

    // 为关键字的每个字符创建状态转换
    for (size_t i = 0; i < keyword.size(); ++i) {
      char c = keyword[i];
      bool is_last_char = (i == keyword.size() - 1);

      // 创建新状态（最后一个字符的状态是接受状态）
      StateMachine::StateId next_state =
        builder.create_state(is_last_char, TokenType::Keyword);

      // 添加转换：当前状态 -> 下一个状态，匹配当前字符
      builder.add_transition(
        current_state, next_state, match_char(static_cast<char32_t>(c))
      );

      current_state = next_state;
    }

    // 设置关键字Token构建函数
    builder.set_token_builder(
      current_state,
      [keyword](
        StateMachine::StateId, size_t line, size_t column, std::string_view
      ) {
        return Token{
          .type = TokenType::Keyword,
          .value = keyword,
          .line = line,
          .column = column
        };
      }
    );
  }
}

// 构建标识符状态机
void LexerBuilders::build_identifier_machine(StateMachine::Builder& builder) {
  // 创建标识符状态
  StateMachine::StateId identifier_state =
    builder.create_state(true, TokenType::Identifier);

  // 从开始状态到标识符状态：匹配标识符起始字符
  builder.add_transition(
    StateMachine::StartState, identifier_state,
    [](char32_t c) { return Utf8Utils::is_identifier_start(c); }
  );

  // 标识符状态自循环：匹配标识符后续字符
  builder.add_transition(identifier_state, identifier_state, [](char32_t c) {
    return Utf8Utils::is_identifier_part(c);
  });

  // 设置标识符Token构建函数
  builder.set_token_builder(
    identifier_state,
    [](
      StateMachine::StateId, size_t line, size_t column,
      std::string_view content
    ) {
      return Token{
        .type = TokenType::Identifier,
        .value = std::string(content),
        .line = line,
        .column = column
      };
    }
  );
}

// 构建数字状态机（整数和浮点数）
void LexerBuilders::build_number_machine(StateMachine::Builder& builder) {
  // 创建数字状态
  StateMachine::StateId integer_state =
    builder.create_state(true, TokenType::Integer);
  StateMachine::StateId dot_state =
    builder.create_state(false, TokenType::InvalidToken);
  StateMachine::StateId float_state =
    builder.create_state(true, TokenType::Float);

  // 从开始状态到整数状态：匹配数字
  builder.add_transition(
    StateMachine::StartState, integer_state, match_range(U'0', U'9')
  );

  // 整数状态自循环：匹配更多数字
  builder.add_transition(integer_state, integer_state, match_range(U'0', U'9'));

  // 整数状态到点状态：匹配小数点
  builder.add_transition(integer_state, dot_state, match_char(U'.'));

  // 点状态到浮点数状态：匹配小数点后的数字
  builder.add_transition(dot_state, float_state, match_range(U'0', U'9'));

  // 浮点数状态自循环：匹配更多数字
  builder.add_transition(float_state, float_state, match_range(U'0', U'9'));

  // 设置整数Token构建函数
  builder.set_token_builder(
    integer_state,
    [](
      StateMachine::StateId, size_t line, size_t column,
      std::string_view content
    ) {
      return Token{
        .type = TokenType::Integer,
        .value = std::string(content),
        .line = line,
        .column = column
      };
    }
  );

  // 设置浮点数Token构建函数
  builder.set_token_builder(
    float_state,
    [](
      StateMachine::StateId, size_t line, size_t column,
      std::string_view content
    ) {
      return Token{
        .type = TokenType::Float,
        .value = std::string(content),
        .line = line,
        .column = column
      };
    }
  );
}

// 构建字符串状态机
void LexerBuilders::build_string_machine(StateMachine::Builder& builder) {
  // 创建字符串状态
  StateMachine::StateId string_start_state =
    builder.create_state(false, TokenType::InvalidToken);
  StateMachine::StateId string_content_state =
    builder.create_state(false, TokenType::InvalidToken);
  StateMachine::StateId string_escape_state =
    builder.create_state(false, TokenType::InvalidToken);
  StateMachine::StateId string_end_state =
    builder.create_state(true, TokenType::String);

  // 从开始状态到字符串开始状态：匹配双引号
  builder.add_transition(
    StateMachine::StartState, string_start_state, match_char(U'"')
  );

  // 字符串开始状态到内容状态：匹配任意字符（除了双引号和反斜杠）
  builder.add_transition(
    string_start_state, string_content_state,
    [](char32_t c) { return c != U'"' && c != U'\\'; }
  );

  // 内容状态自循环：匹配任意字符（除了双引号和反斜杠）
  builder.add_transition(
    string_content_state, string_content_state,
    [](char32_t c) { return c != U'"' && c != U'\\'; }
  );

  // 内容状态到转义状态：匹配反斜杠
  builder.add_transition(
    string_content_state, string_escape_state, match_char(U'\\')
  );

  // 转义状态到内容状态：匹配任意字符（转义字符）
  builder.add_transition(
    string_escape_state, string_content_state, [](char32_t) { return true; }
  );

  // 内容状态到结束状态：匹配双引号
  builder.add_transition(
    string_content_state, string_end_state, match_char(U'"')
  );

  // 设置字符串Token构建函数
  builder.set_token_builder(
    string_end_state,
    [](
      StateMachine::StateId, size_t line, size_t column,
      std::string_view content
    ) {
      // 移除首尾的引号
      std::string str_content = std::string(content);
      if (str_content.size() >= 2) {
        str_content = str_content.substr(1, str_content.size() - 2);
      }
      return Token{
        .type = TokenType::String,
        .value = str_content,
        .line = line,
        .column = column
      };
    }
  );
}

// 构建布尔值状态机
void LexerBuilders::build_boolean_machine(StateMachine::Builder& builder) {
  // "true"的状态路径
  StateMachine::StateId t_state = builder.create_state(false);
  StateMachine::StateId tr_state = builder.create_state(false);
  StateMachine::StateId tru_state = builder.create_state(false);
  StateMachine::StateId true_state =
    builder.create_state(true, TokenType::Boolean);

  // "false"的状态路径
  StateMachine::StateId f_state = builder.create_state(false);
  StateMachine::StateId fa_state = builder.create_state(false);
  StateMachine::StateId fal_state = builder.create_state(false);
  StateMachine::StateId fals_state = builder.create_state(false);
  StateMachine::StateId false_state =
    builder.create_state(true, TokenType::Boolean);

  // 构建"true"路径
  builder.add_transition(StateMachine::StartState, t_state, match_char(U't'));
  builder.add_transition(t_state, tr_state, match_char(U'r'));
  builder.add_transition(tr_state, tru_state, match_char(U'u'));
  builder.add_transition(tru_state, true_state, match_char(U'e'));

  // 构建"false"路径
  builder.add_transition(StateMachine::StartState, f_state, match_char(U'f'));
  builder.add_transition(f_state, fa_state, match_char(U'a'));
  builder.add_transition(fa_state, fal_state, match_char(U'l'));
  builder.add_transition(fal_state, fals_state, match_char(U's'));
  builder.add_transition(fals_state, false_state, match_char(U'e'));

  // 设置布尔值Token构建函数
  builder.set_token_builder(
    true_state,
    [](StateMachine::StateId, size_t line, size_t column, std::string_view) {
      return Token{
        .type = TokenType::Boolean,
        .value = "true",
        .line = line,
        .column = column
      };
    }
  );

  builder.set_token_builder(
    false_state,
    [](StateMachine::StateId, size_t line, size_t column, std::string_view) {
      return Token{
        .type = TokenType::Boolean,
        .value = "false",
        .line = line,
        .column = column
      };
    }
  );
}

// 构建null状态机
void LexerBuilders::build_null_machine(StateMachine::Builder& builder) {
  // 创建"null"的状态路径
  StateMachine::StateId n_state = builder.create_state(false);
  StateMachine::StateId nu_state = builder.create_state(false);
  StateMachine::StateId nul_state = builder.create_state(false);
  StateMachine::StateId null_state =
    builder.create_state(true, TokenType::Null);

  // 构建状态转换
  builder.add_transition(StateMachine::StartState, n_state, match_char(U'n'));
  builder.add_transition(n_state, nu_state, match_char(U'u'));
  builder.add_transition(nu_state, nul_state, match_char(U'l'));
  builder.add_transition(nul_state, null_state, match_char(U'l'));

  // 设置null Token构建函数
  builder.set_token_builder(
    null_state,
    [](StateMachine::StateId, size_t line, size_t column, std::string_view) {
      return Token{
        .type = TokenType::Null, .value = "null", .line = line, .column = column
      };
    }
  );
}

// 构建运算符状态机
void LexerBuilders::build_operator_machine(
  StateMachine::Builder& builder,
  const std::vector<std::string>& op3_list,
  const std::vector<std::string>& op2_list,
  const std::vector<char>& op1_list
) {
  // 先处理三字符运算符（优先级最高）
  for (const std::string& op : op3_list) {
    if (op.size() != 3) {
      continue;
    }

    StateMachine::StateId s1 = builder.create_state(false);
    StateMachine::StateId s2 = builder.create_state(false);
    StateMachine::StateId s3 = builder.create_state(true, TokenType::Operator3);

    builder.add_transition(
      StateMachine::StartState, s1, match_char(static_cast<char32_t>(op[0]))
    );
    builder.add_transition(s1, s2, match_char(static_cast<char32_t>(op[1])));
    builder.add_transition(s2, s3, match_char(static_cast<char32_t>(op[2])));

    builder.set_token_builder(
      s3,
      [op](
        StateMachine::StateId, size_t line, size_t column, std::string_view
      ) {
        return Token{
          .type = TokenType::Operator3,
          .value = op,
          .line = line,
          .column = column
        };
      }
    );
  }

  // 再处理双字符运算符
  for (const std::string& op : op2_list) {
    if (op.size() != 2) {
      continue;
    }

    StateMachine::StateId s1 = builder.create_state(false);
    StateMachine::StateId s2 = builder.create_state(true, TokenType::Operator2);

    builder.add_transition(
      StateMachine::StartState, s1, match_char(static_cast<char32_t>(op[0]))
    );
    builder.add_transition(s1, s2, match_char(static_cast<char32_t>(op[1])));

    builder.set_token_builder(
      s2,
      [op](
        StateMachine::StateId, size_t line, size_t column, std::string_view
      ) {
        return Token{
          .type = TokenType::Operator2,
          .value = op,
          .line = line,
          .column = column
        };
      }
    );
  }

  // 最后处理单字符运算符
  for (char c : op1_list) {
    StateMachine::StateId s = builder.create_state(true, TokenType::Operator1);

    builder.add_transition(
      StateMachine::StartState, s, match_char(static_cast<char32_t>(c))
    );

    builder.set_token_builder(
      s,
      [c](StateMachine::StateId, size_t line, size_t column, std::string_view) {
        return Token{
          .type = TokenType::Operator1,
          .value = std::string(1, c),
          .line = line,
          .column = column
        };
      }
    );
  }
}

// 构建注释状态机
void LexerBuilders::build_comment_machine(StateMachine::Builder& builder) {
  // 单行注释：// ...
  StateMachine::StateId line_comment_slash1 = builder.create_state(false);
  StateMachine::StateId line_comment_state =
    builder.create_state(true, TokenType::InvalidToken);  // 不生成Token

  // 多行注释：/* ... */
  StateMachine::StateId block_comment_slash1 = builder.create_state(false);
  StateMachine::StateId block_comment_star = builder.create_state(false);
  StateMachine::StateId block_comment_content = builder.create_state(false);
  StateMachine::StateId block_comment_end_star = builder.create_state(false);
  StateMachine::StateId block_comment_end =
    builder.create_state(true, TokenType::InvalidToken);  // 不生成Token

  // 构建单行注释状态转换
  builder.add_transition(
    StateMachine::StartState, line_comment_slash1, match_char(U'/')
  );
  builder.add_transition(
    line_comment_slash1, line_comment_state, match_char(U'/')
  );
  builder.add_transition(
    line_comment_state, line_comment_state,
    [](char32_t c) { return c != U'\n'; }
  );  // 匹配除换行外的所有字符

  // 构建多行注释状态转换
  builder.add_transition(
    StateMachine::StartState, block_comment_slash1, match_char(U'/')
  );
  builder.add_transition(
    block_comment_slash1, block_comment_star, match_char(U'*')
  );
  builder.add_transition(
    block_comment_star, block_comment_content,
    [](char32_t c) { return c != U'*'; }
  );
  builder.add_transition(
    block_comment_content, block_comment_content,
    [](char32_t c) { return c != U'*'; }
  );
  builder.add_transition(
    block_comment_content, block_comment_end_star, match_char(U'*')
  );
  builder.add_transition(
    block_comment_end_star, block_comment_end_star, match_char(U'*')
  );
  builder.add_transition(
    block_comment_end_star, block_comment_content,
    [](char32_t c) { return c != U'/'; }
  );
  builder.add_transition(
    block_comment_end_star, block_comment_end, match_char(U'/')
  );

  // 注释不生成Token
  auto no_token_builder =
    [](StateMachine::StateId, size_t, size_t, std::string_view) -> Token {
    return Token{
      .type = TokenType::InvalidToken, .value = "", .line = 0, .column = 0
    };
  };

  builder.set_token_builder(line_comment_state, no_token_builder);
  builder.set_token_builder(block_comment_end, no_token_builder);
}

// 构建默认的完整状态机
StateMachine::Builder LexerBuilders::build_default_machine() {
  StateMachine::Builder builder;

  // 注册注释（优先级最高）
  build_comment_machine(builder);

  // 注册关键字
  const std::vector<std::string> keywords = {"if",     "else",     "for",
                                             "while",  "return",   "function",
                                             "var",    "let",      "const",
                                             "fn",     "template", "typename",
                                             "friend", "auto",     "throw"};
  build_keyword_machine(builder, keywords);

  // 注册常量
  build_boolean_machine(builder);
  build_null_machine(builder);

  // 注册字符串和数字
  build_string_machine(builder);
  build_number_machine(builder);

  // 注册运算符
  const std::vector<std::string> op3_list = {"===", "!=="};
  const std::vector<std::string> op2_list = {">=", "<=", "==", "!=", "&&",
                                             "||", "++", "--", "+=", "-=",
                                             "*=", "/=", "->", "::"};
  const std::vector<char> op1_list = {'+', '-', '*', '/', '%', '>', '<', '!',
                                      '&', '|', '^', '~', '?', '.', ',', '(',
                                      ')', '[', ']', '{', '}', ';', ':', '='};
  build_operator_machine(builder, op3_list, op2_list, op1_list);

  // 注册标识符（优先级最低）
  build_identifier_machine(builder);

  return builder;
}
}  // namespace Lexer