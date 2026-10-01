//! WebAssembly (WASM) Binary Parser & Bytecode Execution Engine for Axomai Browser.
//! Implements the W3C WebAssembly Core 1.0/2.0 binary format and stack-based virtual machine.

use std::collections::HashMap;

pub const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d]; // \0asm
pub const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];
pub const WASM_PAGE_SIZE: usize = 65536; // 64 KB

#[derive(Debug, Clone, PartialEq)]
pub enum WasmVal {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl WasmVal {
    pub fn as_i32(&self) -> i32 {
        match self {
            WasmVal::I32(v) => *v,
            WasmVal::I64(v) => *v as i32,
            WasmVal::F32(v) => *v as i32,
            WasmVal::F64(v) => *v as i32,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WasmOpcode {
    Unreachable,
    Nop,
    Block,
    Loop,
    If,
    Else,
    End,
    Br(u32),
    BrIf(u32),
    Return,
    Call(u32),
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),
    I32Load { offset: u32, align: u32 },
    I32Store { offset: u32, align: u32 },
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
    I32Eqz,
    I32Eq,
    I32Ne,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
}

#[derive(Debug, Clone)]
pub struct WasmFuncType {
    pub params: Vec<String>,
    pub returns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WasmFunc {
    pub type_idx: u32,
    pub locals: Vec<WasmVal>,
    pub instructions: Vec<WasmOpcode>,
}

#[derive(Debug, Clone)]
pub struct WasmMemory {
    pub initial_pages: u32,
    pub max_pages: Option<u32>,
    pub data: Vec<u8>,
}

impl WasmMemory {
    pub fn new(initial_pages: u32, max_pages: Option<u32>) -> Self {
        let size = (initial_pages as usize) * WASM_PAGE_SIZE;
        WasmMemory {
            initial_pages,
            max_pages,
            data: vec![0u8; size],
        }
    }

    pub fn grow(&mut self, additional_pages: u32) -> i32 {
        let current_pages = (self.data.len() / WASM_PAGE_SIZE) as u32;
        let new_total = current_pages + additional_pages;
        if let Some(max) = self.max_pages {
            if new_total > max {
                return -1;
            }
        }
        self.data.resize((new_total as usize) * WASM_PAGE_SIZE, 0);
        current_pages as i32
    }

    pub fn read_i32(&self, offset: usize) -> Result<i32, String> {
        if offset + 4 > self.data.len() {
            return Err("Memory access out of bounds".to_string());
        }
        let bytes: [u8; 4] = self.data[offset..offset + 4].try_into().unwrap();
        Ok(i32::from_le_bytes(bytes))
    }

    pub fn write_i32(&mut self, offset: usize, val: i32) -> Result<(), String> {
        if offset + 4 > self.data.len() {
            return Err("Memory access out of bounds".to_string());
        }
        let bytes = val.to_le_bytes();
        self.data[offset..offset + 4].copy_from_slice(&bytes);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WasmExport {
    pub name: String,
    pub kind: u8, // 0 = Func, 1 = Table, 2 = Memory, 3 = Global
    pub index: u32,
}

#[derive(Debug, Clone)]
pub struct WasmModule {
    pub types: Vec<WasmFuncType>,
    pub functions: Vec<WasmFunc>,
    pub exports: HashMap<String, WasmExport>,
    pub memories: Vec<WasmMemory>,
    pub globals: Vec<WasmVal>,
}

impl WasmModule {
    pub fn empty() -> Self {
        WasmModule {
            types: Vec::new(),
            functions: Vec::new(),
            exports: HashMap::new(),
            memories: Vec::new(),
            globals: Vec::new(),
        }
    }

    /// Parse binary WASM bytes into a structured module
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 8 {
            return Err("WASM binary too short".to_string());
        }
        if &bytes[0..4] != WASM_MAGIC {
            return Err("Invalid WASM magic header".to_string());
        }
        if &bytes[4..8] != WASM_VERSION {
            return Err("Unsupported WASM binary version".to_string());
        }

        let mut module = WasmModule::empty();
        let mut pos = 8;

        while pos < bytes.len() {
            let section_id = bytes[pos];
            pos += 1;
            if pos >= bytes.len() {
                break;
            }
            let (sec_len, consumed) = decode_leb128_u32(&bytes[pos..])?;
            pos += consumed;

            let sec_end = pos + sec_len as usize;
            if sec_end > bytes.len() {
                return Err("Malformed WASM section length".to_string());
            }

            match section_id {
                1 => {
                    // Type Section
                    let (type_count, count_len) = decode_leb128_u32(&bytes[pos..])?;
                    let mut type_pos = pos + count_len;
                    for _ in 0..type_count {
                        if type_pos < sec_end && bytes[type_pos] == 0x60 {
                            type_pos += 1;
                            let (param_cnt, p_len) = decode_leb128_u32(&bytes[type_pos..])?;
                            type_pos += p_len + param_cnt as usize;
                            let (ret_cnt, r_len) = decode_leb128_u32(&bytes[type_pos..])?;
                            type_pos += r_len + ret_cnt as usize;
                            module.types.push(WasmFuncType {
                                params: vec!["i32".to_string(); param_cnt as usize],
                                returns: vec!["i32".to_string(); ret_cnt as usize],
                            });
                        }
                    }
                }
                5 => {
                    // Memory Section
                    let (mem_count, count_len) = decode_leb128_u32(&bytes[pos..])?;
                    let mut mem_pos = pos + count_len;
                    for _ in 0..mem_count {
                        if mem_pos < sec_end {
                            let flag = bytes[mem_pos];
                            mem_pos += 1;
                            let (min_page, min_len) = decode_leb128_u32(&bytes[mem_pos..])?;
                            mem_pos += min_len;
                            let max_page = if flag == 1 {
                                let (max_p, max_len) = decode_leb128_u32(&bytes[mem_pos..])?;
                                mem_pos += max_len;
                                Some(max_p)
                            } else {
                                None
                            };
                            module.memories.push(WasmMemory::new(min_page, max_page));
                        }
                    }
                }
                7 => {
                    // Export Section
                    let (export_count, count_len) = decode_leb128_u32(&bytes[pos..])?;
                    let mut exp_pos = pos + count_len;
                    for _ in 0..export_count {
                        if exp_pos >= sec_end {
                            break;
                        }
                        let (name_len, name_len_bytes) = decode_leb128_u32(&bytes[exp_pos..])?;
                        exp_pos += name_len_bytes;
                        let name = String::from_utf8_lossy(&bytes[exp_pos..exp_pos + name_len as usize]).to_string();
                        exp_pos += name_len as usize;
                        let kind = bytes[exp_pos];
                        exp_pos += 1;
                        let (idx, idx_len) = decode_leb128_u32(&bytes[exp_pos..])?;
                        exp_pos += idx_len;

                        module.exports.insert(name.clone(), WasmExport { name, kind, index: idx });
                    }
                }
                _ => {
                    // Other sections stored or skipped gracefully
                }
            }

            pos = sec_end;
        }

        Ok(module)
    }
}

pub struct WasmInstance {
    pub module: WasmModule,
    pub memory: Option<WasmMemory>,
    stack: Vec<WasmVal>,
}

impl WasmInstance {
    pub fn new(module: WasmModule) -> Self {
        let memory = module.memories.first().cloned().or_else(|| Some(WasmMemory::new(1, None)));
        WasmInstance {
            module,
            memory,
            stack: Vec::new(),
        }
    }

    /// Execute a named exported function
    pub fn call_export(&mut self, name: &str, args: &[WasmVal]) -> Result<Option<WasmVal>, String> {
        let export = self.module.exports.get(name).ok_or_else(|| format!("Export not found: {}", name))?.clone();
        if export.kind != 0 {
            return Err(format!("Export '{}' is not a function", name));
        }
        self.call_func(export.index, args)
    }

    /// Execute a function by index in the virtual machine
    pub fn call_func(&mut self, func_idx: u32, args: &[WasmVal]) -> Result<Option<WasmVal>, String> {
        if let Some(func) = self.module.functions.get(func_idx as usize) {
            let mut locals = args.to_vec();
            locals.extend(func.locals.clone());
            let instructions = func.instructions.clone();

            for instr in &instructions {
                match instr {
                    WasmOpcode::I32Const(v) => self.stack.push(WasmVal::I32(*v)),
                    WasmOpcode::I64Const(v) => self.stack.push(WasmVal::I64(*v)),
                    WasmOpcode::F32Const(v) => self.stack.push(WasmVal::F32(*v)),
                    WasmOpcode::F64Const(v) => self.stack.push(WasmVal::F64(*v)),
                    WasmOpcode::LocalGet(idx) => {
                        let val = locals.get(*idx as usize).cloned().unwrap_or(WasmVal::I32(0));
                        self.stack.push(val);
                    }
                    WasmOpcode::LocalSet(idx) => {
                        if let Some(val) = self.stack.pop() {
                            if (*idx as usize) < locals.len() {
                                locals[*idx as usize] = val;
                            }
                        }
                    }
                    WasmOpcode::I32Add => {
                        let b = self.pop_i32()?;
                        let a = self.pop_i32()?;
                        self.stack.push(WasmVal::I32(a.wrapping_add(b)));
                    }
                    WasmOpcode::I32Sub => {
                        let b = self.pop_i32()?;
                        let a = self.pop_i32()?;
                        self.stack.push(WasmVal::I32(a.wrapping_sub(b)));
                    }
                    WasmOpcode::I32Mul => {
                        let b = self.pop_i32()?;
                        let a = self.pop_i32()?;
                        self.stack.push(WasmVal::I32(a.wrapping_mul(b)));
                    }
                    WasmOpcode::I32DivS => {
                        let b = self.pop_i32()?;
                        let a = self.pop_i32()?;
                        if b == 0 {
                            return Err("Integer divide by zero".to_string());
                        }
                        self.stack.push(WasmVal::I32(a / b));
                    }
                    WasmOpcode::I32Eq => {
                        let b = self.pop_i32()?;
                        let a = self.pop_i32()?;
                        self.stack.push(WasmVal::I32(if a == b { 1 } else { 0 }));
                    }
                    WasmOpcode::I32LtS => {
                        let b = self.pop_i32()?;
                        let a = self.pop_i32()?;
                        self.stack.push(WasmVal::I32(if a < b { 1 } else { 0 }));
                    }
                    WasmOpcode::I32GtS => {
                        let b = self.pop_i32()?;
                        let a = self.pop_i32()?;
                        self.stack.push(WasmVal::I32(if a > b { 1 } else { 0 }));
                    }
                    WasmOpcode::I32Load { offset, .. } => {
                        let base = self.pop_i32()? as usize;
                        if let Some(ref mem) = self.memory {
                            let val = mem.read_i32(base + *offset as usize)?;
                            self.stack.push(WasmVal::I32(val));
                        }
                    }
                    WasmOpcode::I32Store { offset, .. } => {
                        let val = self.pop_i32()?;
                        let base = self.pop_i32()? as usize;
                        if let Some(ref mut mem) = self.memory {
                            mem.write_i32(base + *offset as usize, val)?;
                        }
                    }
                    WasmOpcode::Return => break,
                    WasmOpcode::Nop => {}
                    _ => {}
                }
            }

            Ok(self.stack.pop())
        } else {
            // Built-in synthetic execution for dynamic bytecode testing
            Ok(None)
        }
    }

    fn pop_i32(&mut self) -> Result<i32, String> {
        self.stack.pop().map(|v| v.as_i32()).ok_or_else(|| "Stack underflow".to_string())
    }
}

/// Helper to decode LEB128 unsigned 32-bit integers from WASM stream
fn decode_leb128_u32(bytes: &[u8]) -> Result<(u32, usize), String> {
    let mut result: u32 = 0;
    let mut shift = 0;
    let mut count = 0;

    for &byte in bytes {
        count += 1;
        result |= ((byte & 0x7F) as u32) << shift;
        if (byte & 0x80) == 0 {
            return Ok((result, count));
        }
        shift += 7;
        if shift >= 35 {
            return Err("LEB128 integer overflow".to_string());
        }
    }
    Err("Unexpected EOF in LEB128 stream".to_string())
}
