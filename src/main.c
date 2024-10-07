#include <SDL2/SDL.h>
#include <getopt.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <time.h>
#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#include <emscripten/html5.h>
#endif

#include "bus.h"
#include "cpu.h"
#include "disk.h"
#include "framebuffer.h"
#include "keyboard.h"
#include "mouse.h"
#include "screen.h"
#include "serial.h"

#include "../fox32rom.h"

#define FPS 60
#define TPF 1
#define TPS (FPS * TPF)

fox32_vm_t vm;

extern bool bus_requests_exit;
extern disk_controller_t disk_controller;

uint32_t tick_start;
uint32_t tick_end;
int ticks = 0;
bool done = false;

time_t rtc_time;
uint32_t rtc_uptime;

void main_loop(void);
void load_rom(const char *filename);

int main(int argc, char *argv[]) {
    fox32_init(&vm);
    vm.io_read = bus_io_read;
    vm.io_write = bus_io_write;
    vm.halted = false;
    vm.debug = false;

    memcpy(vm.memory_rom, fox32rom, sizeof(fox32rom));

    size_t disk_id = 0;
#ifndef __EMSCRIPTEN__
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--help") == 0) {
            fprintf(stderr,
                    "Usage: %s [OPTIONS]\n\n"
                    "Options:\n"
                    "  --help         Print this message\n"
                    "  --disk DISK    Specify a disk image to use\n"
                    "  --rom ROM      Specify a ROM image to use\n"
                    "  --debug        Enable debug output\n"
                    "  --headless     Headless mode: don't open a window\n"
                   , argv[0]);
            return 0;
        } else if (strcmp(argv[i], "--disk") == 0) {
            if (i + 1 < argc) {
                new_disk(argv[i + 1], disk_id++);
                i++;
            } else {
                fprintf(stderr, "no disk image specified\n");
                return 1;
            }
        } else if (strcmp(argv[i], "--rom") == 0) {
            if (i + 1 < argc) {
                load_rom(argv[i + 1]);
                i++;
            } else {
                fprintf(stderr, "no rom image specified\n");
                return 1;
            }
        } else if (strcmp(argv[i], "--debug") == 0) {
            vm.debug = true;
        } else if (strcmp(argv[i], "--headless") == 0) {
            vm.headless = true;
        } else {
            fprintf(stderr, "unrecognized option %s\n", argv[i]);
            return 1;
        }
    }
#else
    new_disk("fox32os.img", disk_id++);
#endif

    if (!vm.headless) {
        if (SDL_Init(SDL_INIT_VIDEO) != 0) {
            fprintf(stderr, "unable to initialize SDL: %s", SDL_GetError());
            return 1;
        }

        SDL_ShowCursor(SDL_DISABLE);

        ScreenCreate(
            FRAMEBUFFER_WIDTH, FRAMEBUFFER_HEIGHT,
            draw_framebuffer,
            key_pressed,
            key_released,
            mouse_pressed,
            mouse_released,
            mouse_moved,
            drop_file
        );

        ScreenInit();
        ScreenDraw();
    }

#ifndef WINDOWS
    serial_init();
#endif

    tick_start = SDL_GetTicks();
    tick_end = SDL_GetTicks();

#ifdef __EMSCRIPTEN__
    emscripten_set_main_loop(main_loop, FPS, 1);
#endif

    while (!done && !bus_requests_exit) {
        main_loop();

        tick_end = SDL_GetTicks();
        int delay = 1000/TPS - (tick_end - tick_start);
        if (delay > 0) {
            SDL_Delay(delay);
        } else {
            //printf("time overrun %d\n", delay);
        }
    }

    return 0;
}

void main_loop(void) {
#ifdef __EMSCRIPTEN__
    if (done || bus_requests_exit) {
        emscripten_cancel_main_loop();
    }
#endif
    int dt = SDL_GetTicks() - tick_start;
    tick_start = SDL_GetTicks();
    if (!dt)
        dt = 1;

    int cycles_per_tick = FOX32_CPU_HZ / TPS / dt;
    int extra_cycles = FOX32_CPU_HZ / TPS - (cycles_per_tick * dt);

    fox32_err_t error = FOX32_ERR_OK;

    for (int i = 0; i < dt; i++) {
        rtc_uptime += 1;
        rtc_time = time(NULL);

        int cycles_left = cycles_per_tick;

        if (i == dt - 1)
            cycles_left += extra_cycles;

        while (cycles_left > 0) {
            uint32_t executed = 0;

            error = fox32_resume(&vm, cycles_left, &executed);
            if (error != FOX32_ERR_OK) {
                if (vm.debug) puts(fox32_strerr(error));
                error = fox32_recover(&vm, error);
                if (error != FOX32_ERR_OK)
                    break;
            }

            cycles_left -= executed;
        }
    }

    if ((ticks % TPF) == 0) {
        if (!vm.headless)
            ScreenDraw();
        fox32_raise(&vm, VSYNC_INTERRUPT_VECTOR);
        vm.halted = false;
    }

    done = ScreenProcessEvents();

    ticks++;
}

void load_rom(const char *filename) {
    FILE *rom = fopen(filename, "r");

    if (!rom) {
        fprintf(stderr, "couldn't find ROM file %s\n", filename);
        exit(1);
    }
    printf("using %s as boot ROM\n", filename);

    if (fread(&vm.memory_rom, sizeof(fox32rom), 1, rom) != 1) {
        fprintf(stderr, "error reading ROM file %s\n", filename);
        if (feof(rom)) {
            fprintf(stderr, "ROM file too small, must be %lu bytes\n", sizeof(fox32rom));
        }
        fclose(rom);
        exit(1);
    }

    if (fgetc(rom) != EOF) {
        fprintf(stderr, "error reading ROM file %s\n", filename);
        fprintf(stderr, "ROM file too large, must be %lu bytes\n", sizeof(fox32rom));
        fclose(rom);
        exit(1);
    }

    fclose(rom);
}
