SDL2_CONFIG = sdl2-config
CFLAGS = -g -Ofast -std=c99 -Wall -Wextra `$(SDL2_CONFIG) --cflags --libs`
CC_WIN = x86_64-w64-mingw32-gcc
CFLAGS_WIN = -g -Ofast -std=c99 -Wall -Wextra -lmingw32 -lSDL2main -lSDL2
TARGET=fox32
TARGET_WIN=fox32.exe

CFILES = src/main.c \
		src/bus.c \
		src/cpu.c \
		src/disk.c \
		src/framebuffer.c \
		src/keyboard.c \
		src/mmu.c \
		src/mouse.c \
		src/screen.c

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

clean:
	rm -rf fox32 fox32.exe
