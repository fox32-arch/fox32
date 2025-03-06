# Encoding
```
size instr  off cond dest src <src>          <srcoff> <dest>         <dstoff>
xx   xxxxxx x   xxx  xx   xx  <8,16,32 bits> <8 bits> <8,16,32 bits> <8 bits>
```

# Size table
If the instruction doesn't allow variable sizes or a size was not specified, set the size bits to Word (0b10)
|      |                            |
| :--: | -------------------------- |
| 0b00 | byte/single (8/32 bits)    |
| 0b01 | half/double (16/64 bits)   |
| 0b10 | word/quad (32/128 bits)    |

# Instruction table
| 0x  | -0               | -1               | -2               | -3            | -4                    | -5                    | -6               | -7                    | -8               | -9              | -A                | -B               | -C               | -D  | -E                | -F                |
| :-: | ---------------- | ---------------- | ---------------- | ------------- | --------------------- | --------------------- | ---------------- | --------------------- | ---------------- | --------------- | ----------------- | ---------------- | ---------------- | --- | ----------------- | ----------------- |
| 0-  | NOP              | ADD[.8,16,32]    | MUL[.8,16,32]    | AND[.8,16,32] | SLA[.8,16,32]         | SRA[.8,16,32]         | BSE[.8,16,32]    | CMP[.8,16,32]         | JMP              | RJMP[.8,16,32]  | PUSH[.8,16,32]    | IN               | ISE              | MSE | FPUSH[.32,64,128] | FIREM[.32,64,128] |
| 1-  | HALT             | INC[.8,16,32]    | FSUB[.32,64,128] | OR[.8,16,32]  | IMUL[.8,16,32]        | SRL[.8,16,32]         | BCL[.8,16,32]    | MOV[.8,16,32]         | CALL             | RCALL[.8,16,32] | POP[.8,16,32]     | OUT              | ICL              | MCL | FPOP[.32,64,128]  | FICMP[.32,64,128] |
| 2-  | BRK              | SUB[.8,16,32]    | DIV[.8,16,32]    | XOR[.8,16,32] | ROL[.8,16,32]         | ROR[.8,16,32]         | BTS[.8,16,32]    | MOVZ[.8,16,32]        | LOOP             | RLOOP[.8,16,32] | RET               | FREM[.32,64,128] | INT              | TLB | FIMUL[.32,64,128] |                   |
| 3-  | FADD[.32,64,128] | DEC[.8,16,32]    | REM[.8,16,32]    | NOT[.8,16,32] | IDIV[.8,16,32]        | IREM[.8,16,32]        | FMUL[.32,64,128] | ICMP[.8,16,32]        | FDIV[.32,64,128] | RTA[.8,16,32]   | RETI              | FCMP[.32,64,128] | FMOV[.32,64,128] | FLP | FIDIV[.32,64,128] |                   |

# Condition table
|       |        |                                               |
| :---: | ------ | --------------------------------------------- |
| 0b000 | ---    | always                                        |
| 0b001 | IFZ    | zero                                          |
| 0b010 | IFNZ   | not zero                                      |
| 0b011 | IFC    | carry                                         |
| 0b011 | IFLT   | less than (equivalent to IFC)                 |
| 0b100 | IFNC   | not carry                                     |
| 0b100 | IFGTEQ | greater than or equal to (equivalent to IFNC) |
| 0b101 | IFGT   | greater than                                  |
| 0b110 | IFLTEQ | less than or equal to                         |

# Source/Destination table
|      |                     |
| :--: | ------------------- |
| 0b00 | register            |
| 0b01 | register (pointer)  |
| 0b10 | immediate           |
| 0b11 | immediate (pointer) |
Using an immediate (0b10) as the destination type is only allowed in a few cases, such as with the `OUT` instruction. Using it with instructions where it doesn't make sense (such as doing `mov 0, 1` for example) will throw an invalid opcode exception.

# Register Pointer Offset
The off field indicates that each operand of type 0b01 (register pointer) has an 8-bit immediate following the operand.  This immediate is added to the value of the register before dereferencing.
