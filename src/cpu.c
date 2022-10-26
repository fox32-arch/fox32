#include <stdio.h>
#include <stdnoreturn.h>
#include <string.h>
#include <setjmp.h>

#include "cpu.h"
#include "mmu.h"

typedef fox32_err_t err_t;

static const char *const err_messages[] = {
    "",
    "internal error",
    "breakpoint reached",
    "fault while reading memory",
    "fault while writing memory",
    "invalid opcode",
    "invalid condition",
    "invalid register",
    "write to immediate",
    "division by zero",
    "io read failed",
    "io write failed",
    "interrupts disabled",
    "error is not recoverable"
};

static const char *err_tostring(err_t err) {
    if (err > 0 && err <= FOX32_ERR_CANTRECOVER) {
        return err_messages[err];
    }
    return err_messages[FOX32_ERR_OK];
}

typedef fox32_io_read_t io_read_t;
typedef fox32_io_write_t io_write_t;

static int io_read_default_impl(void *user, uint32_t *value, uint32_t port) {
    return (void) user, (void) value, (int) port;
}
static int io_write_default_impl(void *user, uint32_t value, uint32_t port) {
    if (port == 0) {
        putchar((int) value);
        fflush(stdout);
    }
    return (void) user, (int) port;
}

static io_read_t *const io_read_default = io_read_default_impl;
static io_write_t *const io_write_default = io_write_default_impl;

enum {
    OP_NOP   = 0x00,
    OP_ADD   = 0x01,
    OP_MUL   = 0x02,
    OP_AND   = 0x03,
    OP_SLA   = 0x04,
    OP_SRA   = 0x05,
    OP_BSE   = 0x06,
    OP_CMP   = 0x07,
    OP_JMP   = 0x08,
    OP_RJMP  = 0x09,
    OP_PUSH  = 0x0A,
    OP_IN    = 0x0B,
    OP_ISE   = 0x0C,
    OP_MSE   = 0x0D,
    OP_HALT  = 0x10,
    OP_INC   = 0x11,
    OP_OR    = 0x13,
    OP_IMUL  = 0x14,
    OP_SRL   = 0x15,
    OP_BCL   = 0x16,
    OP_MOV   = 0x17,
    OP_CALL  = 0x18,
    OP_RCALL = 0x19,
    OP_POP   = 0x1A,
    OP_OUT   = 0x1B,
    OP_ICL   = 0x1C,
    OP_MCL   = 0x1D,
    OP_BRK   = 0x20,
    OP_SUB   = 0x21,
    OP_DIV   = 0x22,
    OP_XOR   = 0x23,
    OP_ROL   = 0x24,
    OP_ROR   = 0x25,
    OP_BTS   = 0x26,
    OP_MOVZ  = 0x27,
    OP_LOOP  = 0x28,
    OP_RLOOP = 0x29,
    OP_RET   = 0x2A,
    OP_TLB   = 0x2D,
    OP_DEC   = 0x31,
    OP_REM   = 0x32,
    OP_NOT   = 0x33,
    OP_IDIV  = 0x34,
    OP_IREM  = 0x35,
    OP_RTA   = 0x39,
    OP_RETI  = 0x3A,
    OP_FLP   = 0x3D
};

enum {
    SZ_BYTE,
    SZ_HALF,
    SZ_WORD
};

#define OP(_size, _optype) (((uint8_t) (_optype)) | (((uint8_t) (_size)) << 6))

enum {
    CD_ALWAYS,
    CD_IFZ,
    CD_IFNZ,
    CD_IFC,
    CD_IFNC,
    CD_IFGT,
    CD_IFLTEQ
};

enum {
    TY_REG,
    TY_REGPTR,
    TY_IMM,
    TY_IMMPTR,
    TY_NONE
};

enum {
    EX_DIVZERO  = 256 + 0x00,
    EX_ILLEGAL  = 256 + 0x01,
    EX_FAULT_RD = 256 + 0x02,
    EX_FAULT_WR = 256 + 0x03,
    EX_DEBUGGER = 256 + 0x04,
    EX_BUS      = 256 + 0x05
};

#define SIZE8 1
#define SIZE16 2
#define SIZE32 4

static uint8_t ptr_get8(const void *ptr) {
    return *((const uint8_t *) ptr);
}
static uint16_t ptr_get16(const void *ptr) {
    const uint8_t *bytes = ptr;
    return
        (((uint16_t) bytes[0])) |
        (((uint16_t) bytes[1]) << 8);
}
static uint32_t ptr_get32(const void *ptr) {
    const uint8_t *bytes = ptr;
    return
        (((uint32_t) bytes[0])) |
        (((uint32_t) bytes[1]) <<  8) |
        (((uint32_t) bytes[2]) << 16) |
        (((uint32_t) bytes[3]) << 24);
}
static void ptr_set8(void *ptr, uint8_t value) {
    *((uint8_t *) ptr) = value;
}
static void ptr_set16(void *ptr, uint16_t value) {
    uint8_t *bytes = ptr;
    bytes[0] = (uint8_t) (value);
    bytes[1] = (uint8_t) (value >> 8);
}
static void ptr_set32(void *ptr, uint32_t value) {
    uint8_t *bytes = ptr;
    bytes[0] = (uint8_t) (value);
    bytes[1] = (uint8_t) (value >>  8);
    bytes[2] = (uint8_t) (value >> 16);
    bytes[3] = (uint8_t) (value >> 24);
}

typedef struct {
    uint8_t opcode;
    uint8_t condition;
    uint8_t target;
    uint8_t source;
    uint8_t size;
} asm_instr_t;

static asm_instr_t asm_instr_from(uint16_t half) {
    asm_instr_t instr = {
        (half >>  8),
        (half >>  4) & 7,
        (half >>  2) & 3,
        (half      ) & 3,
        (half >> 14)
    };
    return instr;
}

typedef struct {
    const char *const name;
    const unsigned int prcount;
} asm_iinfo_t;

