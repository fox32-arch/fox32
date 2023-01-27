#pragma once

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <setjmp.h>

#define FOX32_CPU_HZ 33000000

#define FOX32_MEMORY_RAM 0x04000000 //  64 MiB
#define FOX32_MEMORY_ROM 0x00080000 // 512 KiB
#define FOX32_MEMORY_ROM_START 0xF0000000

#define FOX32_POINTER_DEFAULT_INSTR FOX32_MEMORY_ROM_START
#define FOX32_POINTER_DEFAULT_STACK 0x00000000

#define FOX32_POINTER_INTERRUPTVECS 0x00000000

#define FOX32_REGISTER_LOOP 31
#define FOX32_REGISTER_COUNT 32

typedef enum {
    FOX32_ERR_OK,
    FOX32_ERR_INTERNAL,
    FOX32_ERR_DEBUGGER,
    FOX32_ERR_FAULT_RD,
    FOX32_ERR_FAULT_WR,
    FOX32_ERR_BADOPCODE,
    FOX32_ERR_BADCONDITION,
    FOX32_ERR_BADREGISTER,
    FOX32_ERR_BADIMMEDIATE,
    FOX32_ERR_DIVZERO,
    FOX32_ERR_IOREAD,
    FOX32_ERR_IOWRITE,
    FOX32_ERR_NOINTERRUPTS,
    FOX32_ERR_CANTRECOVER
} fox32_err_t;

const char *fox32_strerr(fox32_err_t err);

typedef int fox32_io_read_t(void *user, uint32_t *value, uint32_t port);
typedef int fox32_io_write_t(void *user, uint32_t value, uint32_t port);

typedef struct {
    uint32_t pointer_instr_mut;
    uint32_t pointer_instr;
    uint32_t pointer_stack;
    uint32_t pointer_exception_stack;
    uint32_t pointer_frame;
    uint32_t pointer_page_directory;
    uint32_t registers[FOX32_REGISTER_COUNT];

    bool flag_zero;
    bool flag_carry;
    bool flag_interrupt;
    bool flag_swap_sp;

    bool halted;

    bool debug;
    bool headless;

    bool mmu_enabled;

    jmp_buf panic_jmp;
    fox32_err_t panic_err;

    uint32_t exception_operand;

    void *io_user;
    fox32_io_read_t *io_read;
    fox32_io_write_t *io_write;

    uint8_t memory_ram[FOX32_MEMORY_RAM];
    uint8_t memory_rom[FOX32_MEMORY_ROM];
} fox32_vm_t;

void fox32_init(fox32_vm_t *vm);

fox32_err_t fox32_step(fox32_vm_t *vm);
fox32_err_t fox32_resume(fox32_vm_t *vm, uint32_t count, uint32_t *executed);

fox32_err_t fox32_raise(fox32_vm_t *vm, uint16_t vector);
fox32_err_t fox32_recover(fox32_vm_t *vm, fox32_err_t err);

fox32_err_t fox32_push_byte(fox32_vm_t *vm, uint8_t value);
fox32_err_t fox32_push_half(fox32_vm_t *vm, uint16_t value);
fox32_err_t fox32_push_word(fox32_vm_t *vm, uint32_t value);
fox32_err_t fox32_pop_byte(fox32_vm_t *vm, uint8_t *value);
fox32_err_t fox32_pop_half(fox32_vm_t *vm, uint16_t *value);
fox32_err_t fox32_pop_word(fox32_vm_t *vm, uint32_t *value);
