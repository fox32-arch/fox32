# Fox32 instructions

This file describes every fox32 instruction in detail. For a general
description of the fox32 CPU and instruction encoding details, see [cpu.md](./cpu.md).


## Arithmetic instructions

### ADD: add
### SUB: subtract
### INC: increment (add 1)
### DEC: decrement (subtract 1)
### CMP: compare
### AND: bitwise AND
### OR: bitwise OR
### NOT: bitwise NOT
### XOR: bitwise XOR
### BSE: bit set
### BCL: bit clear
### BTS: test if bit set
### SLA: shift left
### SRA: shift right arithmetic (with sign extension)
### SRL: shift right logical (with zero extension)
### ROL: rotate left
### ROR: rotate right
### MUL: multiply (unsigned)
### IMUL: multiply (signed)
### DIV: divide (unsigned)
### IDIV: divide (signed)
### REM: calculate remainder of division (unsigned)
### IREM: remainder (signed)
### NOP: no operation
### MOV: move value
### MOVZ: move value and clear upper bits in target register
### RTA: calculate address relative to instruction pointer

## Stack instructions

### PUSH: push value to stack
### POP: pop value from stack

## Control flow instructions

### JMP: absolute jump
### RJMP: relative jump
### LOOP: absolute loop
### RLOOP: relative loop
### CALL: absolute call
### RCALL: relative call
### RET: return from function
### RETI: return from interrupt
### ISE: set interrupt enable flag
### ICL: clear interrupt enable flag
### BRK: debug breakpoint
### INT: raise interrupt
### HALT: halt CPU

## MMU instructions

### MSE: set MMU enable flag
### MCL: clear MMU enable flag
### FLP: flush page from TLB
### TLB: flush TLB and set page directory pointer

## I/O instructions

### IN: get input from I/O bus
### OUT: output on I/O bus
