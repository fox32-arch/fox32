#include <SDL2/SDL.h>
#include <getopt.h>
#include <math.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bus.h"
#include "cpu.h"
#include "framebuffer.h"

extern fox32_vm_t vm;

int bus_io_read(void *user, uint32_t *value, uint32_t port) {
    (void) user;
    switch (port) {
        case 0x80000000 ... 0x8000031F: { // overlay port
            uint8_t overlay_number = (port & 0x000000FF);
            uint8_t setting = (port & 0x0000FF00) >> 8;
            switch (setting) {
                case 0x00: {
                    // overlay position
                    uint32_t x = overlay_get(overlay_number)->x;
                    uint32_t y = overlay_get(overlay_number)->y;
                    *value = (y << 16) | x;
                    break;
                };
                case 0x01: {
                    // overlay size
                    uint32_t width = overlay_get(overlay_number)->width;
                    uint32_t height = overlay_get(overlay_number)->height;
                    *value = (height << 16) | width;
                    break;
                };
                case 0x02: {
                    // overlay framebuffer pointer
                    *value = overlay_get(overlay_number)->pointer;
                    break;
                };
                case 0x03: {
                    // overlay enable status
                    *value = overlay_get(overlay_number)->enabled ? 1 : 0;
                    break;
                };
            }

            break;
        };
    }

    return 0;
}

int bus_io_write(void *user, uint32_t value, uint32_t port) {
    (void) user;
    switch (port) {
        case 0x00000000: { // terminal output port
            putchar((int) value);
            fflush(stdout);
            break;
        };

        case 0x80000000 ... 0x8000031F: { // overlay port
            uint8_t overlay_number = (port & 0x000000FF);
            uint8_t setting = (port & 0x0000FF00) >> 8;
            switch (setting) {
                case 0x00: {
                    // overlay position
                    uint32_t x = value & 0x0000FFFF;
                    uint32_t y = (value & 0xFFFF0000) >> 16;
                    overlay_get(overlay_number)->x = x;
                    overlay_get(overlay_number)->y = y;
                    break;
                };
                case 0x01: {
                    // overlay size
                    uint32_t width = value & 0x0000FFFF;
                    uint32_t height = (value & 0xFFFF0000) >> 16;
                    overlay_get(overlay_number)->width = width;
                    overlay_get(overlay_number)->height = height;
                    break;
                };
                case 0x02: {
                    // overlay framebuffer pointer
                    overlay_get(overlay_number)->pointer = value;
                    break;
                };
                case 0x03: {
                    // overlay enable status
                    overlay_get(overlay_number)->enabled = value != 0;
                    break;
                };
            }

            break;
        };
    }

    return 0;
}
