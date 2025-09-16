#pragma once
#include <cassert>
#include <functional>
#include <unordered_map>
#include <utility>
#include <vector>
#include "Lexer/TokenType.h"

namespace Lexer {

/**
 * @brief 通用状态机框架
 * 纯状态机逻辑，与具体任务无关，只负责状态管理和转移
 */
class StateMachine {
 public:
  using StateId = size_t;  // 状态ID类型
  using Event = char;      // 事件类型（输入字符）
  using TransitionCondition = std::function<bool(Event)>;     // 转移条件
  using StateCallback = std::function<void(StateId, Event)>;  // 状态回调

 private:
  // 状态信息结构
  struct State {
    StateId id;
    bool is_accepting;
    StateCallback on_enter;  // 进入状态时调用
    StateCallback on_exit;   // 退出状态时调用
  };

  // 转移规则结构
  struct Transition {
    StateId from;
    StateId to;
    TransitionCondition condition;
  };

  StateId next_state_id = 0;
  std::unordered_map<StateId, State> states;
  using TransitionMap = std::unordered_map<StateId, std::vector<Transition>>;
  TransitionMap transitions;
  StateId current_state;
  StateId initial_state;
  Lexer::TokenType token_type;

 public:
  /**
   * @brief 构造函数
   * @param initial_state_name 初始状态名称
   */
  explicit StateMachine(Lexer::TokenType token_type) : token_type(token_type) {
    initial_state = add_state(false);
    current_state = initial_state;
  }

  /**
   * @brief 添加新状态
   * @param is_accepting 是否为接受状态
   * @param on_enter 进入状态回调
   * @param on_exit 退出状态回调
   * @return 新状态ID
   */
  auto add_state(
    bool is_accepting,
    StateCallback on_enter = nullptr,
    StateCallback on_exit = nullptr
  ) -> StateId {
    StateId id = next_state_id++;
    states[id] = {
      .id = id,
      .is_accepting = is_accepting,
      .on_enter = std::move(on_enter),
      .on_exit = std::move(on_exit)
    };
    return id;
  }

  /**
   * @brief 添加状态转移规则
   * @param from 源状态ID
   * @param to 目标状态ID
   * @param condition 转移条件
   */
  void add_transition(StateId from, StateId to, TransitionCondition condition) {
    assert(states.contains(from) && "源状态不存在");
    assert(states.contains(to) && "目标状态不存在");
    // 向当前源状态的转移列表中添加规则
    transitions[from].emplace_back(
      Transition{.from = from, .to = to, .condition = std::move(condition)}
    );
  }

  /**
   * @brief 处理事件，进行状态转移
   * @param event 事件（输入字符）
   * @return 是否发生了状态转移
   */
  auto process_event(Event event) -> bool {
    // 1. 只获取当前状态对应的转移规则（无规则则直接返回false）
    auto trans_it = transitions.find(current_state);
    if (trans_it == transitions.end()) {
      return false;
    }

    // 2. 遍历当前状态的转移规则（t 远小于原有的 n）
    const auto& current_transitions = trans_it->second;
    auto rule_it =
      std::ranges::find_if(current_transitions, [event](const Transition& t) {
        return t.condition(event);
      });

    if (rule_it != current_transitions.end()) {
      // 触发回调并转移状态（逻辑不变）
      if (states[current_state].on_exit) {
        states[current_state].on_exit(current_state, event);
      }
      current_state = rule_it->to;
      if (states[current_state].on_enter) {
        states[current_state].on_enter(current_state, event);
      }
      return true;
    }
    return false;
  }

  /**
   * @brief 重置状态机到初始状态
   */
  void reset() { current_state = initial_state; }

  /**
   * @brief 获取当前状态ID
   */
  [[nodiscard]] auto get_current_state() const -> StateId {
    return current_state;
  }

  /**
   * @brief 检查当前状态是否为接受状态
   */
  [[nodiscard]] auto is_in_accepting_state() const -> bool {
    auto it = states.find(current_state);
    return (it != states.end()) ? it->second.is_accepting : false;
  }

  [[nodiscard]] auto get_token_type() const -> Lexer::TokenType {
    return token_type;
  }
};
}  // namespace Lexer