static const asm_iinfo_t asm_iinfo_unknown = { "ERROR", 0 };
static const asm_iinfo_t asm_iinfos[256] = {
    [OP_NOP  ] = { "NOP  ", 0 },
    [OP_ADD  ] = { "ADD  ", 2 },
    [OP_MUL  ] = { "MUL  ", 2 },
    [OP_AND  ] = { "AND  ", 2 },
    [OP_SLA  ] = { "SLA  ", 2 },
    [OP_SRA  ] = { "SRA  ", 2 },
    [OP_BSE  ] = { "BSE  ", 2 },
    [OP_CMP  ] = { "CMP  ", 2 },
    [OP_JMP  ] = { "JMP  ", 1 },
    [OP_RJMP ] = { "RJMP ", 1 },
    [OP_PUSH ] = { "PUSH ", 1 },
    [OP_IN   ] = { "IN   ", 2 },
    [OP_ISE  ] = { "ISE  ", 0 },
    [OP_MSE  ] = { "MSE  ", 0 },
    [OP_HALT ] = { "HALT ", 0 },
    [OP_INC  ] = { "INC  ", 1 },
    [OP_OR   ] = { "OR   ", 2 },
    [OP_IMUL ] = { "IMUL ", 2 },
    [OP_SRL  ] = { "SRL  ", 2 },
    [OP_BCL  ] = { "BCL  ", 2 },
    [OP_MOV  ] = { "MOV  ", 2 },
    [OP_CALL ] = { "CALL ", 1 },
    [OP_RCALL] = { "RCALL", 1 },
    [OP_POP  ] = { "POP  ", 1 },
    [OP_OUT  ] = { "OUT  ", 2 },
    [OP_ICL  ] = { "ICL  ", 0 },
    [OP_MCL  ] = { "MCL  ", 0 },
    [OP_BRK  ] = { "BRK  ", 0 },
    [OP_SUB  ] = { "SUB  ", 2 },
    [OP_DIV  ] = { "DIV  ", 2 },
    [OP_XOR  ] = { "XOR  ", 2 },
    [OP_ROL  ] = { "ROL  ", 2 },
    [OP_ROR  ] = { "ROR  ", 2 },
    [OP_BTS  ] = { "BTS  ", 2 },
    [OP_MOVZ ] = { "MOVZ ", 2 },
    [OP_LOOP ] = { "LOOP ", 1 },
    [OP_RLOOP] = { "RLOOP", 1 },
    [OP_RET  ] = { "RET  ", 0 },
    [OP_TLB  ] = { "TLB  ", 1 },
    [OP_DEC  ] = { "DEC  ", 1 },
    [OP_REM  ] = { "REM  ", 2 },
    [OP_NOT  ] = { "NOT  ", 1 },
    [OP_IDIV ] = { "IDIV ", 2 },
    [OP_IREM ] = { "IREM ", 2 },
    [OP_RTA  ] = { "RTA  ", 2 },
    [OP_RETI ] = { "RETI ", 0 },
    [OP_FLP  ] = { "FLP  ", 0 }
};

static const asm_iinfo_t *asm_iinfo_get(uint8_t opcode) {
    const asm_iinfo_t *iinfo = &asm_iinfos[opcode & 0x3F];
    if (iinfo->name == NULL) {
        iinfo = &asm_iinfo_unknown;
    }
    return iinfo;
}

static uint32_t asm_disas_prsize(asm_instr_t instr, uint8_t prtype) {
    if (prtype < TY_IMM) return SIZE8;
    if (prtype < TY_IMMPTR) return SIZE32;
    switch (instr.size) {
        case SZ_BYTE: return SIZE8;
        case SZ_HALF: return SIZE16;
        case SZ_WORD: return SIZE32;
    }
    return 0;
}

static uint32_t asm_disas_paramssize(asm_instr_t instr, const asm_iinfo_t *iinfo) {
    uint32_t size = 0;
    if (iinfo->prcount > 0) size += asm_disas_prsize(instr, instr.source);
    if (iinfo->prcount > 1) size += asm_disas_prsize(instr, instr.target);
    return size;
}

static const char *const asm_disas_strssize[4] = {
    "BYTE",
    "HALF",
    "WORD",
    "????"
};

static const char *const asm_disas_strscondition[8] = {
    "      ",
    "IFZ   ",
    "IFNZ  ",
    "IFC   ",
    "IFNC  ",
    "IFGT  ",
    "IFLTEQ",
    "??????"
};

static const char *const asm_disas_strslocal[36] = {
    "R0 ", "R1 ", "R2 ", "R3 ", "R4 ", "R5 ", "R6 ", "R7 ",
    "R8 ", "R9 ", "R10", "R11", "R12", "R13", "R14", "R15",
    "R16", "R17", "R18", "R19", "R20", "R21", "R22", "R23",
    "R24", "R25", "R26", "R27", "R28", "R29", "R30", "R31",
    "RSP", "ESP", "RFP", "???"
};

static void asm_disas_printparam(asm_instr_t instr, const uint8_t **paramsdata, char **buffer, uint8_t prtype) {
    int count = 0;

    if (prtype < TY_IMM) {
        uint8_t local = ptr_get8(*paramsdata);
        *paramsdata += SIZE8;

        const char* str_local = asm_disas_strslocal[35];
        if (local < 35) {
            str_local = asm_disas_strslocal[local];
        }

        count = sprintf(*buffer, "%s %s     ", prtype == TY_REG ? "REG   " : "REGPTR", str_local);
    } else {
        uint32_t value = 0;
        switch (instr.size) {
            case SZ_BYTE: value = (uint32_t) ptr_get8 (*paramsdata); *paramsdata += SIZE8 ; break;
            case SZ_HALF: value = (uint32_t) ptr_get16(*paramsdata); *paramsdata += SIZE16; break;
            case SZ_WORD: value =            ptr_get32(*paramsdata); *paramsdata += SIZE32; break;
        }

        count = sprintf(*buffer, "%s %08X", prtype == TY_IMM ? "IMM   " : "IMMPTR", value);
    }

    if (count > 0) {
        *buffer += (size_t) count;
    }
}

static void asm_disas_print(asm_instr_t instr, const asm_iinfo_t *iinfo, const uint8_t *paramsdata, char *buffer) {
    const char *str_size = asm_disas_strssize[3];
    if (instr.size < 3) {
        str_size = asm_disas_strssize[instr.size];
    }
    const char *str_condition = asm_disas_strscondition[7];
    if (instr.condition < 8) {
        str_condition = asm_disas_strscondition[instr.condition];
    }

    int count = sprintf(buffer, "%s %s %s", str_condition, str_size, iinfo->name);
    if (count > 0) {
        buffer += (size_t) count;
    }

    if (iinfo->prcount > 0) {
        *buffer++ = ' ';
        asm_disas_printparam(instr, &paramsdata, &buffer, instr.source);
    }
    if (iinfo->prcount > 1) {
        *buffer++ = ',';
        *buffer++ = ' ';
        asm_disas_printparam(instr, &paramsdata, &buffer, instr.target);
    }
}

typedef fox32_vm_t vm_t;

