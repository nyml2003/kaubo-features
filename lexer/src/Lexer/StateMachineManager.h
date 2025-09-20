#pragma once

#include "Lexer/StateMachine.h"
#include "Lexer/TokenType.h"

#include <memory>

namespace Lexer {

/**
 * @brief 多状态机管理器
 * 管理多个并行运行的状态机，支持按“最长匹配+优先级”规则选择最佳匹配结果
 */
template <TokenTypeConstraint TokenType>
class StateMachineManager {
 public:
  using MachineId = size_t;                      // 状态机ID类型
  using Event = StateMachine<TokenType>::Event;  // 事件类型（与状态机一致）
  using MatchResult = std::pair<
    std::weak_ptr<StateMachine<TokenType>>,
    size_t>;  // 匹配结果（状态机ID+匹配长度）

 private:
  // 状态机信息结构（包装状态机及运行时信息）
  struct MachineInfo {
    std::shared_ptr<StateMachine<TokenType>> machine;  // 状态机实例
    size_t match_length;                               // 当前匹配长度
    bool is_active;                                    // 是否仍能继续处理事件
    bool has_accepted;                                 // 是否曾进入接受状态
  };

  MachineId next_machine_id = 0;
  std::unordered_map<MachineId, MachineInfo> machines;  // 管理的状态机集合
  std::vector<MachineId> active_machines;               // 活跃状态机ID缓存

 public:
  StateMachineManager() = default;

  /**
   * @brief 添加状态机
   * @param machine 状态机实例（所有权转移）
   * @param priority 优先级（值越高，相同匹配长度时优先被选择）
   * @return 状态机ID
   */
  auto add_machine(std::unique_ptr<StateMachine<TokenType>> machine)
    -> MachineId {
    assert(machine != nullptr && "状态机实例不能为空");
    MachineId id = next_machine_id++;
    machines[id] = {
      .machine = std::move(machine),
      .match_length = 0,
      .is_active = true,
      .has_accepted = false
    };
    active_machines.push_back(id);
    return id;
  }

  /**
   * @brief 处理事件，驱动所有活跃状态机并行推进
   * @param event 输入事件（字符）
   * @return 是否有至少一个状态机成功处理事件
   */
  auto process_event(Event event) -> bool {
    bool any_active = false;
    std::vector<MachineId> new_active_machines;

    for (MachineId id : active_machines) {
      auto& info = machines.at(id);
      if (!info.is_active) {
        continue;
      }

      // 让状态机处理事件
      bool processed = info.machine->process_event(event);

      if (processed) {
        // 处理成功：更新匹配长度和接受状态
        info.match_length++;
        if (info.machine->is_in_accepting_state()) {
          info.has_accepted = true;
        }
        new_active_machines.push_back(id);
        any_active = true;
      } else {
        // 处理失败：标记为非活跃
        info.is_active = false;
      }
    }

    active_machines.swap(new_active_machines);
    return any_active;
  }

  /**
   * @brief 选择最佳匹配结果（最长匹配+优先级）
   * @return 最佳匹配的状态机ID和匹配长度（若无可匹配项，返回{invalid, 0}）
   */
  [[nodiscard]] auto select_best_match() const -> MatchResult {
    auto best_id = static_cast<MachineId>(-1);
    size_t max_length = 0;
    auto max_priority =
      static_cast<TokenType>(std::numeric_limits<uint8_t>::max());
    ;

    for (const auto& [id, info] : machines) {
      // 只考虑曾进入接受状态的状态机
      if (!info.has_accepted) {
        continue;
      }

      // 优先选择最长匹配
      if (info.match_length > max_length) {
        max_length = info.match_length;
        best_id = id;
        max_priority = info.machine->get_token_type();
      }
      // 长度相同则选择优先级更高的
      else if (info.match_length == max_length &&
               info.machine->get_token_type() < max_priority) {
        best_id = id;
        max_priority = info.machine->get_token_type();
      }
    }

    if (best_id == static_cast<MachineId>(-1)) {
      return {std::weak_ptr<StateMachine<TokenType>>(), max_length};
    }
    return {machines.at(best_id).machine, max_length};
  }

  /**
   * @brief 重置所有状态机到初始状态
   */
  void reset() {
    active_machines.clear();
    for (auto& [id, info] : machines) {
      info.machine->reset();
      info.match_length = 0;
      info.is_active = true;
      info.has_accepted = false;
      active_machines.push_back(id);
    }
  }

  /**
   * @brief 检查是否有活跃状态机
   */
  [[nodiscard]] auto has_active_machines() const -> bool {
    return !active_machines.empty();
  }
};
}  // namespace Lexer