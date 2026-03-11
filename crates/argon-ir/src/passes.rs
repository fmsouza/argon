//! IR analysis and optimization passes (early Phase 4 scaffolding).

use crate::{BasicBlock, BinOp, BlockId, ConstValue, Function, Instruction, Terminator, UnOp, ValueId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Cfg {
    pub entry: BlockId,
    pub preds: HashMap<BlockId, Vec<BlockId>>,
    pub succs: HashMap<BlockId, Vec<BlockId>>,
    pub blocks: Vec<BlockId>,
}

pub fn build_cfg(func: &Function) -> Cfg {
    let entry = func
        .body
        .first()
        .map(|b| b.id)
        .unwrap_or(0);

    let mut preds: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    let mut succs: HashMap<BlockId, Vec<BlockId>> = HashMap::new();

    let mut blocks = Vec::new();
    for block in &func.body {
        blocks.push(block.id);
        preds.entry(block.id).or_default();
        succs.entry(block.id).or_default();
    }

    for block in &func.body {
        let s = terminator_succs(&block.terminator);
        succs.insert(block.id, s.clone());
        for to in s {
            preds.entry(to).or_default().push(block.id);
        }
    }

    Cfg {
        entry,
        preds,
        succs,
        blocks,
    }
}

fn terminator_succs(term: &Terminator) -> Vec<BlockId> {
    match term {
        Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
        Terminator::Jump(t) => vec![*t],
        Terminator::Branch { then, else_, .. } => vec![*then, *else_],
    }
}

pub fn dominators(func: &Function) -> HashMap<BlockId, HashSet<BlockId>> {
    let cfg = build_cfg(func);
    let all: HashSet<BlockId> = cfg.blocks.iter().copied().collect();

    let mut dom: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
    for &b in &cfg.blocks {
        if b == cfg.entry {
            dom.insert(b, HashSet::from([b]));
        } else {
            dom.insert(b, all.clone());
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for &b in &cfg.blocks {
            if b == cfg.entry {
                continue;
            }

            let preds = cfg.preds.get(&b).cloned().unwrap_or_default();
            if preds.is_empty() {
                // Unreachable blocks dominate only themselves.
                let new = HashSet::from([b]);
                if dom.get(&b) != Some(&new) {
                    dom.insert(b, new);
                    changed = true;
                }
                continue;
            }

            let mut it = preds.iter();
            let first = *it.next().unwrap();
            let mut new_set = dom.get(&first).cloned().unwrap_or_default();
            for p in it {
                if let Some(s) = dom.get(p) {
                    new_set = new_set.intersection(s).copied().collect();
                } else {
                    new_set.clear();
                }
            }
            new_set.insert(b);

            if dom.get(&b) != Some(&new_set) {
                dom.insert(b, new_set);
                changed = true;
            }
        }
    }

    dom
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FoldStats {
    pub folded: usize,
}

pub fn constant_fold_function(func: &mut Function) -> FoldStats {
    let mut stats = FoldStats::default();
    for block in &mut func.body {
        stats.folded += constant_fold_block(block);
    }
    stats
}

fn constant_fold_block(block: &mut BasicBlock) -> usize {
    let mut consts: HashMap<ValueId, ConstValue> = HashMap::new();
    let mut folded = 0;

    for inst in &mut block.instructions {
        let cur = inst.clone();
        match cur {
            Instruction::Const { dest, value } => {
                consts.insert(dest, value);
            }
            Instruction::UnOp { op, arg, dest } => {
                if let Some(v) = consts.get(&arg).cloned() {
                    if let Some(out) = fold_unop(op, v) {
                        *inst = Instruction::Const {
                            dest,
                            value: out.clone(),
                        };
                        consts.insert(dest, out);
                        folded += 1;
                    }
                }
            }
            Instruction::BinOp {
                op,
                lhs,
                rhs,
                dest,
            } => {
                let l = consts.get(&lhs).cloned();
                let r = consts.get(&rhs).cloned();
                if let (Some(l), Some(r)) = (l, r) {
                    if let Some(out) = fold_binop(op, l, r) {
                        *inst = Instruction::Const {
                            dest,
                            value: out.clone(),
                        };
                        consts.insert(dest, out);
                        folded += 1;
                    }
                }
            }
            _ => {}
        };
    }

    folded
}

fn fold_unop(op: UnOp, v: ConstValue) -> Option<ConstValue> {
    match (op, v) {
        (UnOp::Neg, ConstValue::Number(n)) => Some(ConstValue::Number(-n)),
        (UnOp::Not, ConstValue::Bool(b)) => Some(ConstValue::Bool(!b)),
        _ => None,
    }
}

fn fold_binop(op: BinOp, l: ConstValue, r: ConstValue) -> Option<ConstValue> {
    match (op, l, r) {
        (BinOp::Add, ConstValue::Number(a), ConstValue::Number(b)) => Some(ConstValue::Number(a + b)),
        (BinOp::Sub, ConstValue::Number(a), ConstValue::Number(b)) => Some(ConstValue::Number(a - b)),
        (BinOp::Mul, ConstValue::Number(a), ConstValue::Number(b)) => Some(ConstValue::Number(a * b)),
        (BinOp::Div, ConstValue::Number(a), ConstValue::Number(b)) => Some(ConstValue::Number(a / b)),
        (BinOp::Mod, ConstValue::Number(a), ConstValue::Number(b)) => Some(ConstValue::Number(a % b)),
        (BinOp::Eq, a, b) => Some(ConstValue::Bool(const_eq(&a, &b))),
        (BinOp::Ne, a, b) => Some(ConstValue::Bool(!const_eq(&a, &b))),
        (BinOp::Lt, ConstValue::Number(a), ConstValue::Number(b)) => Some(ConstValue::Bool(a < b)),
        (BinOp::Le, ConstValue::Number(a), ConstValue::Number(b)) => Some(ConstValue::Bool(a <= b)),
        (BinOp::Gt, ConstValue::Number(a), ConstValue::Number(b)) => Some(ConstValue::Bool(a > b)),
        (BinOp::Ge, ConstValue::Number(a), ConstValue::Number(b)) => Some(ConstValue::Bool(a >= b)),
        _ => None,
    }
}

fn const_eq(a: &ConstValue, b: &ConstValue) -> bool {
    match (a, b) {
        (ConstValue::Number(x), ConstValue::Number(y)) => x == y,
        (ConstValue::String(x), ConstValue::String(y)) => x == y,
        (ConstValue::Bool(x), ConstValue::Bool(y)) => x == y,
        (ConstValue::Null, ConstValue::Null) => true,
        _ => false,
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DceStats {
    pub removed: usize,
}

pub fn local_dce_function(func: &mut Function) -> DceStats {
    let mut stats = DceStats::default();
    for block in &mut func.body {
        stats.removed += local_dce_block(block);
    }
    stats
}

fn local_dce_block(block: &mut BasicBlock) -> usize {
    let mut live: HashSet<ValueId> = HashSet::new();
    add_terminator_uses(&block.terminator, &mut live);

    let mut removed = 0usize;
    let mut out: Vec<Instruction> = Vec::with_capacity(block.instructions.len());

    for inst in block.instructions.iter().rev() {
        let (def, uses, side_effect) = inst_def_uses(inst);

        let keep = side_effect || def.is_none() || def.map(|d| live.contains(&d)).unwrap_or(false);
        if keep {
            if let Some(d) = def {
                live.remove(&d);
            }
            for u in uses {
                live.insert(u);
            }
            out.push(inst.clone());
        } else {
            removed += 1;
        }
    }

    out.reverse();
    block.instructions = out;
    removed
}

fn add_terminator_uses(term: &Terminator, live: &mut HashSet<ValueId>) {
    match term {
        Terminator::Return(Some(v)) => {
            live.insert(*v);
        }
        Terminator::Branch { cond, .. } => {
            live.insert(*cond);
        }
        _ => {}
    }
}

fn inst_def_uses(inst: &Instruction) -> (Option<ValueId>, Vec<ValueId>, bool) {
    match inst {
        Instruction::Const { dest, .. } => (Some(*dest), vec![], false),
        Instruction::VarRef { dest, .. } => (Some(*dest), vec![], false),
        Instruction::Member { object, dest, .. } => (Some(*dest), vec![*object], false),
        Instruction::MemberComputed {
            object,
            property,
            dest,
        } => (Some(*dest), vec![*object, *property], false),
        Instruction::UnOp { arg, dest, .. } => (Some(*dest), vec![*arg], false),
        Instruction::BinOp { lhs, rhs, dest, .. } => (Some(*dest), vec![*lhs, *rhs], false),
        Instruction::ObjectLit { dest, props } => {
            (Some(*dest), props.iter().map(|p| p.value).collect(), false)
        }
        Instruction::ArrayLit { dest, elements } => (
            Some(*dest),
            elements.iter().flatten().copied().collect(),
            false,
        ),
        Instruction::LogicalOp { lhs, rhs, dest, .. } => (Some(*dest), vec![*lhs, *rhs], false),
        Instruction::Conditional {
            cond,
            then_value,
            else_value,
            dest,
        } => (Some(*dest), vec![*cond, *then_value, *else_value], false),
        Instruction::Call { callee, args, dest } => (Some(*dest), uses_vec(*callee, args), true),
        Instruction::New { callee, args, dest } => (Some(*dest), uses_vec(*callee, args), true),
        Instruction::Await { arg, dest } => (Some(*dest), vec![*arg], true),
        Instruction::AssignExpr { src, dest, .. } => (Some(*dest), vec![*src], true),
        Instruction::ExprStmt { value } => (None, vec![*value], true),
        Instruction::VarDecl { init, .. } => (None, init.iter().copied().collect(), true),
        Instruction::AssignVar { src, .. } => (None, vec![*src], true),
        Instruction::Store { dest, src } => (None, vec![*dest, *src], true),
        Instruction::Load { dest, src } => (Some(*dest), vec![*src], false),
        Instruction::ThrowStmt { arg } => (None, vec![*arg], true),
        Instruction::Try {
            try_body,
            catch,
            finally_body,
        } => {
            let mut uses = Vec::new();
            // Don't optimize inside nested bodies (yet); treat as side-effect and mark all uses live.
            for i in try_body {
                let (_, u, _) = inst_def_uses(i);
                uses.extend(u);
            }
            if let Some(c) = catch {
                for i in &c.body {
                    let (_, u, _) = inst_def_uses(i);
                    uses.extend(u);
                }
            }
            if let Some(f) = finally_body {
                for i in f {
                    let (_, u, _) = inst_def_uses(i);
                    uses.extend(u);
                }
            }
            (None, uses, true)
        }
    }
}

fn uses_vec(callee: ValueId, args: &[ValueId]) -> Vec<ValueId> {
    let mut out = Vec::with_capacity(1 + args.len());
    out.push(callee);
    out.extend_from_slice(args);
    out
}
