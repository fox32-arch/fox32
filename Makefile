TARGET = linux
ifeq ($(TARGET),linux)
# default is used for CC
SDL2_CONFIG = sdl2-config
CFLAGS += -g -Ofast -std=c99 -Wall -Wextra `$(SDL2_CONFIG) --cflags`
LDFLAGS += `$(SDL2_CONFIG) --libs`
else
ifeq ($(TARGET),mingw)
CC = x86_64-w64-mingw32-gcc
SDL2_CONFIG = /usr/local/x86_64-w64-mingw32/bin/sdl2-config
CFLAGS += -g -Ofast -std=c99 -Wall -Wextra -DWINDOWS -I/usr/local/x86_64-w64-mingw32/include -Dmain=SDL_main
LDFLAGS += -lmingw32 -lSDL2main -lSDL2 -L/usr/local/x86_64-w64-mingw32/lib -lmingw32 -lSDL2main -lSDL2 -mwindows
TARGET_FILE_EXTENSION = .exe
else
ifeq ($(TARGET),wasm)
CC = emcc
CFLAGS += -O3 -std=c99 -Wall -Wextra
LDFLAGS += -s TOTAL_MEMORY=70057984 -sALLOW_MEMORY_GROWTH=1 -sUSE_SDL=2 --preload-file fox32os.img
TARGET_EXTRADEPS = fox32os.img
TARGET_FILE_EXTENSION = .html
else
$(error unknown TARGET)
endif
endif
endif

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

OBJS = $(addsuffix .o, $(basename $(CFILES)))

.PHONY: all
all: fox32$(TARGET_FILE_EXTENSION)

FOX32ROM_IN = fox32.rom
FOX32ROM_OUT = fox32rom.h

$(FOX32ROM_OUT): $(FOX32ROM_IN)
	xxd -i $(FOX32ROM_IN) $(FOX32ROM_OUT)
	sed -i -e 's/fox32_rom/fox32rom/' $(FOX32ROM_OUT)

fox32$(TARGET_FILE_EXTENSION): $(TARGET_EXTRADEPS) $(OBJS)
	$(CC) -o $@ $(OBJS) $(LDFLAGS)

%.o: %.c $(FOX32ROM_OUT)
	$(CC) -o $@ -c $< $(CFLAGS)

clean:
	rm -rf fox32 fox32.exe fox32.wasm fox32.html fox32.data fox32.js fox32rom.h $(OBJS)
