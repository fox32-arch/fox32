# Fox32 CPU

This document aims to describe the CPU in the the Fox32 architecture.
Peripherals such as the disk controller are described elsewhere.


## Endianness

All 16-bit or 32-bit values are stored in memory in little-endian order.


## Registers

The Fox32 CPU has the following registers:

- **r0-r31**: 32-bit general-purpose registers
- **rsp**: current stack pointer
- **resp**: exception stack pointer
- **rfp**: frame pointer
- **rip**: instruction pointer
- condition flags, which are updated after some operations
  - **zero flag**
  - **carry flag**
- other flags
  - enable use of exception stack pointer (**swap sp**)
  - **interrupt flag**, to enable interrupt handling
- MMU state
  - **MMU enabled** flag
  - **page directory pointer**


## External buses

There are two kinds of external bus that the Fox32 CPU can address:

- **Memory**: Data is read from and written to memory with the `mov` instruction,
  instructions are fetch from memory.
- **I/O bus**: Peripherals are connected to the I/O bus. Peripheral registers
  can be read using the `in` instruction and written using the `out` instruction.


## Instruction encoding

All instructions start with a 16-bit control word, which is optionally followed
by a source operand, or by source and target operands, depending on the opcode.


| bits   | name   | description
|--------|--------|---------------------------------------------------
|  1:0   | source | source operand type
|  3:2   | target | target operand type
|  6:4   | cond   | condition code
|   7    | ---    | reserved, must be zero
| 13:8   | opcode | operation code/type, e.g. `mov` or `add`
| 15:14  | size   | operation size, e.g. 32 bits


### Operand types

| value | description           | size of operand  | what's actually stored?
|-------|-----------------------|------------------|--------------------------
|  0    | register              | 8 bits           | register number
|  1    | register pointer      | 8 bits           | register number
|  2    | immediate             | operation size   | value
|  3    | immediate pointer     | 32 bits          | pointer to memory location


### Register numbers

| value | register
|-------|---------------------------------------------------------------------
| 0-31  | r0-r31
| 32    | rsp
| 33    | resp
| 34    | rfp



### Condition codes

| value | name   | description
|-------|--------|-----------------------------------------------------
|  0    | always | execute unconditionally
|  1    | ifz    | execute if zero flag is set
|  2    | ifnz   | execute if zero flag is not set
|  3    | ifc    | execute if carry flag is set
|  4    | ifnc   | execute if carry flag is not set
|  5    | ifgt   | execute if neither zero flag nor carry flag is set
|  6    | iflteq | execute if zero flag or carry flag is set


### Operation codes

| value | name   | operands | op sizes| description
|-------|--------|----------|---------|-----------------------------------------------------
| 0x00  | NOP    | none     | 8/16/32 | no operation
| 0x01  | ADD    | src+tgt  | 8/16/32 | add
| 0x02  | MUL    | src+tgt  | 8/16/32 | multiply (unsigned)
| 0x03  | AND    | src+tgt  | 8/16/32 | bitwise AND
| 0x04  | SLA    | src+tgt  | 8/16/32 | shift left
| 0x05  | SRA    | src+tgt  | 8/16/32 | shift right arithmetic (with sign extension)
| 0x06  | BSE    | src+tgt  | 8/16/32 | bit set
| 0x07  | CMP    | src+tgt  | 8/16/32 | compare
| 0x08  | JMP    | src      |      32 | absolute jump
| 0x09  | RJMP   | src      |      32 | relative jump
| 0x0A  | PUSH   | src      | 8/16/32 | push value to stack
| 0x0B  | IN     | src+tgt  |      32 | get input from I/O bus
| 0x0C  | ISE    | none     |      32 | set interrupt enable flag
| 0x0D  | MSE    | none     |      32 | set MMU enable flag
| 0x10  | HALT   | none     | 8/16/32 | halt CPU
| 0x11  | INC    | src      | 8/16/32 | increment (add 1)
| 0x13  | OR     | src+tgt  | 8/16/32 | bitwise OR
| 0x14  | IMUL   | src+tgt  | 8/16/32 | multiply (signed)
| 0x15  | SRL    | src+tgt  | 8/16/32 | shift right logical (with zero extension)
| 0x16  | BCL    | src+tgt  | 8/16/32 | bit clear
| 0x17  | MOV    | src+tgt  | 8/16/32 | move value
| 0x18  | CALL   | src      |      32 | absolute call
| 0x19  | RCALL  | src      |      32 | relative call
| 0x1A  | POP    | src      | 8/16/32 | pop value from stack
| 0x1B  | OUT    | src+tgt  |      32 | output on I/O bus
| 0x1C  | ICL    | none     |      32 | clear interrupt enable flag
| 0x1D  | MCL    | none     |      32 | clear MMU enable flag
| 0x20  | BRK    | none     | 8/16/32 | debug breakpoint
| 0x21  | SUB    | src+tgt  | 8/16/32 | subtract
| 0x22  | DIV    | src+tgt  | 8/16/32 | divide (unsigned)
| 0x23  | XOR    | src+tgt  | 8/16/32 | bitwise XOR
| 0x24  | ROL    | src+tgt  | 8/16/32 | rotate left
| 0x25  | ROR    | src+tgt  | 8/16/32 | rotate right
| 0x26  | BTS    | src+tgt  | 8/16/32 | test if bit set
| 0x27  | MOVZ   | src+tgt  | 8/16/32 | move value and clear upper bits in target register
| 0x28  | LOOP   | src      |      32 | absolute loop
| 0x29  | RLOOP  | src      |      32 | relative loop
| 0x2A  | RET    | none     |      32 | return from function
| 0x2C  | INT    | src      |      32 | raise interrupt
| 0x2D  | TLB    | src      |      32 | flush TLB and set page directory pointer
| 0x31  | DEC    | src      | 8/16/32 | decrement (subtract 1)
| 0x32  | REM    | src+tgt  | 8/16/32 | calculate remainder of division (unsigned)
| 0x33  | NOT    | src      | 8/16/32 | bitwise NOT
| 0x34  | IDIV   | src+tgt  | 8/16/32 | divide (signed)
| 0x35  | IREM   | src+tgt  | 8/16/32 | remainder (signed)
| 0x39  | RTA    | src+tgt  |      32 | calculate address relative to instruction pointer
| 0x3A  | RETI   | none     |      32 | return from interrupt
| 0x3D  | FLP    | src      |      32 | flush page from TLB


