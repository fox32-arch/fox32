#include <SDL2/SDL.h>
#include <getopt.h>
#include <math.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cpu.h"
#include "framebuffer.h"
#include "screen.h"

uint32_t PixelBuffer[FRAMEBUFFER_WIDTH * FRAMEBUFFER_HEIGHT];

extern fox32_vm_t vm;

void FramebufferDraw(struct Screen screen) {
    SDL_Texture *texture = ScreenGetTexture(screen);
    memcpy(PixelBuffer, &vm.memory_ram[0x02000000], FRAMEBUFFER_WIDTH * FRAMEBUFFER_HEIGHT * 4);
    SDL_UpdateTexture(texture, NULL, PixelBuffer, FRAMEBUFFER_WIDTH * 4);
}
