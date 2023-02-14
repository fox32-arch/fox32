SDL2_CONFIG = sdl2-config
CFLAGS = -g -Ofast -std=c99 -Wall -Wextra `$(SDL2_CONFIG) --cflags --libs`
CC_WIN = x86_64-w64-mingw32-gcc
CFLAGS_WIN = -g -Ofast -std=c99 -Wall -Wextra -lmingw32 -lSDL2main -lSDL2
CC_WASM = emcc
CFLAGS_WASM = -O3 -std=c99 -Wall -Wextra -s TOTAL_MEMORY=70057984 -sALLOW_MEMORY_GROWTH=1 -sUSE_SDL=2 --preload-file fox32os.img
TARGET=fox32
TARGET_WIN=fox32.exe
TARGET_WASM=fox32.html

CFILES = src/main.c \
		src/bus.c \
		src/cpu.c \
		src/disk.c \
		src/framebuffer.c \
		src/keyboard.c \
		src/mmu.c \
		src/mouse.c \
		src/screen.c \
		src/serial.c

FOX32ROM_IN = fox32.rom
FOX32ROM_OUT = fox32rom.h

$(TARGET): $(CFILES) $(FOX32ROM_IN)
	xxd -i $(FOX32ROM_IN) $(FOX32ROM_OUT)
	sed -i -e 's/fox32_rom/fox32rom/' fox32rom.h
	$(CC) -o $@ $(filter %.c, $^) $(CFLAGS)

$(TARGET_WIN): $(CFILES)
	xxd -i $(FOX32ROM_IN) $(FOX32ROM_OUT)
	sed -i -e 's/fox32_rom/fox32rom/' fox32rom.h
	$(CC_WIN) -o $@ $(filter %.c, $^) $(CFLAGS_WIN)

$(TARGET_WASM): $(CFILES) $(FOX32ROM_IN)
	xxd -i $(FOX32ROM_IN) $(FOX32ROM_OUT)
	sed -i -e 's/fox32_rom/fox32rom/' fox32rom.h
	$(CC_WASM) -o $@ $(filter %.c, $^) $(CFLAGS_WASM)

clean:
	rm -rf $(TARGET) $(TARGET_WIN) $(TARGET_WASM)