static void vm_init(vm_t *vm) {
    memset(vm, 0, sizeof(vm_t));
    vm->pointer_instr = FOX32_POINTER_DEFAULT_INSTR;
    vm->pointer_stack = FOX32_POINTER_DEFAULT_STACK;
    vm->halted = true;
    vm->mmu_enabled = false;
    vm->io_user = NULL;
    vm->io_read = io_read_default;
    vm->io_write = io_write_default;
}

static noreturn void vm_panic(vm_t *vm, err_t err) {
    longjmp(vm->panic_jmp, (vm->panic_err = err, 1));
}
static noreturn void vm_unreachable(vm_t *vm) {
    vm_panic(vm, FOX32_ERR_INTERNAL);
}

static uint32_t vm_io_read(vm_t *vm, uint32_t port) {
    uint32_t value = 0;
    int status = vm->io_read(vm->io_user, &value, port);
    if (status != 0) {
        vm_panic(vm, FOX32_ERR_IOREAD);
    }
    return value;
}
static void vm_io_write(vm_t *vm, uint32_t port, uint32_t value) {
    int status = vm->io_write(vm->io_user, value, port);
    if (status != 0) {
        vm_panic(vm, FOX32_ERR_IOWRITE);
    }
}

static uint8_t vm_flags_get(vm_t *vm) {
    return (((uint8_t) vm->flag_swap_sp) << 3) |
           (((uint8_t) vm->flag_interrupt) << 2) |
           (((uint8_t) vm->flag_carry) << 1) |
           ((uint8_t) vm->flag_zero);
}
static void vm_flags_set(vm_t *vm, uint8_t flags) {
    vm->flag_zero = (flags & 1) != 0;
    vm->flag_carry = (flags & 2) != 0;
    vm->flag_interrupt = (flags & 4) != 0;
    vm->flag_swap_sp = (flags & 8) != 0;
}

static uint32_t *vm_findlocal(vm_t *vm, uint8_t local) {
    if (local < FOX32_REGISTER_COUNT) {
        return &vm->registers[local];
    }
    if (local == FOX32_REGISTER_COUNT) {
        return &vm->pointer_stack;
    }
    if (local == FOX32_REGISTER_COUNT + 1) {
        return &vm->pointer_exception_stack;
    }
    if (local == FOX32_REGISTER_COUNT + 2) {
        return &vm->pointer_frame;
    }
    vm_panic(vm, FOX32_ERR_BADREGISTER);
}

static uint8_t *vm_findmemory(vm_t *vm, uint32_t address, uint32_t size, bool write) {
    uint32_t address_end = address + size;
    if (!vm->mmu_enabled) {
        if (address_end > address) {
            if (address_end <= FOX32_MEMORY_RAM) {
                return &vm->memory_ram[address];
            }
            if (
                !write &&
                (address >= FOX32_MEMORY_ROM_START) &&
                (address -= FOX32_MEMORY_ROM_START) + size <= FOX32_MEMORY_ROM
            ) {
                return &vm->memory_rom[address];
            }
        }
        if (!write) {
            vm->exception_operand = address;
            vm_panic(vm, FOX32_ERR_FAULT_RD);
        } else {
            vm->exception_operand = address;
            vm_panic(vm, FOX32_ERR_FAULT_WR);
        }
    } else {
        mmu_page_t *virtual_page = get_present_page(address);
        if (virtual_page == NULL) {
            if (!write) {
                vm->exception_operand = address;
                vm_panic(vm, FOX32_ERR_FAULT_RD);
            } else {
                vm->exception_operand = address;
                vm_panic(vm, FOX32_ERR_FAULT_WR);
            }
        }
        uint32_t offset = address & 0x00000FFF;
        uint32_t physical_address = virtual_page->physical_address | offset;
        address_end = physical_address + size;
        if (address_end > physical_address) {
            if (address_end <= FOX32_MEMORY_RAM) {
                return &vm->memory_ram[physical_address];
            }
            if (
                !write &&
                (physical_address >= FOX32_MEMORY_ROM_START) &&
                (physical_address -= FOX32_MEMORY_ROM_START) + size <= FOX32_MEMORY_ROM
            ) {
                return &vm->memory_rom[physical_address];
            }
        }
        if (!write) {
            vm->exception_operand = address;
            vm_panic(vm, FOX32_ERR_FAULT_RD);
        } else {
            vm->exception_operand = address;
            vm_panic(vm, FOX32_ERR_FAULT_WR);
        }
    }
}

#define VM_READ_BODY(_ptr_get, _size) \
    return _ptr_get(vm_findmemory(vm, address, _size, false));

static uint8_t vm_read8(vm_t *vm, uint32_t address) {
    VM_READ_BODY(ptr_get8, SIZE8)
}
static uint16_t vm_read16(vm_t *vm, uint32_t address) {
    VM_READ_BODY(ptr_get16, SIZE16)
}
static uint32_t vm_read32(vm_t *vm, uint32_t address) {
    VM_READ_BODY(ptr_get32, SIZE32)
}

#define VM_WRITE_BODY(_ptr_set, _size) \
    _ptr_set(vm_findmemory(vm, address, _size, true), value);

static void vm_write8(vm_t *vm, uint32_t address, uint8_t value) {
    VM_WRITE_BODY(ptr_set8, SIZE8)
}
static void vm_write16(vm_t *vm, uint32_t address, uint16_t value) {
    VM_WRITE_BODY(ptr_set16, SIZE16)
}
static void vm_write32(vm_t *vm, uint32_t address, uint32_t value) {
    VM_WRITE_BODY(ptr_set32, SIZE32)
}

#define VM_PUSH_BODY(_vm_write, _size) \
    _vm_write(vm, vm->pointer_stack -= _size, value);

static void vm_push8(vm_t *vm, uint8_t value) {
    VM_PUSH_BODY(vm_write8, SIZE8)
}
static void vm_push16(vm_t *vm, uint16_t value) {
    VM_PUSH_BODY(vm_write16, SIZE16)
}
static void vm_push32(vm_t *vm, uint32_t value) {
    VM_PUSH_BODY(vm_write32, SIZE32)
}

#define VM_POP_BODY(_vm_read, _size)                 \
    uint32_t pointer_stack_prev = vm->pointer_stack; \
    return _vm_read(vm, (vm->pointer_stack += _size, pointer_stack_prev));

static uint8_t vm_pop8(vm_t *vm) {
    VM_POP_BODY(vm_read8, SIZE8)
}
static uint16_t vm_pop16(vm_t *vm) {
    VM_POP_BODY(vm_read16, SIZE16)
}
static uint32_t vm_pop32(vm_t *vm) {
    VM_POP_BODY(vm_read32, SIZE32)
}

