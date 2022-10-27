SDL2_CONFIG = sdl2-config
CFLAGS = -g -Ofast -std=c99 -Wall -Wextra `$(SDL2_CONFIG) --cflags --libs`
TARGET=fox32

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

clean:
	rm -rf fox32
