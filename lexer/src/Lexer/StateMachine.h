// #pragma once
// #include <algorithm>
// #include <cassert>
// #include <functional>
// #include <map>
// #include <optional>
// #include <string>
// #include <string_view>
// #include <vector>
// #include "Lexer/Token.h"

// namespace Lexer {
// // 状态机相关定义
// namespace StateMachine {
// // 状态ID类型
// using StateId = size_t;

// // 特殊状态定义
// constexpr StateId InvalidState = 0;
// constexpr StateId StartState = 1;

// // 转换条件函数：判断输入是否满足转换条件
// using TransitionCondition = std::function<bool(char32_t)>;

// // 转换动作：当转换发生时执行的操作
// using TransitionAction = std::function<void(char32_t)>;

// // 状态转换
// struct Transition {
//   StateId target;                 // 目标状态
//   TransitionCondition condition;  // 转换条件
//   TransitionAction action;        // 转换动作(可选)
// };

// // 状态定义
// struct State {
//   StateId id;
//   bool is_accepting;                    // 是否为接受状态
//   TokenType token_type;                 // 接受状态对应的Token类型
//   std::vector<Transition> transitions;  // 状态转换列表
//   std::function<Token(StateId, size_t, size_t, std::string_view)>
//     token_builder;  // Token构建函数
// };

// // 状态机构建器
// class Builder {
//  private:
//   std::map<StateId, State> states;
//   StateId next_state_id{};

//  public:
//   Builder() = default;  // 从2开始，0和1是特殊状态

//   // 创建新状态
//   auto create_state(
//     bool is_accepting = false,
//     TokenType token_type = TokenType::InvalidToken
//   ) -> StateId {
//     StateId state_id = next_state_id++;
//     states[state_id] = {
//       .id = state_id,
//       .is_accepting = is_accepting,
//       .token_type = token_type,
//       .transitions = {},
//       .token_builder = {}
//     };
//     return state_id;
//   }

//   // 添加状态转换
//   void add_transition(
//     StateId from,
//     StateId to,
//     TransitionCondition condition,
//     TransitionAction action = {}
//   ) {
//     assert(states.contains(from) && "源状态不存在");
//     assert(states.contains(to) && "目标状态不存在");

//     states[from].transitions.push_back(
//       {.target = to,
//        .condition = std::move(condition),
//        .action = std::move(action)}
//     );
//   }

//   // 设置状态的Token构建函数
//   void set_token_builder(
//     StateId state,
//     std::function<Token(StateId, size_t, size_t, std::string_view)> builder
//   ) {
//     assert(states.contains(state) && "状态不存在");
//     states[state].token_builder = std::move(builder);
//   }

//   // 获取所有状态
//   [[nodiscard]] auto get_states() const -> const std::map<StateId, State>& {
//     return states;
//   }
// };
// }  // namespace StateMachine

// // 基于状态机的词法分析器
// class StateMachineLexer {
//  private:
//   std::string buffer;  // 输入缓冲区
//   size_t pos = 0;      // 当前字节位置(0-based)
//   size_t line = 1;     // 当前行号(1-based)
//   size_t column = 1;   // 当前列号(1-based)
//   std::map<StateMachine::StateId, StateMachine::State> states;  // 状态机状态

//   // 用于构建Token的临时数据
//   struct TokenBuildData {
//     size_t start_pos{};
//     size_t start_line{};
//     size_t start_column{};
//     std::string content;
//   };
//   TokenBuildData current_token_data;

//   // 重置当前Token构建数据
//   void reset_token_data() {
//     current_token_data = {
//       .start_pos = pos,
//       .start_line = line,
//       .start_column = column,
//       .content = ""
//     };
//   }

//   // 跳过Unicode空白字符
//   void skip_whitespace() {
//     while (pos < buffer.size()) {
//       auto result = Utf8Utils::get_utf8_codepoint(buffer, pos);
//       if (result.is_err()) {
//         break;  // 解码错误，停止跳过
//       }

//       auto [code_point, len] = result.unwrap();
//       if (Utf8Utils::is_unicode_whitespace(code_point)) {
//         if (code_point == U'\n') {  // 换行：更新行号，重置列号
//           line++;
//           column = 1;
//         } else {
//           column++;  // 其他空白：列号+1
//         }
//         pos += len;
//       } else {
//         break;  // 非空白，停止跳过
//       }
//     }
//   }

//   // 收缩缓冲区（避免内存膨胀）
//   void shrink_buffer() {
//     if (pos > buffer.size() / 2) {  // 已处理超过一半时收缩
//       buffer = buffer.substr(pos);
//       pos = 0;
//     }
//   }

//  public:
//   StateMachineLexer() = default;
//   explicit StateMachineLexer(StateMachine::Builder&& builder)
//     : states(std::move(builder).get_states()) {}

//   // 追加输入数据
//   void feed(std::string_view data) { buffer.append(data); }

//   // 设置状态机
//   void set_state_machine(StateMachine::Builder&& builder) {
//     states = std::move(builder).get_states();
//   }

//   // 获取下一个Token（nullopt表示需要更多输入）
//   auto next_token() -> std::optional<Token> {
//     skip_whitespace();  // 跳过空白
//     shrink_buffer();    // 收缩缓冲区

//     if (pos >= buffer.size()) {
//       // 检查是否真的结束
//       if (buffer.empty()) {
//         return Token{
//           .type = TokenType::Eof, .value = "", .line = line, .column = column
//         };
//       }
//       return std::nullopt;  // 缓冲区空，需要更多输入
//     }

//     reset_token_data();
//     StateMachine::StateId current_state = StateMachine::StartState;
//     StateMachine::StateId last_accepting_state = StateMachine::InvalidState;
//     size_t last_accepting_pos = pos;
//     size_t last_accepting_line = line;
//     size_t last_accepting_column = column;
//     std::string last_accepting_content;