#define VM_SOURCE_BODY(_vm_read, _size, _type, _move)                           \
    uint32_t pointer_base = vm->pointer_instr_mut;                              \
    switch (prtype) {                                                           \
        case TY_REG: {                                                          \
            if (_move) vm->pointer_instr_mut += SIZE8;                          \
            return (_type) *vm_findlocal(vm, vm_read8(vm, pointer_base));       \
        };                                                                      \
        case TY_REGPTR: {                                                       \
            if (_move) vm->pointer_instr_mut += SIZE8;                          \
            return _vm_read(vm, *vm_findlocal(vm, vm_read8(vm, pointer_base))); \
        };                                                                      \
        case TY_IMM: {                                                          \
            if (_move) vm->pointer_instr_mut += _size;                          \
            return _vm_read(vm, pointer_base);                                  \
        };                                                                      \
        case TY_IMMPTR: {                                                       \
            if (_move) vm->pointer_instr_mut += SIZE32;                         \
            return _vm_read(vm, vm_read32(vm, pointer_base));                   \
        };                                                                      \
    }                                                                           \
    vm_unreachable(vm);

static uint8_t vm_source8(vm_t *vm, uint8_t prtype) {
    VM_SOURCE_BODY(vm_read8, SIZE8, uint8_t, true)
}
static uint8_t vm_source8_stay(vm_t *vm, uint8_t prtype) {
    VM_SOURCE_BODY(vm_read8, SIZE8, uint8_t, false)
}
static uint16_t vm_source16(vm_t *vm, uint8_t prtype) {
    VM_SOURCE_BODY(vm_read16, SIZE16, uint16_t, true)
}
static uint16_t vm_source16_stay(vm_t *vm, uint8_t prtype) {
    VM_SOURCE_BODY(vm_read16, SIZE16, uint16_t, false)
}
static uint32_t vm_source32(vm_t *vm, uint8_t prtype) {
    VM_SOURCE_BODY(vm_read32, SIZE32, uint32_t, true)
}
static uint32_t vm_source32_stay(vm_t *vm, uint8_t prtype) {
    VM_SOURCE_BODY(vm_read32, SIZE32, uint32_t, false)
}

#define VM_TARGET_BODY(_vm_write, _localvalue)                                   \
    uint32_t pointer_base = vm->pointer_instr_mut;                               \
    switch (prtype) {                                                            \
        case TY_REG: {                                                           \
            vm->pointer_instr_mut += SIZE8;                                      \
            uint8_t local = vm_read8(vm, pointer_base);                          \
            *vm_findlocal(vm, local) = _localvalue;                              \
            return;                                                              \
        };                                                                       \
        case TY_REGPTR: {                                                        \
            vm->pointer_instr_mut += SIZE8;                                      \
            _vm_write(vm, *vm_findlocal(vm, vm_read8(vm, pointer_base)), value); \
            return;                                                              \
        };                                                                       \
        case TY_IMM: {                                                           \
            vm_panic(vm, FOX32_ERR_BADIMMEDIATE);                                \
            return;                                                              \
        };                                                                       \
        case TY_IMMPTR: {                                                        \
            vm->pointer_instr_mut += SIZE32;                                     \
            _vm_write(vm, vm_read32(vm, pointer_base), value);                   \
            return;                                                              \
        };                                                                       \
    };                                                                           \
    vm_unreachable(vm);

static void vm_target8(vm_t *vm, uint8_t prtype, uint8_t value) {
    VM_TARGET_BODY(vm_write8, (*vm_findlocal(vm, local) & 0xFFFFFF00) | (uint32_t) value)
}
static void vm_target8_zero(vm_t *vm, uint8_t prtype, uint8_t value) {
    VM_TARGET_BODY(vm_write8, (uint32_t) value)
}
static void vm_target16(vm_t *vm, uint8_t prtype, uint16_t value) {
    VM_TARGET_BODY(vm_write16, (*vm_findlocal(vm, local) & 0xFFFF0000) | (uint32_t) value)
}
static void vm_target16_zero(vm_t *vm, uint8_t prtype, uint16_t value) {
    VM_TARGET_BODY(vm_write16, (uint32_t) value)
}
static void vm_target32(vm_t *vm, uint8_t prtype, uint32_t value) {
    VM_TARGET_BODY(vm_write32, value)
}

static bool vm_shouldskip(vm_t *vm, uint8_t condition) {
    switch (condition) {
        case CD_ALWAYS: {
            return false;
        };
        case CD_IFZ: {
            return vm->flag_zero == false;
        };
        case CD_IFNZ: {
            return vm->flag_zero == true;
        };
        case CD_IFC: {
            return vm->flag_carry == false;
        };
        case CD_IFNC: {
            return vm->flag_carry == true;
        };
        case CD_IFGT: {
            return (vm->flag_zero == true) || (vm->flag_carry == true);
        };
        case CD_IFLTEQ: {
            return (vm->flag_zero == false) && (vm->flag_carry == false);
        };
    }
    vm_panic(vm, FOX32_ERR_BADCONDITION);
}

static void vm_skipparam(vm_t *vm, uint32_t size, uint8_t prtype) {
    if (prtype < TY_IMM) {
        vm->pointer_instr_mut += SIZE8;
    } else if (prtype == TY_IMMPTR) {
        vm->pointer_instr_mut += SIZE32;
    } else {
        vm->pointer_instr_mut += size;
    }
}

#define CHECKED_ADD(_a, _b, _out) __builtin_add_overflow(_a, _b, _out)
#define CHECKED_SUB(_a, _b, _out) __builtin_sub_overflow(_a, _b, _out)
#define CHECKED_MUL(_a, _b, _out) __builtin_mul_overflow(_a, _b, _out)

#define OPER_DIV(_a, _b) ((_a) / (_b))
#define OPER_REM(_a, _b) ((_a) % (_b))
#define OPER_AND(_a, _b) ((_a) & (_b))
#define OPER_XOR(_a, _b) ((_a) ^ (_b))
#define OPER_OR(_a, _b) ((_a) | (_b))
#define OPER_SHIFT_LEFT(_a, _b) ((_a) << (_b))
#define OPER_SHIFT_RIGHT(_a, _b) ((_a) >> (_b))
#define OPER_BIT_SET(_a, _b) ((_a) | (1 << (_b)))
#define OPER_BIT_CLEAR(_a, _b) ((_a) & ~(1 << (_b)))

