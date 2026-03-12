//! SSA construction for the current IR.
//!
//! This pass is currently used for analysis/testing only; it is not wired into codegen.
//! It works over named variables (VarDecl/AssignVar/AssignExpr/VarRef) and emits Phi nodes
//! at dominance frontiers plus a mapping from `VarRef.dest` -> SSA `ValueId`.

use crate::passes::{build_cfg, dominators};
use crate::{BlockId, Function, Instruction, Terminator, ValueId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct PhiNode {
    pub var: String,
    pub dest: ValueId,
    pub sources: Vec<(BlockId, ValueId)>,
}

#[derive(Debug, Clone)]
pub struct SsaBlock {
    pub id: BlockId,
    pub phis: Vec<PhiNode>,
}

#[derive(Debug, Clone)]
pub struct SsaFunction {
    pub entry: BlockId,
    pub blocks: HashMap<BlockId, SsaBlock>,
    // Map `VarRef.dest` to the SSA value representing the variable at that program point.
    pub var_reads: HashMap<ValueId, ValueId>,
    pub idom: HashMap<BlockId, BlockId>,
}

#[derive(Debug)]
pub enum SsaError {
    Unsupported(String),
    UninitializedVar(String),
}

impl std::fmt::Display for SsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SsaError::Unsupported(s) => write!(f, "Unsupported SSA: {}", s),
            SsaError::UninitializedVar(v) => write!(f, "Uninitialized var: {}", v),
        }
    }
}

impl std::error::Error for SsaError {}

pub fn build(func: &Function) -> Result<SsaFunction, SsaError> {
    let cfg = build_cfg(func);
    let dom = dominators(func);
    let (idom, dom_tree) = compute_idoms(&cfg.blocks, cfg.entry, &dom);
    let df = dominance_frontier(
        &cfg.blocks,
        cfg.entry,
        &cfg.preds,
        &cfg.succs,
        &idom,
        &dom_tree,
    );

    let mut next_value = max_value_id(func) + 1;

    // Collect variable def blocks.
    let mut def_blocks: HashMap<String, HashSet<BlockId>> = HashMap::new();
    for b in &func.body {
        for inst in &b.instructions {
            match inst {
                Instruction::VarDecl { name, init, .. } => {
                    if init.is_some() {
                        def_blocks.entry(name.clone()).or_default().insert(b.id);
                    }
                }
                Instruction::AssignVar { name, .. } => {
                    def_blocks.entry(name.clone()).or_default().insert(b.id);
                }
                Instruction::AssignExpr { name, .. } => {
                    def_blocks.entry(name.clone()).or_default().insert(b.id);
                }
                Instruction::Try { .. } => {
                    return Err(SsaError::Unsupported(
                        "SSA over try/catch/finally is not supported yet".to_string(),
                    ));
                }
                _ => {}
            }
        }
    }

    // Place Phi nodes via iterated dominance frontier.
    let mut phi_vars: HashMap<BlockId, HashSet<String>> = HashMap::new();
    for (var, defs) in &def_blocks {
        let mut work: Vec<BlockId> = defs.iter().copied().collect();
        let mut visited: HashSet<BlockId> = HashSet::new();

        while let Some(x) = work.pop() {
            if !visited.insert(x) {
                continue;
            }
            for y in df.get(&x).cloned().unwrap_or_default() {
                let set = phi_vars.entry(y).or_default();
                if set.insert(var.clone()) {
                    if !defs.contains(&y) {
                        work.push(y);
                    }
                }
            }
        }
    }

    // Materialize SSA blocks and Phi dests.
    let mut blocks: HashMap<BlockId, SsaBlock> = HashMap::new();
    for b in &cfg.blocks {
        let mut phis = Vec::new();
        if let Some(vars) = phi_vars.get(b) {
            let mut vars: Vec<String> = vars.iter().cloned().collect();
            vars.sort();
            for var in vars {
                phis.push(PhiNode {
                    var,
                    dest: {
                        let v = next_value;
                        next_value += 1;
                        v
                    },
                    sources: Vec::new(),
                });
            }
        }
        blocks.insert(*b, SsaBlock { id: *b, phis });
    }

    // Rename variables along dominator tree.
    let mut stacks: HashMap<String, Vec<ValueId>> = HashMap::new();
    let mut var_reads: HashMap<ValueId, ValueId> = HashMap::new();
    let mut block_env_out: HashMap<BlockId, HashMap<String, ValueId>> = HashMap::new();

    rename_block(
        func,
        cfg.entry,
        &cfg.succs,
        &dom_tree,
        &mut blocks,
        &mut stacks,
        &mut var_reads,
        &mut block_env_out,
    )?;

    // Fill phi sources using predecessor environments.
    for b in &cfg.blocks {
        let preds = cfg.preds.get(b).cloned().unwrap_or_default();
        if preds.is_empty() {
            continue;
        }
        let block = blocks.get_mut(b).unwrap();
        for phi in &mut block.phis {
            phi.sources.clear();
            for p in &preds {
                let env = block_env_out.get(p).cloned().unwrap_or_default();
                let v = env.get(&phi.var).copied().ok_or_else(|| {
                    SsaError::UninitializedVar(format!(
                        "{} (needed for phi in block {})",
                        phi.var, b
                    ))
                })?;
                phi.sources.push((*p, v));
            }
            phi.sources.sort_by_key(|(bid, _)| *bid);
        }
    }

    Ok(SsaFunction {
        entry: cfg.entry,
        blocks,
        var_reads,
        idom,
    })
}

