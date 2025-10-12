#define LOG(format, ...) {           \
    if (should_log) {                \
        printf("[emulator] ");       \
        printf(format, __VA_ARGS__); \
    }                                \
}

#define LOG0(str) { if (should_log) printf("[emulator] %s", str); }
