# Encoding
```
size instr  . cond dest src <src>          <dest>
xx   xxxxxx 0 xxx  xx   xx  <8,16,32 bits> <8,16,32 bits>
```


# Size table
If the instruction doesn't allow variable sizes or a size was not specified, set the size bits to Word (0b10)
|      |                |
| :--: | -------------- |
| 0b00 | byte (8 bits)  |
| 0b01 | half (16 bits) |
| 0b10 | word (32 bits) |

# Instruction table
| 0x  | -0   | -1            | -2            | -3            | -4            | -5            | -6            | -7             | -8   | -9    | -A             | -B  | -C  | -D  | -E  | -F  |
| :-: | ---- | ------------- | ------------- | ------------- | ------------- | ------------- | ------------- | -------------- | ---- | ----- | -------------- | --- | --- | --- | --- | --- |
| 0-  | NOP  | ADD[.8,16,32] | MUL[.8,16,32] | AND[.8,16,32] | SLA[.8,16,32] | SRA[.8,16,32] | BSE[.8,16,32] | CMP[.8,16,32]  | JMP  | RJMP  | PUSH[.8,16,32] | IN  | ISE |     |     |     |
| 1-  | HALT | INC[.8,16,32] |               | OR[.8,16,32]  |               | SRL[.8,16,32] | BCL[.8,16,32] | MOV[.8,16,32]  | CALL | RCALL | POP[.8,16,32]  | OUT | ICL |     |     |     |
| 2-  | BRK  | SUB[.8,16,32] | DIV[.8,16,32] | XOR[.8,16,32] | ROL[.8,16,32] | ROR[.8,16,32] | BTS[.8,16,32] | MOVZ[.8,16,32] | LOOP | RLOOP | RET            |     | INT |     |     |     |
| 3-  |      | DEC[.8,16,32] | REM[.8,16,32] | NOT[.8,16,32] |               |               |               |                |      | RTA   | RETI           |     |     |     |     |     |

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

# Destination table
|      |                     |
| :--: | ------------------- |
| 0b00 | register            |
| 0b01 | register (pointer)  |
| 0b10 | (invalid)           |
| 0b11 | immediate (pointer) |

# Source table
|      |                     |
| :--: | ------------------- |
| 0b00 | register            |
| 0b01 | register (pointer)  |
| 0b10 | immediate           |
| 0b11 | immediate (pointer) |
