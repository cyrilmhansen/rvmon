#![forbid(unsafe_code)]

fn main() {
    let image = luna_assembler::assemble("addi x1,x0,1").expect("bootstrap source must assemble");
    let mut machine = luna_machine::Machine::new(4096);
    machine.load(image.entry, &image.text).expect("image must load");
    let result = machine.step().expect("addi must execute");
    println!("pc=0x{:016x} -> 0x{:016x}; x1=0x{:016x}; instructions={}", result.pc_before, result.pc_after, machine.x[1], machine.instructions);
    assert_eq!(machine.x[1], 1);
}
