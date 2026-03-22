use koopa::ir::{FunctionData, Value, ValueKind};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::option::Option;
use crate::ast::EvalExp;

/// 活跃区间：值从定义（start）到最后使用（end）的区间
#[derive(Debug, Clone)]
pub struct LiveInterval {
    value: Value,
    start: usize,        // 指令序号（程序点）
    end: usize,          // 最后使用点
    reg: Option<String>, // 分配的寄存器（如果有）
}

/// 按 end 排序用于 active 集合
#[derive(Debug, Clone)]
pub struct ActiveInterval {
    end: usize,
    value: Value,
    reg: String,
}

impl PartialEq for ActiveInterval {
    fn eq(&self, other: &Self) -> bool {
        self.end == other.end
    }
}
impl Eq for ActiveInterval {}
impl PartialOrd for ActiveInterval {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.end.cmp(&self.end)) // 最小堆（end 小的在顶部）
    }
}
impl Ord for ActiveInterval {
    fn cmp(&self, other: &Self) -> Ordering {
        other.end.cmp(&self.end)
    }
}

/// 函数级寄存器分配器
pub struct LinearScanAlloc {
    // 可用的物理寄存器池（排除预留的 scratch）
    free_regs: Vec<String>,
    // 已分配：Value -> 物理寄存器
    allocation: HashMap<Value, String>,
    // Spill 到栈的映射：Value -> 栈偏移
    stack_slots: HashMap<Value, usize>,
    stack_count: usize,
    // 预留的临时寄存器（给 CodeGen 用，不参与分配）
    scratch_regs: Vec<String>,
}

struct op_results {
    reg: Vec<Value>,
    stack: Vec<Value>,
}

impl LinearScanAlloc {
    pub fn new() -> Self {
        LinearScanAlloc {
            free_regs: vec![
                // "t0".to_string(),
                // "t1".to_string(),
                // "t2".to_string(),
                // "t3".to_string(),
                // "t4".to_string(),
                "a0".to_string(),
                "a1".to_string(),
                "a2".to_string(),
                "a3".to_string(),
                "a4".to_string(),
                "a5".to_string(),
                "a6".to_string(),
                "a7".to_string(),
            ],
            allocation: HashMap::new(),
            stack_slots: HashMap::new(),
            stack_count: 0,
            scratch_regs: vec!["t5".to_string(), "t6".to_string()], // 预留 s0, s1 给 CodeGen
        }
    }

    fn build_intervals(&self, func: &FunctionData) -> (Vec<LiveInterval>, HashMap<Value, usize>) {
        let dfg = func.dfg();
        let mut start: HashMap<Value, usize> = HashMap::new();
        let mut end: HashMap<Value, usize> = HashMap::new();
        let mut stack_start: HashMap<Value, usize> = HashMap::new();
        let mut stack_end: HashMap<Value, usize> = HashMap::new();
        let mut program_point = 0usize;
        // 按布局顺序遍历基本块
        for (bb, node) in func.layout().bbs() {
            // 处理基本块参数（如果有的话）
            for param in dfg.bb(*bb).params() {
                // 块参数在块入口处定义
                start.insert(*param, program_point);
                program_point += 1;
            }

            // 遍历指令
            for &inst in node.insts().keys() {
                let value_data = dfg.value(inst);
                // println!("Value: {:?}, Inst: {:?}", value_data, inst);
                // 记录定义点
                // 排除没有结果的指令（如 branch, jump, store）
                if self.has_result(value_data) {
                    if self.is_alloc(value_data) {
                        stack_start.insert(inst, program_point);
                    } else {
                        start.insert(inst, program_point);
                    }
                }

                // 更新操作数的最后使用点
                let op_res = self.get_operands(value_data, &inst);
                for register in op_res.reg {
                    // println!("Operand: {:?}", register);
                    // 只有当操作数不是立即数（Integer）时，才需要分配寄存器/记录活跃区间
                    if !matches!(dfg.value(register).kind(), ValueKind::Integer(_)) {
                        end.insert(register, program_point);
                    }
                }
                for stack in op_res.stack {
                    stack_end.insert(stack, program_point);
                }

                program_point += 1;
            }
        }
        let mut liveint = Vec::new(); // 创建一个新的 Vec
        for (k, v) in start.iter() {
            let live_interval = LiveInterval {
                value: *k,
                start: *v,
                end: *end.get(k).unwrap_or(v), // 如果没有使用点，end 就是 start
                reg: None,
            };
            println!("Live interval for value {:?}: {:?}", k, live_interval);
            liveint.push(live_interval); // 记得把构建好的 interval 存入 Vec
        }
        (liveint, stack_start) // 返回填充好的 Vec
    }