//     while (true) {
//       // 检查当前状态是否是接受状态
//       auto it = states.find(current_state);
//       if (it != states.end() && it->second.is_accepting) {
//         last_accepting_state = current_state;
//         last_accepting_pos = pos;
//         last_accepting_line = line;
//         last_accepting_column = column;
//         last_accepting_content = current_token_data.content;
//       }

//       // 如果已经到达缓冲区末尾，退出循环
//       if (pos >= buffer.size()) {
//         break;
//       }

//       // 获取当前UTF-8码点
//       auto result = Utf8Utils::get_utf8_codepoint(buffer, pos);
//       if (result.is_err()) {
//         // UTF-8解码错误
//         Token err_token{
//           .type = TokenType::Utf8Error,
//           .value = result.unwrap_err(),
//           .line = line,
//           .column = column
//         };
//         pos++;  // 跳过错误字节
//         column++;
//         return err_token;
//       }

//       auto [code_point, len] = result.unwrap();

//       // 查找合适的转换
//       bool found_transition = false;
//       if (it != states.end()) {
//         for (const auto& transition : it->second.transitions) {
//           if (transition.condition(code_point)) {
//             // 执行转换动作
//             if (transition.action) {
//               transition.action(code_point);
//             }

//             // 更新当前Token内容
//             current_token_data.content.append(buffer.substr(pos, len));

//             // 更新位置信息
//             pos += len;
//             column++;

//             // 转换到目标状态
//             current_state = transition.target;
//             found_transition = true;
//             break;
//           }
//         }
//       }

//       if (!found_transition) {
//         break;
//       }
//     }

//     // 检查是否找到接受状态
//     if (last_accepting_state != StateMachine::InvalidState) {
//       // 回退到最后一个接受状态的位置
//       pos = last_accepting_pos;
//       line = last_accepting_line;
//       column = last_accepting_column;

//       // 构建Token
//       auto it = states.find(last_accepting_state);
//       if (it != states.end() && it->second.token_builder) {
//         return it->second.token_builder(
//           last_accepting_state, current_token_data.start_line,
//           current_token_data.start_column, last_accepting_content
//         );
//       }
//       // 默认的Token构建
//       return Token{
//         .type =
//           it != states.end() ? it->second.token_type : TokenType::InvalidToken,
//         .value = last_accepting_content,
//         .line = current_token_data.start_line,
//         .column = current_token_data.start_column
//       };
//     }

//     // 未找到任何接受状态，生成无效Token
//     auto result = Utf8Utils::get_utf8_codepoint(buffer, pos);
//     if (result.is_err()) {
//       Token err_token{
//         .type = TokenType::Utf8Error,
//         .value = result.unwrap_err(),
//         .line = line,
//         .column = column
//       };
//       pos++;
//       column++;
//       return err_token;
//     }

//     auto [code_point, len] = result.unwrap();
//     std::string invalid_str = buffer.substr(pos, len);
//     pos += len;
//     column++;

//     return Token{
//       .type = TokenType::InvalidToken,
//       .value = invalid_str,
//       .line = current_token_data.start_line,
//       .column = current_token_data.start_column
//     };
//   }

//   // 获取词法序列
//   auto tokenize() -> std::vector<Token> {
//     std::vector<Token> tokens;
//     while (true) {
//       auto token = next_token();
//       if (!token) {
//         // 如果需要更多输入但没有了，就退出
//         if (is_eof()) {
//           break;
//         }
//         continue;
//       }
//       tokens.push_back(*token);
//       if (token->type == TokenType::Eof) {
//         break;
//       }
//     }
//     return tokens;
//   }

//   // 判断是否处理完所有输入
//   [[nodiscard]] auto is_eof() const -> bool {
//     return pos >= buffer.size() && buffer.empty();
//   }

//   // 获取当前行列号
//   [[nodiscard]] std::pair<size_t, size_t> get_position() const {
//     return {line, column};
//   }
// };

// // 状态机构建辅助函数
// namespace LexerBuilders {
// // 构建关键字状态机
// void build_keyword_machine(
//   StateMachine::Builder& builder,
//   const std::vector<std::string>& keywords
// );

// // 构建标识符状态机
// void build_identifier_machine(StateMachine::Builder& builder);

// // 构建数字状态机（整数和浮点数）
// void build_number_machine(StateMachine::Builder& builder);

// // 构建字符串状态机
// void build_string_machine(StateMachine::Builder& builder);

// // 构建布尔值状态机
// void build_boolean_machine(StateMachine::Builder& builder);

// // 构建null状态机
// void build_null_machine(StateMachine::Builder& builder);

// // 构建运算符状态机
// void build_operator_machine(
//   StateMachine::Builder& builder,
//   const std::vector<std::string>& op3_list,
//   const std::vector<std::string>& op2_list,
//   const std::vector<char>& op1_list
// );

// // 构建注释状态机
// void build_comment_machine(StateMachine::Builder& builder);

// // 构建默认的完整状态机
// auto build_default_machine() -> StateMachine::Builder;
// }  // namespace LexerBuilders

// inline auto match_char(char32_t c) -> StateMachine::TransitionCondition {
//   return [c](char32_t input) { return input == c; };
// }

// inline auto match_any_of(const std::vector<char32_t>& chars)
//   -> StateMachine::TransitionCondition {
//   return [chars](char32_t input) {
//     return std::ranges::any_of(chars, [input](char32_t c) {
//       return input == c;
//     });
//   };
// }

// inline auto match_range(char32_t start, char32_t end)
//   -> StateMachine::TransitionCondition {
//   return
//     [start, end](char32_t input) { return input >= start && input <= end; };
// }
// }  // namespace Lexer