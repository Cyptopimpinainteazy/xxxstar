//! Tests for the Universal Contracts SDK.

#[cfg(test)]
mod tests {
    use crate::actions::{Action, Domain};
    use crate::compiler::Compiler;
    use crate::error::UcError;
    use crate::intents::IntentBuilder;
    use crate::sdk::UniversalContract;

    // ===== Action::validate() tests =====

    #[test]
    fn test_action_lock_valid() {
        let action = Action::Lock {
            asset_id: 1,
            amount: 1000,
            domain: Domain::X3Native,
        };
        assert!(action.validate().is_ok());
    }

    #[test]
    fn test_action_lock_zero_amount() {
        let action = Action::Lock {
            asset_id: 1,
            amount: 0,
            domain: Domain::X3Native,
        };
        assert_eq!(action.validate(), Err(UcError::ZeroAmount));
    }

    #[test]
    fn test_action_mint_valid() {
        let action = Action::Mint {
            asset_id: 2,
            amount: 500,
            domain: Domain::X3Evm,
        };
        assert!(action.validate().is_ok());
    }

    #[test]
    fn test_action_swap_valid() {
        let action = Action::Swap {
            asset_in: 1,
            asset_out: 2,
            amount_in: 1000,
            min_out: 500,
            domain: Domain::X3Native,
        };
        assert!(action.validate().is_ok());
    }

    #[test]
    fn test_action_swap_same_asset() {
        let action = Action::Swap {
            asset_in: 1,
            asset_out: 1,
            amount_in: 1000,
            min_out: 500,
            domain: Domain::X3Native,
        };
        assert_eq!(action.validate(), Err(UcError::SameAsset));
    }

    #[test]
    fn test_action_swap_zero_amount() {
        let action = Action::Swap {
            asset_in: 1,
            asset_out: 2,
            amount_in: 0,
            min_out: 500,
            domain: Domain::X3Native,
        };
        assert_eq!(action.validate(), Err(UcError::ZeroAmount));
    }

    // ===== Action::is_cross_vm() tests =====

    #[test]
    fn test_action_settle_is_cross_vm() {
        let action = Action::Settle {
            packet_id: [0u8; 32],
        };
        assert!(action.is_cross_vm());
    }

    #[test]
    fn test_action_lock_not_cross_vm() {
        let action = Action::Lock {
            asset_id: 1,
            amount: 1000,
            domain: Domain::X3Native,
        };
        assert!(!action.is_cross_vm());
    }

    #[test]
    fn test_action_abort_is_terminal() {
        let action = Action::Abort { reason: [0u8; 32] };
        assert!(action.is_terminal());
    }

    #[test]
    fn test_action_lock_not_terminal() {
        let action = Action::Lock {
            asset_id: 1,
            amount: 1000,
            domain: Domain::X3Native,
        };
        assert!(!action.is_terminal());
    }

    // ===== Action::commitment() tests =====

    #[test]
    fn test_action_commitment_deterministic() {
        let action1 = Action::Lock {
            asset_id: 1,
            amount: 1000,
            domain: Domain::X3Native,
        };
        let action2 = Action::Lock {
            asset_id: 1,
            amount: 1000,
            domain: Domain::X3Native,
        };
        assert_eq!(action1.commitment(), action2.commitment());
    }

    #[test]
    fn test_action_commitment_different_for_different_actions() {
        let action1 = Action::Lock {
            asset_id: 1,
            amount: 1000,
            domain: Domain::X3Native,
        };
        let action2 = Action::Lock {
            asset_id: 2,
            amount: 1000,
            domain: Domain::X3Native,
        };
        assert_ne!(action1.commitment(), action2.commitment());
    }

    // ===== Compiler::compile() tests =====

    #[test]
    fn test_compiler_empty_actions_list() {
        let result = Compiler::compile(&[]);
        assert!(result.is_err());
        assert!(matches!(result, Err(UcError::EmptyActionList)));
    }

    #[test]
    fn test_compiler_single_lock() {
        let actions = vec![Action::Lock {
            asset_id: 1,
            amount: 1000,
            domain: Domain::X3Native,
        }];
        let result = Compiler::compile(&actions);
        assert!(result.is_ok());
        let bundle = result.unwrap();
        assert_eq!(bundle.len(), 1);
        assert!(!bundle.is_empty());
    }

    #[test]
    fn test_compiler_lock_mint_sequence() {
        let actions = vec![
            Action::Lock {
                asset_id: 1,
                amount: 1000,
                domain: Domain::X3Native,
            },
            Action::Mint {
                asset_id: 2,
                amount: 1000,
                domain: Domain::X3Evm,
            },
        ];
        let result = Compiler::compile(&actions);
        assert!(result.is_ok());
        let bundle = result.unwrap();
        assert_eq!(bundle.len(), 2);
    }