#define ROTATE_LEFT(_size, _a, _b) (((_a) << (_b)) | ((_a) >> (((_size) * 8) - (_b))))
#define ROTATE_LEFT8(_a, _b) ROTATE_LEFT(SIZE8, _a, _b)
#define ROTATE_LEFT16(_a, _b) ROTATE_LEFT(SIZE16, _a, _b)
#define ROTATE_LEFT32(_a, _b) ROTATE_LEFT(SIZE32, _a, _b)
#define ROTATE_RIGHT(_size, _a, _b) (((_a) >> (_b)) | ((_a) << (((_size) * 8) - (_b))))
#define ROTATE_RIGHT8(_a, _b) ROTATE_RIGHT(SIZE8, _a, _b)
#define ROTATE_RIGHT16(_a, _b) ROTATE_RIGHT(SIZE16, _a, _b)
#define ROTATE_RIGHT32(_a, _b) ROTATE_RIGHT(SIZE32, _a, _b)

#define SOURCEMAP_IDENTITY(x) (x)
#define SOURCEMAP_RELATIVE(x) (instr_base + (x))

#define VM_PRELUDE_0() {                      \
    if (vm_shouldskip(vm, instr.condition)) { \
        break;                                \
    }                                         \
}
#define VM_PRELUDE_1(_size) {                  \
    if (vm_shouldskip(vm, instr.condition)) {  \
        vm_skipparam(vm, _size, instr.source); \
        break;                                 \
    }                                          \
}
#define VM_PRELUDE_2(_size) {                  \
    if (vm_shouldskip(vm, instr.condition)) {  \
        vm_skipparam(vm, _size, instr.target); \
        vm_skipparam(vm, _size, instr.source); \
        break;                                 \
    }                                          \
}

#define VM_IMPL_JMP(_sourcemap) {                                      \
    VM_PRELUDE_1(SIZE32);                                              \
    vm->pointer_instr_mut = _sourcemap(vm_source32(vm, instr.source)); \
    break;                                                             \
}

#define VM_IMPL_LOOP(_sourcemap) {                                         \
    if (                                                                   \
        !vm_shouldskip(vm, instr.condition) &&                             \
        (vm->registers[FOX32_REGISTER_LOOP] -= 1) != 0                     \
    ) {                                                                    \
        vm->pointer_instr_mut = _sourcemap(vm_source32(vm, instr.source)); \
    } else {                                                               \
        vm_skipparam(vm, SIZE32, instr.source);                            \
    }                                                                      \
    break;                                                                 \
}

#define VM_IMPL_CALL(_sourcemap) {                         \
    VM_PRELUDE_1(SIZE32);                                  \
    uint32_t pointer_call = vm_source32(vm, instr.source); \
    vm_push32(vm, vm->pointer_instr_mut);                  \
    vm->pointer_instr_mut = _sourcemap(pointer_call);      \
    break;                                                 \
}

#define VM_IMPL_POP(_size, _vm_target, _vm_pop) { \
    VM_PRELUDE_1(_size);                          \
    _vm_target(vm, instr.source, _vm_pop(vm));    \
    break;                                        \
}

#define VM_IMPL_PUSH(_size, _vm_source, _vm_push) { \
    VM_PRELUDE_1(_size);                            \
    _vm_push(vm, _vm_source(vm, instr.source));     \
    break;                                          \
}

#define VM_IMPL_MOV(_size, _vm_source, _vm_target) {            \
    VM_PRELUDE_2(_size);                                        \
    _vm_target(vm, instr.target, _vm_source(vm, instr.source)); \
    break;                                                      \
}

#define VM_IMPL_NOT(_size, _type, _vm_source_stay, _vm_target) { \
    VM_PRELUDE_1(_size);                                         \
    _type v = _vm_source_stay(vm, instr.source);                 \
    _type x = ~v;                                                \
    vm->flag_zero = x == 0;                                      \
    _vm_target(vm, instr.source, x);                             \
    break;                                                       \
}

#define VM_IMPL_INC(_size, _type, _vm_source_stay, _vm_target, _oper) { \
    VM_PRELUDE_1(_size);                                                \
    _type v = _vm_source_stay(vm, instr.source);                        \
    _type x;                                                            \
    vm->flag_carry = _oper(v, 1, &x);                                   \
    vm->flag_zero = x == 0;                                             \
    _vm_target(vm, instr.source, x);                                    \
    break;                                                              \
}

#define VM_IMPL_ADD(_size, _type, _type_target, _vm_source, _vm_source_stay, _vm_target, _oper) { \
    VM_PRELUDE_2(_size);                                                                          \
    _type a = (_type) _vm_source(vm, instr.source);                                               \
    _type b = (_type) _vm_source_stay(vm, instr.target);                                          \
    _type x;                                                                                      \
    vm->flag_carry = _oper(b, a, &x);                                                             \
    vm->flag_zero = x == 0;                                                                       \
    _vm_target(vm, instr.target, (_type_target) x);                                               \
    break;                                                                                        \
}

#define VM_IMPL_AND(_size, _type, _type_target, _vm_source, _vm_source_stay, _vm_target, _oper) { \
    VM_PRELUDE_2(_size);                                                                          \
    _type a = (_type) _vm_source(vm, instr.source);                                               \
    _type b = (_type) _vm_source_stay(vm, instr.target);                                          \
    _type x = _oper(b, a);                                                                        \
    vm->flag_zero = x == 0;                                                                       \
    _vm_target(vm, instr.target, (_type_target) x);                                               \
    break;                                                                                        \
}

#define VM_IMPL_CMP(_size, _type, _vm_source) { \
    VM_PRELUDE_0();                             \
    _type a = _vm_source(vm, instr.source);     \
    _type b = _vm_source(vm, instr.target);     \
    _type x;                                    \
    vm->flag_carry = CHECKED_SUB(b, a, &x);     \
    vm->flag_zero = x == 0;                     \
    break;                                      \
}

#define VM_IMPL_BTS(_size, _type, _vm_source) { \
    VM_PRELUDE_2(_size);                        \
    _type a = _vm_source(vm, instr.source);     \
    _type b = _vm_source(vm, instr.target);     \
    _type x = b & (1 << a);                     \
    vm->flag_zero = x == 0;                     \
    break;                                      \
}

static void vm_debug(vm_t *vm, asm_instr_t instr, uint32_t address) {
    const asm_iinfo_t *iinfo = asm_iinfo_get(instr.opcode);

    uint32_t params_size = asm_disas_paramssize(instr, iinfo);
    uint8_t *params_data = NULL;
    if (params_size > 0) {
        params_data = vm_findmemory(vm, address + SIZE16, params_size, false);
    }

    char buffer[128] = {};
    asm_disas_print(instr, iinfo, params_data, buffer);

    printf("%08X %s\n", address, buffer);
}

