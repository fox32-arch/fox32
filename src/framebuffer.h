#pragma once

#include "screen.h"

#define FRAMEBUFFER_WIDTH  640
#define FRAMEBUFFER_HEIGHT 480

#define VSYNC_INTERRUPT_VECTOR 0xFF

void FramebufferDraw(struct Screen screen);
