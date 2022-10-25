SDL2_CONFIG = sdl2-config
CFLAGS = -g -Ofast -std=c99 -Wall -Wextra `$(SDL2_CONFIG) --cflags --libs`
TARGET=fox32

CFILES = src/main.c \
		src/bus.c \
		src/cpu.c \
		src/disk.c \
		src/framebuffer.c \
		src/keyboard.c \
		src/mouse.c \
		src/screen.c

$(TARGET): $(CFILES)
	$(CC) -o $@ $(filter %.c, $^) $(CFLAGS)

clean:
	rm -rf fox32
