SDL2_CONFIG = sdl2-config
CFLAGS = -g -Ofast -std=c99 `$(SDL2_CONFIG) --cflags --libs`
TARGET=fox32

CFILES = src/main.c \
		src/cpu.c \
		src/framebuffer.c \
		src/screen.c

$(TARGET): $(CFILES)
	$(CC) -o $@ $(filter %.c, $^) $(CFLAGS)

clean:
	rm -rf fox32*
