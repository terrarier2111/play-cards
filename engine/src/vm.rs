use crate::{
    bytecode::{ByteCode, Function},
    rt::{Ordering, RtRef, RtType},
};

pub struct Vm {
    code: Vec<ByteCode>,
    ip: usize,
    stack: Vec<RtRef>,
    funcs: Vec<Function>,
}

impl Vm {
    pub fn new(code: Vec<ByteCode>, funcs: Vec<Function>) -> Self {
        Self {
            code,
            ip: 0,
            stack: vec![],
            funcs,
        }
    }

    fn cleanup(val: RtRef) {
        // free up unused memory
        match val.ty() {
            RtType::String => {
                let _ = unsafe { Box::from_raw(val.dst()) };
            }
            RtType::Cards => todo!(),
            _ => {}
        }
    }

    fn clone_ref(val: RtRef) -> RtRef {
        match val.ty() {
            RtType::String => {
                let cloned = unsafe { val.get_string_directly() }.clone();
                RtRef::string(Box::new(cloned))
            }
            RtType::Cards => todo!(),
            _ => val,
        }
    }

    pub fn run(&mut self) {
        // FIXME: run an optimizer on the bytecode beforehand, eliminating push/pop sequences
        while let Some(curr) = self.code.get(self.ip) {
            match curr {
                ByteCode::Push { val } => {
                    self.stack.push(Self::clone_ref(*val));
                }
                ByteCode::Pop { offset } => {
                    let val = self.stack.remove(self.stack.len() - 1 - *offset as usize);
                    Self::cleanup(val);
                }
                ByteCode::Mov { src_idx, dst_idx } => {
                    let prev = self.stack[*dst_idx as usize];

                    self.stack[*dst_idx as usize] = Self::clone_ref(self.stack[*src_idx as usize]);
                    Self::cleanup(prev);
                }
                ByteCode::Call {
                    fn_idx,
                    push_val,
                    arg_indices,
                } => {
                    let func = &mut self.funcs[*fn_idx as usize];
                    let args = {
                        let mut args = vec![];
                        // FIXME: perform type checking!
                        for (i, idx) in arg_indices.iter().enumerate() {
                            let val = self.stack.get(*idx as usize).unwrap();
                            args.push(*val);
                        }
                        args
                    };
                    let fun = func.call;
                    let val = fun(args);
                    if *push_val {
                        // FIXME: should we even push if the value is None?
                        self.stack.push(val.unwrap_or(RtRef::NULL));
                    }
                }
                ByteCode::Add { arg1_idx, arg2_idx } => {
                    let left = *self.stack.get(*arg1_idx as usize).unwrap();
                    let right = *self.stack.get(*arg2_idx as usize).unwrap();
                    self.stack.push(RtRef::decimal(
                        left.get_decimal().unwrap() + right.get_decimal().unwrap(),
                    ));
                }
                ByteCode::Sub { arg1_idx, arg2_idx } => {
                    let left = *self.stack.get(*arg1_idx as usize).unwrap();
                    let right = *self.stack.get(*arg2_idx as usize).unwrap();
                    self.stack.push(RtRef::decimal(
                        left.get_decimal().unwrap() - right.get_decimal().unwrap(),
                    ));
                }
                ByteCode::Mul { arg1_idx, arg2_idx } => {
                    let left = *self.stack.get(*arg1_idx as usize).unwrap();
                    let right = *self.stack.get(*arg2_idx as usize).unwrap();
                    self.stack.push(RtRef::decimal(
                        left.get_decimal().unwrap() * right.get_decimal().unwrap(),
                    ));
                }
                ByteCode::Div { arg1_idx, arg2_idx } => {
                    let left = *self.stack.get(*arg1_idx as usize).unwrap();
                    let right = *self.stack.get(*arg2_idx as usize).unwrap();
                    self.stack.push(RtRef::decimal(
                        left.get_decimal().unwrap() / right.get_decimal().unwrap(),
                    ));
                }
                ByteCode::Mod { arg1_idx, arg2_idx } => {
                    let left = *self.stack.get(*arg1_idx as usize).unwrap();
                    let right = *self.stack.get(*arg2_idx as usize).unwrap();
                    self.stack.push(RtRef::decimal(
                        left.get_decimal().unwrap() % right.get_decimal().unwrap(),
                    ));
                }
                ByteCode::And { arg1_idx, arg2_idx } => {
                    let left = *self.stack.get(*arg1_idx as usize).unwrap();
                    let right = *self.stack.get(*arg2_idx as usize).unwrap();
                    self.stack.push(RtRef::bool(
                        left.get_bool().unwrap() && right.get_bool().unwrap(),
                    ));
                }
                ByteCode::Or { arg1_idx, arg2_idx } => {
                    let left = *self.stack.get(*arg1_idx as usize).unwrap();
                    let right = *self.stack.get(*arg2_idx as usize).unwrap();
                    self.stack.push(RtRef::bool(
                        left.get_bool().unwrap() || right.get_bool().unwrap(),
                    ));
                }
                ByteCode::Jump { relative_off } => {
                    self.ip = ((self.ip as isize) + *relative_off) as usize; // FIXME: guard against overflow!
                    continue;
                }
                ByteCode::JumpCond {
                    relative_off,
                    arg_idx,
                } => {
                    let val = *self.stack.get(*arg_idx as usize).unwrap(); // FIXME: guard against inval param
                    if val.ty() != RtType::Bool {
                        panic!("invalid type {:?} {:?}", val.ty(), val.get_decimal());
                        // FIXME: auto convert to bool if possible
                    }
                    if val == RtRef::bool(true) {
                        self.ip = ((self.ip as isize) + *relative_off) as usize; // FIXME: guard against overflow!
                        continue;
                    }
                }
                ByteCode::Compare {
                    arg1_idx,
                    arg2_idx,
                    expected,
                } => {
                    let left = *self.stack.get(*arg1_idx as usize).unwrap();
                    let right = *self.stack.get(*arg2_idx as usize).unwrap();
                    // FIXME: add implicit conversion
                    assert!(
                        left.ty() == right.ty(),
                        "got {:?} and {:?}",
                        left.ty(),
                        right.ty()
                    );
                    let cmp = match left.ty() {
                        RtType::Decimal => Ordering::from_std(unsafe {
                            left.get_decimal_directly()
                                .total_cmp(&right.get_decimal_directly())
                        }),
                        RtType::None => Ordering::Equal,
                        RtType::Bool => {
                            if left == right {
                                Ordering::Equal
                            } else {
                                Ordering::NotEqual
                            }
                        }
                        RtType::String => todo!(),
                        RtType::Player => todo!(),
                        RtType::Inventory => todo!(),
                        RtType::Cards => todo!(),
                    };
                    self.stack.push(RtRef::bool(*expected == cmp));
                }
            }
            self.ip += 1;
        }
    }
}