    pub fn allocate(&mut self, func: &FunctionData) {
        let (mut intervals, stack_maps) = self.build_intervals(func);
        intervals.sort_by_key(|i| i.start); // 按 start 升序排序
        let mut active: BinaryHeap<ActiveInterval> = BinaryHeap::new();
        let mut stack_slots: Vec<Value> = Vec::new();

        for current in intervals {
            println!(
                "Allocating for value {:?} (start: {}, end: {})",
                current.value, current.start, current.end
            );
            // 1. 释放已死亡的区间
            while let Some(top) = active.peek() {
                if top.end < current.start {
                    let dead: ActiveInterval = active.pop().unwrap();
                    println!(
                        "  Freeing register {} from expired value {:?}",
                        dead.reg, dead.value
                    );
                    self.free_regs.push(dead.reg);
                } else {
                    break;
                }
            }

            // 2. 尝试分配
            if let Some(reg) = self.free_regs.pop() {
                // 有空闲寄存器
                println!("  Assigned register {} to value {:?}", reg, current.value);
                self.allocation.insert(current.value, reg.clone());
                active.push(ActiveInterval {
                    end: current.end,
                    value: current.value,
                    reg,
                });
            } else {
                stack_slots.push(current.value);
                println!("No free registers found, stack size {} assigned to value {:?}, ", stack_slots.len() * 4, current.value);
            }

            // {
            //     // 3. 需要 spill：比较结束时间
            //     let spill_candidate = active
            //         .peek()
            //         .expect("Active set should not be empty if no free regs");
            //     if spill_candidate.end > current.end {
            //         // 驱逐结束最晚的，给当前用
            //         let spilled = active.pop().unwrap();
            //         println!(
            //             "  Spilling value {:?} (end: {}) to free register {}",
            //             spilled.value, spilled.end, spilled.reg
            //         );
            //         self.allocation.remove(&spilled.value);
            //         self.spill_value(spilled.value);
            //
            //         println!(
            //             "  Assigned stolen register {} to value {:?}",
            //             spilled.reg, current.value
            //         );
            //         self.allocation.insert(current.value, spilled.reg.clone());
            //         active.push(ActiveInterval {
            //             end: current.end,
            //             value: current.value,
            //             reg: spilled.reg,
            //         });
            //     } else {
            //         // 当前结束更早，直接 spill 当前
            //         println!("  Spilling current value {:?} immediately", current.value);
            //         self.spill_value(current.value);
            //     }
            // }
        }

        // 3.分配stack
        for (stack, place) in stack_maps.iter() {
            stack_slots.push(*stack);
            println!("Assigned stack size {} assigned to value {:?}", stack_slots.len() * 4, stack);
        }

        self.stack_count = stack_slots.len() * 4;
        for (idx, stack) in stack_slots.iter().enumerate() {
            self.stack_slots.insert(*stack, self.stack_count - idx * 4);
        }
    }

    /// 判断指令是否产生结果（需要寄存器）
    fn has_result(&self, data: &koopa::ir::entities::ValueData) -> bool {
        use koopa::ir::ValueKind::*;
        !matches!(data.kind(), Branch(_) | Jump(_) | Store(_) | Return(_))
    }

    fn is_alloc(&self, data: &koopa::ir::entities::ValueData) -> bool {
        use koopa::ir::ValueKind::*;
        matches!(data.kind(), Alloc(_))
    }

    pub fn get_stack_count(&self) -> usize {
        self.stack_count
    }

    fn get_operands(&self, data: &koopa::ir::entities::ValueData, inst: &Value) -> op_results {
        use koopa::ir::ValueKind::*;
        let mut ret = op_results { reg: Vec::new(), stack: Vec::new() };
        match data.kind() {
            Binary(bin) => ret.reg = vec![bin.lhs(), bin.rhs()],
            Return(retval) => ret.reg = retval.value().map_or(vec![], |v| vec![v]),
            Alloc(_) => ret.stack = vec![*inst], // We suppose the memory of a variable is allocated when alloc, just like C
            Load(load) => {
                ret.reg = vec![*inst];
                ret.stack = vec![load.src()];
            }
            // Store(store) => vec![store.dest(), store.value()],
            _ => {}
        }
        ret
    }

    // fn spill_value(&mut self, value: Value) {
    //     stack_slots.insert(value, self.next_stack_slot);
    //     self.next_stack_slot += 4; // 假设 4 字节
    // }

    /// 查询分配结果
    pub fn get_reg(&self, value: &Value) -> Option<&String> {
        println!(
            "Querying register for value {:?} in allocation: {:?}",
            value, self.allocation
        );
        self.allocation.get(value)
    }

    pub fn get_stack(&self, value: &Value) -> Option<&usize> {
        self.stack_slots.get(value)
    }
}
