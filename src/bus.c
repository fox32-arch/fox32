#include <SDL2/SDL.h>
#include <getopt.h>
#include <math.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <time.h>

#include "bus.h"
#include "cpu.h"
#include "disk.h"
#include "framebuffer.h"
#include "keyboard.h"
#include "mouse.h"
#include "serial.h"

bool bus_requests_exit = false;

extern time_t rtc_time;
extern uint32_t rtc_uptime;

extern fox32_vm_t vm;
extern disk_controller_t disk_controller;
extern mouse_t mouse;

int bus_io_read(void *user, uint32_t *value, uint32_t port) {
    (void) user;
    switch (port) {
        case 0x00000000: { // serial port
            *value = serial_get();
            break;
        };

        case 0x80000000 ... 0x8000031F: { // overlay port
            uint8_t overlay_number = port & 0x000000FF;
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

        case 0x80000400 ... 0x80000401: { // mouse port
            uint8_t setting = port & 0x000000FF;
            switch (setting) {
                case 0x00: {
                    // button states
                    if (mouse.clicked) *value |= 0b001;
                    if (mouse.released) *value |= 0b010;
                    if (mouse.held) *value |= 0b100; else *value &= ~(0b100);
                    break;
                };
                case 0x01: {
                    // position
                    *value = (mouse.y << 16) | mouse.x;
                    break;
                };
            }

            break;
        };

        case 0x80000500: {
            *value = (uint32_t) key_take();

            break;
        }

        case 0x80000700 ... 0x80000707: { // RTC port
            uint8_t setting = port & 0x000000FF;
            struct tm *now = localtime(&rtc_time);
            switch (setting) {
                case 0x00: { // year
                    *value = now->tm_year + 1900;
                    break;
                }
                case 0x01: { // month
                    *value = now->tm_mon + 1;
                    break;
                }
                case 0x02: { // day
                    *value = now->tm_mday;
                    break;
                }
                case 0x03: { // hour
                    *value = now->tm_hour;
                    break;
                }
                case 0x04: { // minute
                    *value = now->tm_min;
                    break;
                }
                case 0x05: { // second
                    *value = now->tm_sec;
                    break;
                }
                case 0x06: { // ms since startup
                    *value = rtc_uptime;
                    break;
                }
                case 0x07: { // daylight savings time active
                    *value = now->tm_isdst;
                    break;
                }
            }

            break;
        }

        case 0x80001000 ... 0x80002003: { // disk controller port
            size_t id = port & 0xFF;
            uint8_t operation = (port & 0x0000F000) >> 8;
            switch (operation) {
                case 0x10: {
                    // current insert state of specified disk id
                    // size will be zero if disk isn't inserted
                    *value = get_disk_size(id);
                    break;
                };
                case 0x20: {
                    // current buffer pointer
                    *value = disk_controller.buffer_pointer;
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
        case 0x00000000: { // serial port
            serial_put(value);
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

        case 0x80000400 ... 0x80000401: { // mouse port
            uint8_t setting = port & 0x000000FF;
            switch (setting) {
                case 0x00: {
                    // button states
                    mouse.clicked = value & 0b001;
                    mouse.released = value & 0b010;
                    mouse.held = value & 0b100;
                    break;
                };
                case 0x01: {
                    // position
                    mouse.x = value & 0x0000FFFF;
                    mouse.y = (value & 0xFFFF0000) >> 16;
                    break;
                };
            }

            break;
        };

        case 0x80001000 ... 0x80005003: { // disk controller port
            size_t id = port & 0xFF;
            uint8_t operation = (port & 0x0000F000) >> 8;
            switch (operation) {
                case 0x10: {
                    // no-op
                    break;
                };
                case 0x20: {
                    // set the buffer pointer
                    disk_controller.buffer_pointer = value;
                    break;
                };
                case 0x30: {
                    // read specified disk sector into memory
                    set_disk_sector(id, value);
                    read_disk_into_memory(id);
                    break;
                };
                case 0x40: {
                    // write specified disk sector from memory
                    set_disk_sector(id, value);
                    write_disk_from_memory(id);
                    break;
                };
                case 0x50: {
                    // remove specified disk
                    remove_disk(id);
                    break;
                };
            }

            break;
        };

        case 0x80010000: { // power control port
            if (value == 0) {
                bus_requests_exit = true;
            }
        };
    }

    return 0;
}

void drop_file(char *filename) {
    int last_id = 0;
    for (int i = 0; i < 4; i++) {
        if (disk_controller.disks[i].size != 0) {
            last_id++;
        }
    }
    new_disk(filename, last_id);
}
