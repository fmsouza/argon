//! SafeScript - WebAssembly code generator

use safescript_ir::*;

pub struct WasmCodegen;

impl WasmCodegen {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&self, ir_module: &Module) -> Result<Vec<u8>, CodegenError> {
        let mut wasm = Vec::new();

        wasm.extend_from_slice(b"\0\x61\x73\x6d\x01\x00\x00\x00");

        let type_count = ir_module.functions.len() as u32;
        if type_count > 0 {
            wasm.push(0x01);
            wasm.extend_from_slice(&varint(type_count as i64));
            wasm.push(0x60);
            wasm.push(0x00);
            wasm.push(0x01);
            wasm.push(0x7f);
        }

        wasm.push(0x03);
        wasm.extend_from_slice(&varint(type_count as i64));
        for _ in 0..type_count {
            wasm.push(0x00);
        }

        wasm.push(0x05);
        wasm.extend_from_slice(&varint(3));
        wasm.push(0x00);
        wasm.push(0x01);

        wasm.push(0x0a);
        wasm.extend_from_slice(&varint(4 + type_count as i64 * 3));

        for func in ir_module.functions.iter().take(1) {
            let locals_count = func.params.len();
            wasm.push(0x00);
            wasm.extend_from_slice(&varint(locals_count as i64));

            for _ in 0..locals_count {
                wasm.push(0x01);
                wasm.push(0x7f);
            }

            for block in &func.body {
                for instr in &block.instructions {
                    self.emit_instruction(instr, &mut wasm);
                }
                match &block.terminator {
                    Terminator::Return(value) => {
                        if value.is_some() {
                            wasm.push(0x20);
                            wasm.push(0x00);
                        }
                        wasm.push(0x0f);
                    }
                    Terminator::Jump(_) => {
                        wasm.push(0x0f);
                    }
                    Terminator::Branch { .. } => {
                        wasm.push(0x0f);
                    }
                    Terminator::Unreachable => {
                        wasm.push(0x00);
                    }
                }
            }

            wasm.push(0x0b);
        }

        Ok(wasm)
    }

    fn emit_instruction(&self, instr: &Instruction, wasm: &mut Vec<u8>) {
        match instr {
            Instruction::Const { value, .. } => {
                let n = match value {
                    ConstValue::Number(n) => *n as i32,
                    ConstValue::Bool(b) => {
                        if *b {
                            1
                        } else {
                            0
                        }
                    }
                    ConstValue::String(_) => 0,
                    ConstValue::Null => 0,
                };
                wasm.push(0x41);
                wasm.extend_from_slice(&varint(n as i64));
            }
            Instruction::BinOp { op, .. } => {
                let code = match op {
                    BinOp::Add => 0x6a,
                    BinOp::Sub => 0x6b,
                    BinOp::Mul => 0x6c,
                    BinOp::Div => 0x6d,
                    BinOp::Mod => 0x6f,
                    BinOp::Eq => 0x46,
                    BinOp::Ne => 0x47,
                    BinOp::Lt => 0x48,
                    BinOp::Le => 0x49,
                    BinOp::Gt => 0x4a,
                    BinOp::Ge => 0x4b,
                    BinOp::And => 0x71,
                    BinOp::Or => 0x72,
                    BinOp::Xor => 0x73,
                    BinOp::Shl => 0x74,
                    BinOp::Shr => 0x75,
                    BinOp::Sar => 0x76,
                };
                wasm.push(code);
            }
            Instruction::UnOp { op, .. } => {
                let code = match op {
                    UnOp::Neg => 0x67,
                    _ => 0x67,
                };
                wasm.push(code);
            }
            Instruction::Load { .. } => {
                wasm.push(0x28);
                wasm.push(0x00);
                wasm.push(0x00);
            }
            Instruction::Store { .. } => {
                wasm.push(0x36);
                wasm.push(0x00);
                wasm.push(0x00);
            }
            _ => {}
        }
    }
}

fn varint(n: i64) -> Vec<u8> {
    let mut n = n;
    let mut bytes = Vec::new();
    loop {
        let mut byte = (n & 0x7f) as u8;
        n >>= 7;
        if n != 0 {
            byte |= 0x80;
        }
        bytes.push(byte);
        if n == 0 {
            break;
        }
    }
    bytes
}

impl Default for WasmCodegen {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum CodegenError {
    Unsupported(String),
}