static void vm_execute(vm_t *vm) {
    uint32_t instr_base = vm->pointer_instr;
    uint16_t instr_raw = vm_read16(vm, instr_base);

    asm_instr_t instr = asm_instr_from(instr_raw);

    vm->pointer_instr_mut = instr_base + SIZE16;

    if (vm->debug) vm_debug(vm, instr, instr_base);

    switch (instr.opcode) {
        case OP(SZ_BYTE, OP_NOP):
        case OP(SZ_HALF, OP_NOP):
        case OP(SZ_WORD, OP_NOP): {
            break;
        };

        case OP(SZ_BYTE, OP_HALT):
        case OP(SZ_HALF, OP_HALT):
        case OP(SZ_WORD, OP_HALT): {
            VM_PRELUDE_0();
            vm->halted = true;
            break;
        };

        case OP(SZ_BYTE, OP_BRK):
        case OP(SZ_HALF, OP_BRK):
        case OP(SZ_WORD, OP_BRK): {
            VM_PRELUDE_0();
            vm_panic(vm, FOX32_ERR_DEBUGGER);
            break;
        };

        case OP(SZ_WORD, OP_IN): {
            VM_PRELUDE_2(SIZE32);
            vm_target32(vm, instr.target, vm_io_read(vm, vm_source32(vm, instr.source)));
            break;
        };
        case OP(SZ_WORD, OP_OUT): {
            VM_PRELUDE_2(SIZE32);
            uint32_t value = vm_source32(vm, instr.source);
            uint32_t port = vm_source32(vm, instr.target);
            vm_io_write(vm, port, value);
            break;
        };

        case OP(SZ_WORD, OP_RTA): {
            VM_PRELUDE_2(SIZE32);
            vm_target32(vm, instr.target, instr_base + vm_source32(vm, instr.source));
            break;
        };

        case OP(SZ_WORD, OP_RET): {
            VM_PRELUDE_0();
            vm->pointer_instr_mut = vm_pop32(vm);
            break;
        };
        case OP(SZ_WORD, OP_RETI): {
            VM_PRELUDE_0();
            vm_flags_set(vm, vm_pop8(vm));
            vm->pointer_instr_mut = vm_pop32(vm);
            break;
        };

        case OP(SZ_WORD, OP_ISE): {
            VM_PRELUDE_0();
            vm->flag_interrupt = true;
            break;
        };
        case OP(SZ_WORD, OP_ICL): {
            VM_PRELUDE_0();
            vm->flag_interrupt = false;
            break;
        };

        case OP(SZ_WORD, OP_JMP): VM_IMPL_JMP(SOURCEMAP_IDENTITY);
        case OP(SZ_WORD, OP_CALL): VM_IMPL_CALL(SOURCEMAP_IDENTITY);
        case OP(SZ_WORD, OP_LOOP): VM_IMPL_LOOP(SOURCEMAP_IDENTITY);
        case OP(SZ_WORD, OP_RJMP): VM_IMPL_JMP(SOURCEMAP_RELATIVE);
        case OP(SZ_WORD, OP_RCALL): VM_IMPL_CALL(SOURCEMAP_RELATIVE);
        case OP(SZ_WORD, OP_RLOOP): VM_IMPL_LOOP(SOURCEMAP_RELATIVE);

        case OP(SZ_BYTE, OP_POP): VM_IMPL_POP(SIZE8, vm_target8, vm_pop8);
        case OP(SZ_HALF, OP_POP): VM_IMPL_POP(SIZE16, vm_target16, vm_pop16);
        case OP(SZ_WORD, OP_POP): VM_IMPL_POP(SIZE32, vm_target32, vm_pop32);

        case OP(SZ_BYTE, OP_PUSH): VM_IMPL_PUSH(SIZE8, vm_source8, vm_push8);
        case OP(SZ_HALF, OP_PUSH): VM_IMPL_PUSH(SIZE16, vm_source16, vm_push16);
        case OP(SZ_WORD, OP_PUSH): VM_IMPL_PUSH(SIZE32, vm_source32, vm_push32);

        case OP(SZ_BYTE, OP_MOV): VM_IMPL_MOV(SIZE8, vm_source8, vm_target8);
        case OP(SZ_BYTE, OP_MOVZ): VM_IMPL_MOV(SIZE8, vm_source8, vm_target8_zero);
        case OP(SZ_HALF, OP_MOV): VM_IMPL_MOV(SIZE16, vm_source16, vm_target16);
        case OP(SZ_HALF, OP_MOVZ): VM_IMPL_MOV(SIZE16, vm_source16, vm_target16_zero);
        case OP(SZ_WORD, OP_MOV):
        case OP(SZ_WORD, OP_MOVZ): VM_IMPL_MOV(SIZE32, vm_source32, vm_target32);

        case OP(SZ_BYTE, OP_NOT): VM_IMPL_NOT(SIZE8, uint8_t, vm_source8_stay, vm_target8);
        case OP(SZ_HALF, OP_NOT): VM_IMPL_NOT(SIZE16, uint16_t, vm_source16_stay, vm_target16);
        case OP(SZ_WORD, OP_NOT): VM_IMPL_NOT(SIZE32, uint32_t, vm_source32_stay, vm_target32);

        case OP(SZ_BYTE, OP_INC): VM_IMPL_INC(SIZE8, uint8_t, vm_source8_stay, vm_target8, CHECKED_ADD);
        case OP(SZ_HALF, OP_INC): VM_IMPL_INC(SIZE16, uint16_t, vm_source16_stay, vm_target16, CHECKED_ADD);
        case OP(SZ_WORD, OP_INC): VM_IMPL_INC(SIZE32, uint32_t, vm_source32_stay, vm_target32, CHECKED_ADD);
        case OP(SZ_BYTE, OP_DEC): VM_IMPL_INC(SIZE8, uint8_t, vm_source8_stay, vm_target8, CHECKED_SUB);
        case OP(SZ_HALF, OP_DEC): VM_IMPL_INC(SIZE16, uint16_t, vm_source16_stay, vm_target16, CHECKED_SUB);
        case OP(SZ_WORD, OP_DEC): VM_IMPL_INC(SIZE32, uint32_t, vm_source32_stay, vm_target32, CHECKED_SUB);

        case OP(SZ_BYTE, OP_ADD): VM_IMPL_ADD(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, CHECKED_ADD);
        case OP(SZ_HALF, OP_ADD): VM_IMPL_ADD(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, CHECKED_ADD);
        case OP(SZ_WORD, OP_ADD): VM_IMPL_ADD(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, CHECKED_ADD);
        case OP(SZ_BYTE, OP_SUB): VM_IMPL_ADD(SIZE8, uint8_t, uint8_t ,vm_source8, vm_source8_stay, vm_target8, CHECKED_SUB);
        case OP(SZ_HALF, OP_SUB): VM_IMPL_ADD(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, CHECKED_SUB);
        case OP(SZ_WORD, OP_SUB): VM_IMPL_ADD(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, CHECKED_SUB);
        case OP(SZ_BYTE, OP_MUL): VM_IMPL_ADD(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, CHECKED_MUL);
        case OP(SZ_HALF, OP_MUL): VM_IMPL_ADD(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, CHECKED_MUL);
        case OP(SZ_WORD, OP_MUL): VM_IMPL_ADD(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, CHECKED_MUL);
        case OP(SZ_BYTE, OP_IMUL): VM_IMPL_ADD(SIZE8, int8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, CHECKED_MUL);
        case OP(SZ_HALF, OP_IMUL): VM_IMPL_ADD(SIZE16, int16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, CHECKED_MUL);
        case OP(SZ_WORD, OP_IMUL): VM_IMPL_ADD(SIZE32, int32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, CHECKED_MUL);

        case OP(SZ_BYTE, OP_DIV): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_DIV);
        case OP(SZ_HALF, OP_DIV): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_DIV);
        case OP(SZ_WORD, OP_DIV): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_DIV);
        case OP(SZ_BYTE, OP_REM): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_REM);
        case OP(SZ_HALF, OP_REM): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_REM);
        case OP(SZ_WORD, OP_REM): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_REM);
        case OP(SZ_BYTE, OP_IDIV): VM_IMPL_AND(SIZE8, int8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_DIV);
        case OP(SZ_HALF, OP_IDIV): VM_IMPL_AND(SIZE16, int16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_DIV);
        case OP(SZ_WORD, OP_IDIV): VM_IMPL_AND(SIZE32, int32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_DIV);
        case OP(SZ_BYTE, OP_IREM): VM_IMPL_AND(SIZE8, int8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_REM);
        case OP(SZ_HALF, OP_IREM): VM_IMPL_AND(SIZE16, int16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_REM);
        case OP(SZ_WORD, OP_IREM): VM_IMPL_AND(SIZE32, int32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_REM);

        case OP(SZ_BYTE, OP_AND): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_AND);
        case OP(SZ_HALF, OP_AND): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_AND);
        case OP(SZ_WORD, OP_AND): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_AND);
        case OP(SZ_BYTE, OP_XOR): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_XOR);
        case OP(SZ_HALF, OP_XOR): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_XOR);
        case OP(SZ_WORD, OP_XOR): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_XOR);
        case OP(SZ_BYTE, OP_OR): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_OR);
        case OP(SZ_HALF, OP_OR): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_OR);
        case OP(SZ_WORD, OP_OR): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_OR);

        case OP(SZ_BYTE, OP_SLA): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_SHIFT_LEFT);
        case OP(SZ_HALF, OP_SLA): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_SHIFT_LEFT);
        case OP(SZ_WORD, OP_SLA): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_SHIFT_LEFT);
        case OP(SZ_BYTE, OP_SRL): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_SHIFT_RIGHT);
        case OP(SZ_HALF, OP_SRL): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_SHIFT_RIGHT);
        case OP(SZ_WORD, OP_SRL): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_SHIFT_RIGHT);
        case OP(SZ_BYTE, OP_SRA): VM_IMPL_AND(SIZE8, int8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_SHIFT_RIGHT);
        case OP(SZ_HALF, OP_SRA): VM_IMPL_AND(SIZE16, int16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_SHIFT_RIGHT);
        case OP(SZ_WORD, OP_SRA): VM_IMPL_AND(SIZE32, int32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_SHIFT_RIGHT);

        case OP(SZ_BYTE, OP_ROL): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, ROTATE_LEFT8);
        case OP(SZ_HALF, OP_ROL): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, ROTATE_LEFT16);
        case OP(SZ_WORD, OP_ROL): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, ROTATE_LEFT32);
        case OP(SZ_BYTE, OP_ROR): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, ROTATE_RIGHT8);
        case OP(SZ_HALF, OP_ROR): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, ROTATE_RIGHT16);
        case OP(SZ_WORD, OP_ROR): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, ROTATE_RIGHT32);

        case OP(SZ_BYTE, OP_BSE): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_BIT_SET);
        case OP(SZ_HALF, OP_BSE): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_BIT_SET);
        case OP(SZ_WORD, OP_BSE): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_BIT_SET);
        case OP(SZ_BYTE, OP_BCL): VM_IMPL_AND(SIZE8, uint8_t, uint8_t, vm_source8, vm_source8_stay, vm_target8, OPER_BIT_CLEAR);
        case OP(SZ_HALF, OP_BCL): VM_IMPL_AND(SIZE16, uint16_t, uint16_t, vm_source16, vm_source16_stay, vm_target16, OPER_BIT_CLEAR);
        case OP(SZ_WORD, OP_BCL): VM_IMPL_AND(SIZE32, uint32_t, uint32_t, vm_source32, vm_source32_stay, vm_target32, OPER_BIT_CLEAR);

        case OP(SZ_BYTE, OP_CMP): VM_IMPL_CMP(SIZE8, uint8_t, vm_source8);
        case OP(SZ_HALF, OP_CMP): VM_IMPL_CMP(SIZE16, uint16_t, vm_source16);
        case OP(SZ_WORD, OP_CMP): VM_IMPL_CMP(SIZE32, uint32_t, vm_source32);

        case OP(SZ_BYTE, OP_BTS): VM_IMPL_BTS(SIZE8, uint8_t, vm_source8);
        case OP(SZ_HALF, OP_BTS): VM_IMPL_BTS(SIZE16, uint16_t, vm_source16);
        case OP(SZ_WORD, OP_BTS): VM_IMPL_BTS(SIZE32, uint32_t, vm_source32);

        case OP(SZ_WORD, OP_MSE): {
            VM_PRELUDE_0();
            vm->mmu_enabled = true;
            break;
        };
        case OP(SZ_WORD, OP_MCL): {
            VM_PRELUDE_0();
            vm->mmu_enabled = false;
            break;
        };
        case OP(SZ_WORD, OP_TLB): {
            VM_PRELUDE_1(SIZE32);
            set_and_flush_tlb(vm_source32(vm, instr.source));
            break;
        };
        case OP(SZ_WORD, OP_FLP): {
            VM_PRELUDE_1(SIZE32);
            flush_single_page(vm_source32(vm, instr.source));
            break;
        };

        default:
            vm_panic(vm, FOX32_ERR_BADOPCODE);
    }

    vm->pointer_instr = vm->pointer_instr_mut;
}

