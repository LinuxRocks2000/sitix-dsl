// the bytecode interpreter
const OP_RETURN : u8 = 0; // return from the current *block*.
const OP_CONSTANT : u8 = 1;

use num_traits::ops::{ FromBytes, ToBytes };

enum Value {
    Number(f64)
}


struct Program {
    bytes : Vec<u8>,
    head : usize
}


impl Program {
    fn new() -> Self {
        Self {
            bytes : Vec::new(),
            head : 0
        }
    }

    fn push(&mut self, num : impl ToBytes) {
        self.bytes.extend(&num.to_bytes());
    }
}


struct Machine {
    program : Program,
    lines : Vec<(u16, u16)>,
    constants : Vec<Value>
}


impl Machine {
    fn new() -> Self {
        Self {
            program : Program::new(),
            lines : Vec::new(),
            constants : Vec::new()
        }
    }

    fn push_inst(&mut self, opcode : u8, line : u16, operands : &[&dyn ToBytes + Copy]) {
        self.program.push(opcode);
        for operand in operands {
            self.program.push(*operand);
        }
        if let Some((l, cnt)) = self.lines.last_mut() {
            if *l == line {
                cnt += 1;
                return;
            }
        }
        self.lines.push((line, 1));
    }

    fn add_constant(&mut self, constant : Value) -> usize {
        self.constants.push(constant);
        self.constants.len() - 1
    }

    fn disassemble(&mut self) {
        let mut linedex = 0;
        let mut line_run = 1;
        loop {
            println!("{}. (line {}) {}", ind, lines[linedex].0, match *inst {
                OP_RETURN => "ret",
                _ => "invalid instruction"
            });
            line_run -= 1;
            if line_run == 0 {
                linedex += 1;
                line_run = lines[linedex].1;
            }
        }
    }
}


fn main() {
    let mut machine = Machine::new();
    let constant = machine.add_constant(Value::Number(5.0));
    machine.push_inst(OP_CONSTANT, 1, &[&constant]);
    machine.disassemble();
}
