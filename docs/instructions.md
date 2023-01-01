# Fox32 instructions

This file describes every fox32 instruction in detail. For a general
description of the fox32 CPU, see cpu.md.

## NOP: no operation
## ADD: add
## MUL: multiply (unsigned)
## AND: bitwise AND
## SLA: shift left
## SRA: shift right arithmetic (with sign extension)
## BSE: bit set
## CMP: compare
## JMP: absolute jump
## RJMP: relative jump
## PUSH: push value to stack
## IN: get input from I/O bus
## ISE: set interrupt enable flag
## MSE: set MMU enable flag
## HALT: halt CPU
## INC: increment (add 1)
## OR: bitwise OR
## IMUL: multiply (signed)
## SRL: shift right logical (with zero extension)
## BCL: bit clear
## MOV: move value
## CALL: absolute call
## RCALL: relative call
## POP: pop value from stack
## OUT: output on I/O bus
## ICL: clear interrupt enable flag
## MCL: clear MMU enable flag
## BRK: debug breakpoint
## SUB: subtract
## DIV: divide (unsigned)
## XOR: bitwise XOR
## ROL: rotate left
## ROR: rotate right
## BTS: test if bit set
## MOVZ: move value and clear upper bits in target register
## LOOP: absolute loop
## RLOOP: relative loop
## RET: return from function
## INT: raise interrupt
## TLB: flush TLB and set page directory pointer
## DEC: decrement (subtract 1)
## REM: calculate remainder of division (unsigned)
## NOT: bitwise NOT
## IDIV: divide (signed)
## IREM: remainder (signed)
## RTA: calculate address relative to instruction pointer
## RETI: return from interrupt
## FLP: flush page from TLB
