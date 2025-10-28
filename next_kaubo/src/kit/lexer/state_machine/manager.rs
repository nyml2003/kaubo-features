use super::machine::Machine;
use super::types::{Event, TokenKindTrait};

type MachineId = usize;

pub struct Manager<TokenKind>
where
    TokenKind: TokenKindTrait,
{
    machines: Vec<MachineInfo<TokenKind>>,
}

#[derive(Debug, PartialEq, Eq)]
enum MachineStatus {
    Active,
    Inactive,
    Completed,
    Selectable,
}

struct MachineInfo<TokenKind>
where
    TokenKind: TokenKindTrait,
{
    machine: Machine<TokenKind>,
    match_length: usize,
    status: MachineStatus,
}

impl<TokenKind> Manager<TokenKind>
where
    TokenKind: TokenKindTrait,
{
    pub fn new() -> Self {
        Self {
            machines: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        for info in &mut self.machines {
            info.machine.reset();
            info.match_length = 0;
            info.status = MachineStatus::Active;
        }
    }

    pub fn add_machine(&mut self, machine: Machine<TokenKind>) {
        self.machines.push(MachineInfo {
            machine,
            match_length: 0,
            status: MachineStatus::Active,
        });
    }

    pub fn process_event(&mut self, event: Event) -> bool {
        let mut any_completed = false;
        for machine_info in &mut self.machines {
            if machine_info.status == MachineStatus::Inactive {
                continue;
            }
            if machine_info.machine.process_event(event).is_some() {
                if machine_info.status == MachineStatus::Completed {
                    machine_info.status = MachineStatus::Selectable;
                    continue;
                }
                machine_info.status = MachineStatus::Inactive;
                continue;
            }
            machine_info.match_length += 1;
            if machine_info.machine.is_in_accepting_state() {
                any_completed = true;
                machine_info.status = MachineStatus::Completed;
            }
        }
        any_completed
    }

    pub fn select_best_match(&self) -> (Option<MachineId>, usize) {
        let mut best_id: Option<MachineId> = None;
        let mut max_length = 0;
        let mut max_priority = None;

        for (id, info) in self.machines.iter().enumerate() {
            if info.status != MachineStatus::Selectable {
                continue;
            }
            let current_priority = info.machine.get_token_kind();

            // 优先选择最长匹配
            if info.match_length > max_length {
                max_length = info.match_length;
                best_id = Some(id);
                max_priority = Some(current_priority);
            }
            // 长度相同则选择优先级更高的（假设T的Ord实现中，较小的值代表较高优先级）
            else if info.match_length == max_length {
                if let Some(mp) = max_priority.as_ref() {
                    if current_priority < *mp {
                        best_id = Some(id);
                        max_priority = Some(current_priority);
                    }
                }
            }
        }
        (best_id, max_length)
    }

    pub fn get_machine_token_kind_by_index(&self, index: usize) -> Option<TokenKind> {
        self.machines
            .get(index)
            .map(|info| info.machine.get_token_kind())
    }
}

#[cfg(test)]
mod tests {
    use crate::kit::lexer::state_machine::{
        builder::{build_multi_char_machine, build_single_char_machine},
        manager,
    };

    use super::*;

    #[test]
    fn test_manager() {
        #[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq)]
        #[repr(u8)]
        enum TokenKind {
            Equal = 1,
            DoubleEqual = 0,
        }

        impl From<TokenKind> for u8 {
            fn from(token: TokenKind) -> u8 {
                token as u8
            }
        }

        impl TokenKindTrait for TokenKind {}
        let mut manager = manager::Manager::new();
        manager.add_machine(build_single_char_machine(TokenKind::Equal, '=').unwrap());
        manager.add_machine(
            build_multi_char_machine(TokenKind::DoubleEqual, "==".chars().collect()).unwrap(),
        );
    }

    #[test]
    fn test_with_token3() {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(u8)]
        enum TokenKind {
            DoubleEqual = 100,      // ==
            LessThanEqual = 101,    // <=
            GreaterThanEqual = 102, // >=
            NotEqual = 103,         // !=

            LessThan = 200,    // <
            Equal = 201,       // =
            GreaterThan = 202, // >
        }

        impl From<TokenKind> for u8 {
            fn from(token: TokenKind) -> u8 {
                token as u8
            }
        }

        impl TokenKindTrait for TokenKind {}

        // 批量注册
        // let mut machine = build_multi_char_machine(TokenKind::DoubleEqual, vec!['=', '=']).unwrap();
        let mut manager = manager::Manager::new();
        for (kind, operator) in vec![
            (TokenKind::DoubleEqual, "=="),
            (TokenKind::LessThanEqual, "<="),
            (TokenKind::GreaterThanEqual, ">="),
            (TokenKind::NotEqual, "!="),
        ] {
            manager
                .add_machine(build_multi_char_machine(kind, operator.chars().collect()).unwrap());
        }

        for (kind, operator) in vec![
            (TokenKind::LessThan, '<'),
            (TokenKind::Equal, '='),
            (TokenKind::GreaterThan, '>'),
        ] {
            manager.add_machine(build_single_char_machine(kind, operator).unwrap());
        }

        // let input = "== = > >= <= > < ".chars().collect::<Vec<char>>();
        // for c in input.iter() {
        //     let any_completed = manager.process_event(*c);
        //     for (id, info) in manager.machines.iter().enumerate() {
        //         println!(
        //             "id: {:?}, status: {:?}, match_length: {:?}, token_kind:{:?}",
        //             id,
        //             info.status,
        //             info.match_length,
        //             info.machine.get_token_type()
        //         );
        //     }
        //     println!("any_completed: {:?}\n", any_completed);
        //     if !any_completed {
        //         let (id, length) = manager.select_best_match();
        //         println!("id: {:?}, length: {}", id, length);
        //         if let Some(id) = id {
        //             println!(
        //                 "Matched token: {:?}, length: {}, input: {:?}",
        //                 manager.machines[id].machine.get_token_type(),
        //                 length,
        //                 &input[..length]
        //             );
        //         }
        //     }
        // }
        // let mut last_index = 0;
        // for c in input.iter() {
        //     let any_completed = manager.process_event(*c);
        //     if !any_completed {
        //         let (id, length) = manager.select_best_match();
        //         if let Some(id) = id {
        //             println!(
        //                 "Matched token: {:?}, length: {}, input: {:?}",
        //                 manager.machines[id].machine.get_token_type(),
        //                 length,
        //                 &input[last_index..last_index + length]
        //             );
        //             last_index += length + 1;
        //         }
        //         manager.reset();
        //     }
        // }
        let mut any_completed = false;
        let mut best_machine_id = None;
        let mut best_machine: Option<&MachineInfo<TokenKind>> = None;
        let mut best_match_length = 0;
        any_completed = manager.process_event('=');
        assert!(any_completed);
        any_completed = manager.process_event('=');
        assert!(any_completed);
        any_completed = manager.process_event(' ');
        assert!(!any_completed);
        (best_machine_id, best_match_length) = manager.select_best_match();
        best_machine = manager.machines.get(best_machine_id.unwrap());
        assert_eq!(
            best_machine.unwrap().machine.get_token_kind(),
            TokenKind::DoubleEqual
        );
        assert_eq!(best_match_length, 2);
        manager.reset();
        any_completed = manager.process_event('>');
        assert!(any_completed);
        any_completed = manager.process_event('=');
        assert!(any_completed);
        any_completed = manager.process_event(' ');
        assert!(!any_completed);
        (best_machine_id, best_match_length) = manager.select_best_match();
        best_machine = manager.machines.get(best_machine_id.unwrap());
        assert_eq!(
            best_machine.unwrap().machine.get_token_kind(),
            TokenKind::GreaterThanEqual
        );
        assert_eq!(best_match_length, 2);
        manager.reset();
    }
}