fn rename_block(
    func: &Function,
    b: BlockId,
    succs: &HashMap<BlockId, Vec<BlockId>>,
    dom_tree: &HashMap<BlockId, Vec<BlockId>>,
    blocks: &mut HashMap<BlockId, SsaBlock>,
    stacks: &mut HashMap<String, Vec<ValueId>>,
    var_reads: &mut HashMap<ValueId, ValueId>,
    block_env_out: &mut HashMap<BlockId, HashMap<String, ValueId>>,
) -> Result<(), SsaError> {
    let mut pushed: Vec<String> = Vec::new();

    // Push Phi defs first.
    if let Some(block) = blocks.get(&b) {
        for phi in &block.phis {
            stacks.entry(phi.var.clone()).or_default().push(phi.dest);
            pushed.push(phi.var.clone());
        }
    }

    let ir_block = func
        .body
        .iter()
        .find(|bb| bb.id == b)
        .ok_or_else(|| SsaError::Unsupported("missing block".to_string()))?;

    for inst in &ir_block.instructions {
        match inst {
            Instruction::VarRef { dest, name } => {
                let cur = stacks
                    .get(name)
                    .and_then(|s| s.last())
                    .copied()
                    .ok_or_else(|| SsaError::UninitializedVar(name.clone()))?;
                var_reads.insert(*dest, cur);
            }
            Instruction::VarDecl { name, init, .. } => {
                let v = init.ok_or_else(|| {
                    SsaError::Unsupported(format!("var decl without initializer: {}", name))
                })?;
                stacks.entry(name.clone()).or_default().push(v);
                pushed.push(name.clone());
            }
            Instruction::AssignVar { name, src } => {
                stacks.entry(name.clone()).or_default().push(*src);
                pushed.push(name.clone());
            }
            Instruction::AssignExpr { name, dest, .. } => {
                stacks.entry(name.clone()).or_default().push(*dest);
                pushed.push(name.clone());
            }
            Instruction::Try { .. } => {
                return Err(SsaError::Unsupported(
                    "SSA over try/catch/finally is not supported yet".to_string(),
                ));
            }
            _ => {}
        }
    }

    // Capture outgoing environment for phi source selection.
    let mut env: HashMap<String, ValueId> = HashMap::new();
    for (k, v) in stacks.iter() {
        if let Some(top) = v.last() {
            env.insert(k.clone(), *top);
        }
    }
    block_env_out.insert(b, env);

    // Recurse.
    for child in dom_tree.get(&b).cloned().unwrap_or_default() {
        rename_block(
            func,
            child,
            succs,
            dom_tree,
            blocks,
            stacks,
            var_reads,
            block_env_out,
        )?;
    }

    // Pop local defs.
    for var in pushed.into_iter().rev() {
        if let Some(s) = stacks.get_mut(&var) {
            s.pop();
            if s.is_empty() {
                stacks.remove(&var);
            }
        }
    }

    Ok(())
}

fn max_value_id(func: &Function) -> ValueId {
    let mut max_v = 0usize;
    for b in &func.body {
        for inst in &b.instructions {
            for v in inst_values(inst) {
                max_v = max_v.max(v);
            }
        }
        for v in term_values(&b.terminator) {
            max_v = max_v.max(v);
        }
    }
    max_v
}