static err_t vm_step(vm_t *vm) {
    if (setjmp(vm->panic_jmp) != 0) {
        return vm->halted = true, vm->panic_err;
    }
    vm_execute(vm);
    return FOX32_ERR_OK;
}
static err_t vm_resume(vm_t *vm, uint32_t count) {
    if (setjmp(vm->panic_jmp) != 0) {
        return vm->halted = true, vm->panic_err;
    }
    uint32_t remaining = count;
    while (!vm->halted && remaining > 0) {
        vm_execute(vm);
        remaining -= 1;
    }
    return FOX32_ERR_OK;
}

static fox32_err_t vm_raise(vm_t *vm, uint16_t vector) {
    if (!vm->flag_interrupt && vector < 256) {
        return FOX32_ERR_NOINTERRUPTS;
    }
    if (setjmp(vm->panic_jmp) != 0) {
        return vm->panic_err;
    }

    uint32_t pointer_handler =
        vm->memory_ram[SIZE32 * (uint32_t) vector] |
        vm->memory_ram[SIZE32 * (uint32_t) vector + 1] <<  8 |
        vm->memory_ram[SIZE32 * (uint32_t) vector + 2] << 16 |
        vm->memory_ram[SIZE32 * (uint32_t) vector + 3] << 24;

    if (vm->flag_swap_sp) {
        uint32_t old_stack_pointer = vm->pointer_stack;
        vm->pointer_stack = vm->pointer_exception_stack;
        vm_push32(vm, old_stack_pointer);
        vm_push32(vm, vm->pointer_instr);
        vm_push8(vm, vm_flags_get(vm));
        vm->flag_swap_sp = false;
    } else {
        vm_push32(vm, vm->pointer_instr);
        vm_push8(vm, vm_flags_get(vm));
    }

    if (vector >= 256) {
        // if this is an exception, push the operand
        vm_push32(vm, vm->exception_operand);
        vm->exception_operand = 0;
    } else {
        // if this is an interrupt, push the vector
        vm_push32(vm, (uint32_t) vector);
    }

    vm->pointer_instr = pointer_handler;
    vm->halted = true;
    vm->flag_interrupt = false;

    return FOX32_ERR_OK;
}

