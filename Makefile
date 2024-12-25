SDL2_CONFIG ?= sdl2-config
FOX32ROM_IN ?= fox32.rom

FOX32ROM_OUT = fox32rom.h
CFLAGS += -g -Ofast -std=c99 -Wall -Wextra `$(SDL2_CONFIG) --cflags`
LDFLAGS += `$(SDL2_CONFIG) --libs`

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
all: fox32

$(FOX32ROM_OUT): $(FOX32ROM_IN)
	cp $(FOX32ROM_IN) fox32.rom
	xxd -i fox32.rom $(FOX32ROM_OUT)
	rm -f fox32.rom
	sed -i -e 's/fox32_rom/fox32rom/' $(FOX32ROM_OUT)

fox32: $(OBJS)
	$(CC) -o $@ $(OBJS) $(LDFLAGS)

%.o: %.c $(FOX32ROM_OUT)
	$(CC) -o $@ -c $< $(CFLAGS)

clean:
	rm -rf fox32 $(FOX32ROM_OUT) $(OBJS)