    #[test]
    fn test_compiler_abort_not_last() {
        let actions = vec![
            Action::Abort { reason: [0u8; 32] },
            Action::Lock {
                asset_id: 1,
                amount: 1000,
                domain: Domain::X3Native,
            },
        ];
        let result = Compiler::compile(&actions);
        assert!(result.is_err());
        assert!(matches!(result, Err(UcError::AbortNotLast)));
    }

    #[test]
    fn test_compiler_abort_last_ok() {
        let actions = vec![
            Action::Lock {
                asset_id: 1,
                amount: 1000,
                domain: Domain::X3Native,
            },
            Action::Abort { reason: [0u8; 32] },
        ];
        let result = Compiler::compile(&actions);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_compiler_invalid_action_fails() {
        let actions = vec![Action::Lock {
            asset_id: 1,
            amount: 0,
            domain: Domain::X3Native,
        }];
        let result = Compiler::compile(&actions);
        assert!(result.is_err());
        assert!(matches!(result, Err(UcError::ZeroAmount)));
    }

    #[test]
    fn test_compiler_program_hash_deterministic() {
        let actions = vec![Action::Lock {
            asset_id: 1,
            amount: 1000,
            domain: Domain::X3Native,
        }];
        let bundle1 = Compiler::compile(&actions).unwrap();
        let bundle2 = Compiler::compile(&actions).unwrap();
        assert_eq!(bundle1.program_hash, bundle2.program_hash);
    }

    // ===== IntentBuilder tests =====

    #[test]
    fn test_intent_builder_default_values() {
        let builder = IntentBuilder::new([1u128], [0u8; 32], [0u8; 32]);
        let intent = builder.build();
        assert!(intent.is_ok());
    }

    #[test]
    fn test_intent_builder_zero_fee_cap() {
        let builder = IntentBuilder::new([1u128], [0u8; 32], [0u8; 32]).fee_cap(0);
        let intent = builder.build();
        assert!(intent.is_err());
        assert!(matches!(intent, Err(UcError::ZeroFeeCap)));
    }

    #[test]
    fn test_intent_builder_nonzero_fee_cap() {
        let builder = IntentBuilder::new([1u128], [0u8; 32], [0u8; 32]).fee_cap(1000);
        let intent = builder.build();
        assert!(intent.is_ok());
    }

    #[test]
    fn test_intent_builder_bond() {
        let builder = IntentBuilder::new([1u128], [0u8; 32], [0u8; 32]).bond(500);
        let intent = builder.build();
        assert!(intent.is_ok());
    }

    #[test]
    fn test_intent_builder_slashable() {
        let builder = IntentBuilder::new([1u128], [0u8; 32], [0u8; 32])
            .slashable()
            .bond(100); // Slashable requires non-zero bond
        let intent = builder.build();
        assert!(intent.is_ok());
    }

    // ===== UniversalContract tests =====

    #[test]
    fn test_universal_contract_single_action() {
        let contract = UniversalContract::new([0u8; 32])
            .fee_cap(1000)
            .submitted_at(10)
            .action(Action::Lock {
                asset_id: 1,
                amount: 500,
                domain: Domain::X3Native,
            });
        let result = contract.compile();
        assert!(result.is_ok());
    }

    #[test]
    fn test_universal_contract_multiple_actions() {
        let contract = UniversalContract::new([0u8; 32])
            .fee_cap(2000)
            .submitted_at(20)
            .action(Action::Lock {
                asset_id: 1,
                amount: 500,
                domain: Domain::X3Native,
            })
            .action(Action::Mint {
                asset_id: 2,
                amount: 500,
                domain: Domain::X3Evm,
            });
        let result = contract.compile();
        assert!(result.is_ok());
    }

    #[test]
    fn test_universal_contract_to_packet_no_cross_vm() {
        let contract = UniversalContract::new([0u8; 32])
            .fee_cap(1000)
            .submitted_at(10)
            .action(Action::Lock {
                asset_id: 1,
                amount: 500,
                domain: Domain::X3Native,
            });
        let compiled = contract.compile().unwrap();
        let packet = compiled.to_packet([0u8; 32], 1, 100);
        assert_eq!(
            packet, None,
            "packet should be None for non-cross-VM actions"
        );
    }

    #[test]
    fn test_universal_contract_to_packet_with_cross_vm() {
        let contract = UniversalContract::new([0u8; 32])
            .fee_cap(1000)
            .submitted_at(10)
            .action(Action::Lock {
                asset_id: 1,
                amount: 500,
                domain: Domain::X3Native,
            })
            .action(Action::Settle {
                packet_id: [1u8; 32],
            });
        let compiled = contract.compile().unwrap();
        let packet = compiled.to_packet([0u8; 32], 1, 100);
        assert!(
            packet.is_some(),
            "packet should be Some for cross-VM actions"
        );
    }
}
