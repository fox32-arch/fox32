#pragma once

#include "screen.h"

#define FRAMEBUFFER_WIDTH  640
#define FRAMEBUFFER_HEIGHT 480

#define VSYNC_INTERRUPT_VECTOR 0xFF

void draw_framebuffer(struct Screen *screen);

typedef struct {
    uint32_t pointer;
    uint32_t x, y;
    uint32_t width, height;
    bool enabled;
} overlay_t;

overlay_t *overlay_get(uint32_t index);
