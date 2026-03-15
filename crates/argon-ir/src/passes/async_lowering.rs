//! Async Lowering Pass
//!
//! Transforms async functions into state machine structs with poll functions.
//! Requires CFG flattening to have run first (no nested control flow).
//!
//! Algorithm:
//! 1. Find all Await instructions in the flattened CFG
//! 2. Compute which named variables are live across each await point
//! 3. Generate enum variants (one per await point + Start + Done)
//! 4. Generate a poll function that matches on the current state
//! 5. Replace the original async function with a wrapper

use crate::*;
use std::collections::HashSet;

/// A detected await point in the function body.
#[derive(Debug, Clone)]
pub struct AwaitPoint {
    pub block_id: BlockId,
    pub instruction_index: usize,
    pub arg: ValueId,
    pub dest: ValueId,
}

/// Identify all await instructions and their positions.
pub fn find_await_points(func: &Function) -> Vec<AwaitPoint> {
    let mut points = Vec::new();
    for block in &func.body {
        for (idx, inst) in block.instructions.iter().enumerate() {
            if let Instruction::Await { arg, dest } = inst {
                points.push(AwaitPoint {
                    block_id: block.id,
                    instruction_index: idx,
                    arg: *arg,
                    dest: *dest,
                });
            }
        }
    }
    points
}

/// Compute which named variables are live across each await point.
/// A variable is "live across" an await if it is defined before the await
/// and used after the await.
pub fn compute_live_across_awaits(
    func: &Function,
    await_points: &[AwaitPoint],
) -> Vec<HashSet<String>> {
    let mut result = Vec::new();

    for point in await_points {
        let mut defined_before: HashSet<String> = HashSet::new();
        let mut used_after: HashSet<String> = HashSet::new();

        // Include function parameters as defined
        for param in &func.params {
            defined_before.insert(param.name.clone());
        }

        // Walk all instructions, collecting defs before and uses after the await
        let mut past_await = false;
        for block in &func.body {
            for (idx, inst) in block.instructions.iter().enumerate() {
                if block.id == point.block_id && idx == point.instruction_index {
                    past_await = true;
                    continue;
                }
                if !past_await {
                    collect_defs(inst, &mut defined_before);
                } else {
                    collect_uses(inst, &mut used_after);
                }
            }
        }

        // Live across = defined before AND used after
        let live: HashSet<String> = defined_before.intersection(&used_after).cloned().collect();
        result.push(live);
    }

    result
}

fn collect_defs(inst: &Instruction, defs: &mut HashSet<String>) {
    match inst {
        Instruction::VarDecl { name, .. } => {
            defs.insert(name.clone());
        }
        Instruction::AssignVar { name, .. } => {
            defs.insert(name.clone());
        }
        Instruction::AssignExpr { name, .. } => {
            defs.insert(name.clone());
        }
        _ => {}
    }
}

fn collect_uses(inst: &Instruction, uses: &mut HashSet<String>) {
    match inst {
        Instruction::VarRef { name, .. } => {
            uses.insert(name.clone());
        }
        Instruction::AssignVar { .. } => {
            // Variable redefined. We track uses via VarRef.
        }
        _ => {}
    }
}

/// Result of transforming an async function into a state machine.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

/// Transform an async function into (wrapper, poll_fn, state_enum).
pub fn transform_async_function(func: &Function) -> (Function, Function, EnumDef) {
    let await_points = find_await_points(func);
    let live_sets = compute_live_across_awaits(func, &await_points);
    let enum_name = format!("{}_State", capitalize(&func.id));

    // Build enum variants
    let mut variants = Vec::new();

    // Start variant: carries function parameters
    variants.push(EnumVariant {
        name: "Start".to_string(),
        fields: func
            .params
            .iter()
            .map(|p| (p.name.clone(), Some(p.ty)))
            .collect(),
    });

    // One variant per await point (carries live variables + the future being awaited)
    for (i, (_point, live)) in await_points.iter().zip(live_sets.iter()).enumerate() {
        let mut fields: Vec<(String, Option<TypeId>)> =
            live.iter().map(|name| (name.clone(), None)).collect();
        fields.push((format!("__future_{}", i), None));
        variants.push(EnumVariant {
            name: format!("Awaiting{}", i),
            fields,
        });
    }

    // Done variant
    variants.push(EnumVariant {
        name: "Done".to_string(),
        fields: vec![("__result".to_string(), func.return_type)],
    });

    let state_enum = EnumDef {
        name: enum_name.clone(),
        variants: variants.clone(),
    };

    // Build poll function (skeleton — body generation is future work)
    let poll_fn = Function {
        id: format!("{}_poll", func.id),
        params: vec![
            Param {
                name: "state".to_string(),
                ty: 0,
            },
            Param {
                name: "waker".to_string(),
                ty: 0,
            },
        ],
        return_type: func.return_type,
        is_async: false,
        body: vec![BasicBlock {
            id: 0,
            instructions: vec![],
            terminator: Terminator::Return(None),
        }],
    };

    // Build wrapper function (replaces original async fn)
    let wrapper = Function {
        id: func.id.clone(),
        params: func.params.clone(),
        return_type: func.return_type,
        is_async: false,
        body: vec![BasicBlock {
            id: 0,
            instructions: vec![],
            terminator: Terminator::Return(None),
        }],
    };

    (wrapper, poll_fn, state_enum)
}

