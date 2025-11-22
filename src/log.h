#define LOG(format, ...) {                                  \
    if (should_log) {                                       \
        printf("[emulator] [RIP=0x%X] ", vm.pointer_instr); \
        printf(format, __VA_ARGS__);                        \
    }                                                       \
}
#define FORCE_LOG(format, ...) {                        \
    printf("[emulator] [RIP=0x%X] ", vm.pointer_instr); \
    printf(format, __VA_ARGS__);                        \
}

#define LOG0(str) { if (should_log) printf("[emulator] [RIP=0x%X] %s", vm.pointer_instr, str); }
#define FORCE_LOG0(str) { printf("[emulator] [RIP=0x%X] %s", vm.pointer_instr, str); }