static fox32_err_t vm_recover(vm_t *vm, err_t err) {
    switch (err) {
        case FOX32_ERR_DEBUGGER:
            return vm_raise(vm, EX_DEBUGGER);
        case FOX32_ERR_FAULT_RD:
            return vm_raise(vm, EX_FAULT_RD);
        case FOX32_ERR_FAULT_WR:
            return vm_raise(vm, EX_FAULT_WR);
        case FOX32_ERR_BADOPCODE:
        case FOX32_ERR_BADCONDITION:
        case FOX32_ERR_BADREGISTER:
        case FOX32_ERR_BADIMMEDIATE:
            return vm_raise(vm, EX_ILLEGAL);
        case FOX32_ERR_DIVZERO:
            return vm_raise(vm, EX_DIVZERO);
        case FOX32_ERR_IOREAD:
        case FOX32_ERR_IOWRITE:
            return vm_raise(vm, EX_BUS);
        default:
            return FOX32_ERR_CANTRECOVER;
    }
}

#define VM_SAFEPUSH_BODY(_vm_push)    \
    if (setjmp(vm->panic_jmp) != 0) { \
        return vm->panic_err;         \
    }                                 \
    _vm_push(vm, value);              \
    return FOX32_ERR_OK;

static fox32_err_t vm_safepush_byte(vm_t *vm, uint8_t value) {
    VM_SAFEPUSH_BODY(vm_push8)
}
static fox32_err_t vm_safepush_half(vm_t *vm, uint16_t value) {
    VM_SAFEPUSH_BODY(vm_push16)
}
static fox32_err_t vm_safepush_word(vm_t *vm, uint32_t value) {
    VM_SAFEPUSH_BODY(vm_push32)
}

#define VM_SAFEPOP_BODY(_vm_pop)      \
    *value = 0;                       \
    if (setjmp(vm->panic_jmp) != 0) { \
        return vm->panic_err;         \
    }                                 \
    *value = _vm_pop(vm);             \
    return FOX32_ERR_OK;

static fox32_err_t vm_safepop_byte(vm_t *vm, uint8_t *value) {
    VM_SAFEPOP_BODY(vm_pop8)
}
static fox32_err_t vm_safepop_half(vm_t *vm, uint16_t *value) {
    VM_SAFEPOP_BODY(vm_pop16)
}
static fox32_err_t vm_safepop_word(vm_t *vm, uint32_t *value) {
    VM_SAFEPOP_BODY(vm_pop32)
}

const char *fox32_strerr(fox32_err_t err) {
    return err_tostring(err);
}
void fox32_init(fox32_vm_t *vm) {
    vm_init(vm);
}
fox32_err_t fox32_step(fox32_vm_t *vm) {
    return vm_step(vm);
}
fox32_err_t fox32_resume(fox32_vm_t *vm, uint32_t count) {
    return vm_resume(vm, count);
}
fox32_err_t fox32_raise(fox32_vm_t *vm, uint16_t vector) {
    return vm_raise(vm, vector);
}
fox32_err_t fox32_recover(fox32_vm_t *vm, fox32_err_t err) {
    return vm_recover(vm, err);
}
fox32_err_t fox32_push_byte(fox32_vm_t *vm, uint8_t value) {
    return vm_safepush_byte(vm, value);
}
fox32_err_t fox32_push_half(fox32_vm_t *vm, uint16_t value) {
    return vm_safepush_half(vm, value);
}
fox32_err_t fox32_push_word(fox32_vm_t *vm, uint32_t value) {
    return vm_safepush_word(vm, value);
}
fox32_err_t fox32_pop_byte(fox32_vm_t *vm, uint8_t *value) {
    return vm_safepop_byte(vm, value);
}
fox32_err_t fox32_pop_half(fox32_vm_t *vm, uint16_t *value) {
    return vm_safepop_half(vm, value);
}
fox32_err_t fox32_pop_word(fox32_vm_t *vm, uint32_t *value) {
    return vm_safepop_word(vm, value);
}