/// Transform all async functions in a module into state machines.
/// Call this before codegen for non-JS targets.
pub fn lower_async_module(module: &mut Module) {
    let async_fns: Vec<Function> = module
        .functions
        .iter()
        .filter(|f| f.is_async)
        .cloned()
        .collect();

    for func in &async_fns {
        let (wrapper, poll_fn, state_enum) = transform_async_function(func);

        // Replace the original function with the wrapper
        if let Some(pos) = module.functions.iter().position(|f| f.id == func.id) {
            module.functions[pos] = wrapper;
        }

        // Add poll function
        module.functions.push(poll_fn);

        // Add state enum type
        module.types.push(TypeDef::Enum {
            name: state_enum.name,
            variants: state_enum.variants,
        });
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_async_fn() -> Function {
        Function {
            id: "f".to_string(),
            params: vec![Param {
                name: "url".to_string(),
                ty: 0,
            }],
            return_type: Some(0),
            is_async: true,
            body: vec![BasicBlock {
                id: 0,
                instructions: vec![
                    Instruction::VarRef {
                        dest: 0,
                        name: "url".to_string(),
                    },
                    Instruction::VarRef {
                        dest: 10,
                        name: "get".to_string(),
                    },
                    Instruction::Call {
                        callee: 10,
                        args: vec![0],
                        dest: 1,
                    },
                    Instruction::Await { arg: 1, dest: 2 },
                    Instruction::Member {
                        object: 2,
                        property: "body".to_string(),
                        dest: 3,
                    },
                ],
                terminator: Terminator::Return(Some(3)),
            }],
        }
    }

    #[test]
    fn finds_await_split_points() {
        let func = make_simple_async_fn();
        let splits = find_await_points(&func);
        assert_eq!(splits.len(), 1, "should find 1 await point");
        assert_eq!(splits[0].arg, 1);
        assert_eq!(splits[0].dest, 2);
    }

    #[test]
    fn computes_liveness_across_await() {
        let func = make_simple_async_fn();
        let points = find_await_points(&func);
        let live = compute_live_across_awaits(&func, &points);
        assert_eq!(live.len(), 1);
        // "url" is defined as param but not used after the await via VarRef
        // (it's used before the await). So the live set should be empty.
        assert!(
            live[0].is_empty(),
            "expected empty live set, got {:?}",
            live[0]
        );
    }

    #[test]
    fn liveness_captures_variable_used_after_await() {
        // async function g(x):
        //   let y = x + 1
        //   await something()
        //   return y   <-- y is live across the await
        let func = Function {
            id: "g".to_string(),
            params: vec![Param {
                name: "x".to_string(),
                ty: 0,
            }],
            return_type: Some(0),
            is_async: true,
            body: vec![BasicBlock {
                id: 0,
                instructions: vec![
                    Instruction::VarDecl {
                        kind: VarKind::Let,
                        name: "y".to_string(),
                        init: Some(0),
                    },
                    Instruction::Await { arg: 1, dest: 2 },
                    Instruction::VarRef {
                        dest: 3,
                        name: "y".to_string(),
                    },
                ],
                terminator: Terminator::Return(Some(3)),
            }],
        };

        let points = find_await_points(&func);
        let live = compute_live_across_awaits(&func, &points);
        assert_eq!(live.len(), 1);
        assert!(
            live[0].contains("y"),
            "y should be live across the await, got {:?}",
            live[0]
        );
    }

    #[test]
    fn transforms_simple_async_to_state_machine() {
        let func = make_simple_async_fn();
        let (wrapper, poll_fn, state_enum) = transform_async_function(&func);

        assert!(!wrapper.is_async);
        assert_eq!(wrapper.id, "f");

        assert_eq!(poll_fn.id, "f_poll");
        assert!(!poll_fn.is_async);

        // Start + Awaiting0 + Done = 3 variants
        assert_eq!(state_enum.variants.len(), 3);
        assert_eq!(state_enum.variants[0].name, "Start");
        assert_eq!(state_enum.variants[1].name, "Awaiting0");
        assert_eq!(state_enum.variants[2].name, "Done");

        // Start variant should carry the "url" parameter
        assert!(state_enum.variants[0]
            .fields
            .iter()
            .any(|(name, _)| name == "url"));
    }

    #[test]
    fn lower_async_module_replaces_functions() {
        let mut module = Module {
            functions: vec![make_simple_async_fn()],
            types: vec![],
            globals: vec![],
            imports: vec![],
            exports: vec![],
        };

        lower_async_module(&mut module);

        // Original async fn should be replaced with sync wrapper + poll fn added
        assert_eq!(module.functions.len(), 2);
        assert!(!module.functions[0].is_async, "wrapper should be sync");
        assert_eq!(module.functions[0].id, "f");
        assert_eq!(module.functions[1].id, "f_poll");

        // State enum should be in types
        assert_eq!(module.types.len(), 1);
        assert!(matches!(&module.types[0], TypeDef::Enum { name, .. } if name == "F_State"));
    }
}
