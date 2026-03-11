use crate::passes;
use crate::{BasicBlock, BinOp, ConstValue, Function, Instruction, Terminator, UnOp};

mod cfg_and_dominators {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn computes_dominators_for_simple_diamond() {
        let func = Function {
            id: "f".to_string(),
            params: Vec::new(),
            return_type: None,
            is_async: false,
            body: vec![
                BasicBlock {
                    id: 0,
                    instructions: vec![],
                    terminator: Terminator::Branch {
                        cond: 0,
                        then: 1,
                        else_: 2,
                    },
                },
                BasicBlock {
                    id: 1,
                    instructions: vec![],
                    terminator: Terminator::Jump(2),
                },
                BasicBlock {
                    id: 2,
                    instructions: vec![],
                    terminator: Terminator::Return(None),
                },
            ],
        };

        let dom = passes::dominators(&func);
        assert_eq!(dom.get(&0).unwrap(), &HashSet::from([0]));
        assert_eq!(dom.get(&1).unwrap(), &HashSet::from([0, 1]));
        assert_eq!(dom.get(&2).unwrap(), &HashSet::from([0, 2]));
    }
}

mod ssa_construction {
    use super::*;
    use crate::passes::ssa;

    #[test]
    fn inserts_phi_for_variable_defined_in_two_predecessors() {
        let func = Function {
            id: "f".to_string(),
            params: Vec::new(),
            return_type: None,
            is_async: false,
            body: vec![
                BasicBlock {
                    id: 0,
                    instructions: vec![Instruction::Const {
                        dest: 0,
                        value: ConstValue::Bool(true),
                    }],
                    terminator: Terminator::Branch {
                        cond: 0,
                        then: 1,
                        else_: 2,
                    },
                },
                BasicBlock {
                    id: 1,
                    instructions: vec![
                        Instruction::Const {
                            dest: 1,
                            value: ConstValue::Number(1.0),
                        },
                        Instruction::AssignVar {
                            name: "x".to_string(),
                            src: 1,
                        },
                    ],
                    terminator: Terminator::Jump(3),
                },
                BasicBlock {
                    id: 2,
                    instructions: vec![
                        Instruction::Const {
                            dest: 2,
                            value: ConstValue::Number(2.0),
                        },
                        Instruction::AssignVar {
                            name: "x".to_string(),
                            src: 2,
                        },
                    ],
                    terminator: Terminator::Jump(3),
                },
                BasicBlock {
                    id: 3,
                    instructions: vec![Instruction::VarRef {
                        dest: 3,
                        name: "x".to_string(),
                    }],
                    terminator: Terminator::Return(Some(3)),
                },
            ],
        };

        let ssa = ssa::build(&func).unwrap();
        let join = ssa.blocks.get(&3).unwrap();
        assert!(join.phis.iter().any(|p| p.var == "x"));

        let phi = join.phis.iter().find(|p| p.var == "x").unwrap();
        assert_eq!(phi.sources, vec![(1, 1), (2, 2)]);

        // The VarRef in block 3 should read the phi result.
        assert_eq!(ssa.var_reads.get(&3).copied(), Some(phi.dest));
    }
}

mod constant_folding {
    use super::*;

    #[test]
    fn folds_simple_arithmetic() {
        let mut func = Function {
            id: "f".to_string(),
            params: Vec::new(),
            return_type: None,
            is_async: false,
            body: vec![BasicBlock {
                id: 0,
                instructions: vec![
                    Instruction::Const {
                        dest: 0,
                        value: ConstValue::Number(1.0),
                    },
                    Instruction::Const {
                        dest: 1,
                        value: ConstValue::Number(2.0),
                    },
                    Instruction::BinOp {
                        op: BinOp::Add,
                        lhs: 0,
                        rhs: 1,
                        dest: 2,
                    },
                ],
                terminator: Terminator::Return(Some(2)),
            }],
        };

        let stats = passes::constant_fold_function(&mut func);
        assert_eq!(stats.folded, 1);

        let block = &func.body[0];
        assert!(block
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::Const { dest: 2, value: ConstValue::Number(n) } if (*n - 3.0).abs() < 1e-9)));
        assert!(!block
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::BinOp { dest: 2, .. })));
    }

    #[test]
    fn folds_unary_not() {
        let mut func = Function {
            id: "f".to_string(),
            params: Vec::new(),
            return_type: None,
            is_async: false,
            body: vec![BasicBlock {
                id: 0,
                instructions: vec![
                    Instruction::Const {
                        dest: 0,
                        value: ConstValue::Bool(true),
                    },
                    Instruction::UnOp {
                        op: UnOp::Not,
                        arg: 0,
                        dest: 1,
                    },
                ],
                terminator: Terminator::Return(Some(1)),
            }],
        };

        let stats = passes::constant_fold_function(&mut func);
        assert_eq!(stats.folded, 1);
        assert!(func.body[0]
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::Const { dest: 1, value: ConstValue::Bool(false) })));
    }
}

mod local_dce {
    use super::*;

    #[test]
    fn removes_dead_pure_value() {
        let mut func = Function {
            id: "f".to_string(),
            params: Vec::new(),
            return_type: None,
            is_async: false,
            body: vec![BasicBlock {
                id: 0,
                instructions: vec![
                    Instruction::Const {
                        dest: 0,
                        value: ConstValue::Number(1.0),
                    },
                    // dead: never used
                    Instruction::Const {
                        dest: 99,
                        value: ConstValue::Number(123.0),
                    },
                ],
                terminator: Terminator::Return(Some(0)),
            }],
        };

        let stats = passes::local_dce_function(&mut func);
        assert_eq!(stats.removed, 1);
        assert!(!func.body[0]
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::Const { dest: 99, .. })));
    }
}