### Operation sizes

| value | description
|-------|--------------------------------------------------------------
|  0    | byte (8 bits)
|  1    | half (16 bits)
|  2    | word (32 bits)
|  3    | reserved


## Interrupts and Exceptions

Interrupts indicate asynchronous hardware events (such as VSYNC) or execution
of the `int` instruction, while exceptions indicate various synchronous errors.

There are 0x100 interrupt vectors and 5 exception vectors. Interrupt vectors
are at 0x000 to 0x3FC, and exception vectors at 0x400 to 0x410. These memory
locations simply store the address of the interrupt/exception handler,
or 0x0 when no handler has been installed.

(TODO: what should the hardware do when a handler is missing?)

| type      | vector   | description
|-----------|----------|-------------------------------------------
| interrupt | 0 - 0xF0 | free for software use
| interrupt | 0xFB     | audio channel 3 refill
| interrupt | 0xFC     | audio channel 2 refill
| interrupt | 0xFD     | audio channel 1 refill
| interrupt | 0xFE     | audio channel 0 refill
| interrupt | 0xFF     | display VSYNC
| exception | 0x00     | divide by zero
| exception | 0x01     | invalid opcode
| exception | 0x02     | page fault during read
| exception | 0x03     | page fault during write
| exception | 0x04     | breakpoint


Upon interrupt/exception entry, the CPU performs the following operations:

- read handler address from vector
- if *swap sp* is enabled:
  - switch to the exception stack pointer, and push the old stack pointer
- push current instruction pointer
- push flags (8 bits)
- push interrupt vector (0x00-0xff) or exception operand
- clear interrupt flag and *swap sp* flag
- jump to handler


Interrupt/exception handlers are exited through the `reti` instruction, which
performs the following operations:

- pop and restore flags
- pop instruction pointer
- if *swap sp* flag is set, pop stack pointer


The flags are stored in the following format:

| bit    | description
|--------|-----------------------------------------------
| 0      | zero flag
| 1      | carry flag
| 2      | interrupt flag
| 3      | *swap sp* flag


## MMU

If the MMU is enabled, two-level page tables are used to translate virtual
addresses to physical addresses.

The address of the page directory can be set using the `tlb` instruction.

| virtual address bits  | purpose
|-----------------------|--------------------------------------------------
|  11:0                 | lowest 12 bits of physical address
|  21:12                | page table index
|  31:22                | page directory index


Page directories and page tables are arrays of 1024 elements of the following format:

| bits  | bit mask  ts  | purpose
|-------|---------------|--------------------------------------------------
| 31:12 | 0xfffff000    | address of page table or page
| 1     | 0x00000002    | page is writable
| 0     | 0x00000001    | page table or page is present


A page table walk is performed as follows:

- read page directory entry at page directory index in page directory
- abort if page table is not present
- read page table entry at page table index in page table
- abort of page is not present
- use physical page address and writability information from page table entry