fn inst_values(inst: &Instruction) -> Vec<ValueId> {
    match inst {
        Instruction::Load { dest, src } => vec![*dest, *src],
        Instruction::Store { dest, src } => vec![*dest, *src],
        Instruction::ObjectLit { dest, props } => {
            let mut out = vec![*dest];
            out.extend(props.iter().map(|p| p.value));
            out
        }
        Instruction::New { callee, args, dest } => {
            let mut out = vec![*dest, *callee];
            out.extend(args.iter().copied());
            out
        }
        Instruction::Await { arg, dest } => vec![*dest, *arg],
        Instruction::VarDecl { init, .. } => init.iter().copied().collect(),
        Instruction::AssignVar { src, .. } => vec![*src],
        Instruction::AssignExpr { src, dest, .. } => vec![*src, *dest],
        Instruction::ThrowStmt { arg } => vec![*arg],
        Instruction::Try { .. } => Vec::new(),
        Instruction::ExprStmt { value } => vec![*value],
        Instruction::VarRef { dest, .. } => vec![*dest],
        Instruction::Member { object, dest, .. } => vec![*object, *dest],
        Instruction::MemberComputed {
            object,
            property,
            dest,
        } => vec![*object, *property, *dest],
        Instruction::BinOp { lhs, rhs, dest, .. } => vec![*lhs, *rhs, *dest],
        Instruction::UnOp { arg, dest, .. } => vec![*arg, *dest],
        Instruction::Call { callee, args, dest } => {
            let mut out = vec![*callee, *dest];
            out.extend(args.iter().copied());
            out
        }
        Instruction::ArrayLit { dest, elements } => {
            let mut out = vec![*dest];
            out.extend(elements.iter().flatten().copied());
            out
        }
        Instruction::LogicalOp { lhs, rhs, dest, .. } => vec![*lhs, *rhs, *dest],
        Instruction::Conditional {
            cond,
            then_value,
            else_value,
            dest,
        } => vec![*cond, *then_value, *else_value, *dest],
        Instruction::Const { dest, .. } => vec![*dest],
    }
}

fn term_values(term: &Terminator) -> Vec<ValueId> {
    match term {
        Terminator::Return(Some(v)) => vec![*v],
        Terminator::Return(None) => Vec::new(),
        Terminator::Branch { cond, .. } => vec![*cond],
        Terminator::Jump(_) => Vec::new(),
        Terminator::Unreachable => Vec::new(),
    }
}

fn compute_idoms(
    blocks: &[BlockId],
    entry: BlockId,
    dom: &HashMap<BlockId, HashSet<BlockId>>,
) -> (HashMap<BlockId, BlockId>, HashMap<BlockId, Vec<BlockId>>) {
    // Immediate dominator from dominator sets:
    // idom(b) is the strict dominator of b that doesn't dominate any other strict dominator of b.
    let mut idom: HashMap<BlockId, BlockId> = HashMap::new();
    idom.insert(entry, entry);

    for &b in blocks {
        if b == entry {
            continue;
        }
        let ds = dom.get(&b).cloned().unwrap_or_default();
        let strict: Vec<BlockId> = ds.into_iter().filter(|d| *d != b).collect();
        let mut chosen: Option<BlockId> = None;
        for &d in &strict {
            let mut is_idom = true;
            for &other in &strict {
                if other == d {
                    continue;
                }
                if dom.get(&other).map(|s| s.contains(&d)).unwrap_or(false) {
                    is_idom = false;
                    break;
                }
            }
            if is_idom {
                chosen = Some(d);
                break;
            }
        }
        if let Some(c) = chosen {
            idom.insert(b, c);
        }
    }

    let mut tree: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for &b in blocks {
        tree.entry(b).or_default();
    }
    for (&b, &p) in &idom {
        if b == entry {
            continue;
        }
        tree.entry(p).or_default().push(b);
    }
    for v in tree.values_mut() {
        v.sort();
    }
    (idom, tree)
}

fn dominance_frontier(
    blocks: &[BlockId],
    entry: BlockId,
    preds: &HashMap<BlockId, Vec<BlockId>>,
    succs: &HashMap<BlockId, Vec<BlockId>>,
    idom: &HashMap<BlockId, BlockId>,
    dom_tree: &HashMap<BlockId, Vec<BlockId>>,
) -> HashMap<BlockId, HashSet<BlockId>> {
    let mut df: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
    for &b in blocks {
        df.insert(b, HashSet::new());
    }

    for &b in blocks {
        let s = succs.get(&b).cloned().unwrap_or_default();
        for y in s {
            if idom.get(&y).copied().unwrap_or(entry) != b {
                df.entry(b).or_default().insert(y);
            }
        }
    }

    // Post-order traversal of dominator tree.
    let mut post: Vec<BlockId> = Vec::new();
    dom_tree_postorder(entry, dom_tree, &mut post);
    for b in post {
        for child in dom_tree.get(&b).cloned().unwrap_or_default() {
            let child_df = df.get(&child).cloned().unwrap_or_default();
            for y in child_df {
                if idom.get(&y).copied().unwrap_or(entry) != b {
                    df.entry(b).or_default().insert(y);
                }
            }
        }

        // Also handle join blocks with multiple preds (classic DF def).
        let ps = preds.get(&b).cloned().unwrap_or_default();
        if ps.len() >= 2 {
            for p in ps {
                if idom.get(&b).copied().unwrap_or(entry) != p {
                    df.entry(p).or_default().insert(b);
                }
            }
        }
    }

    df
}

fn dom_tree_postorder(
    node: BlockId,
    tree: &HashMap<BlockId, Vec<BlockId>>,
    out: &mut Vec<BlockId>,
) {
    for child in tree.get(&node).cloned().unwrap_or_default() {
        dom_tree_postorder(child, tree, out);
    }
    out.push(node);
}
