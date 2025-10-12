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
#include "sound.h"

#include "log.h"

#include "../fox32rom.h"

#define FPS 60
#define TPF 1
#define TPS (FPS * TPF)

fox32_vm_t vm;

extern bool bus_requests_exit;
extern disk_controller_t disk_controller;
extern sound_t snd;
extern int ScreenScale;

bool should_log = false;

uint32_t tick_start;
uint32_t tick_end;
int ticks = 0;
bool done = false;

time_t rtc_time;
uint32_t rtc_uptime;

void main_loop(void);
void load_rom(const char *filename);

int main(int argc, char *argv[]) {
    uint32_t memory_size = 64 * 1024 * 1024; // 64 MiB
    size_t disk_id = 0;
    char *rom_path = NULL;
    bool debug = false;
    bool headless = false;
    int filtering_mode = 0;
#ifndef __EMSCRIPTEN__
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--help") == 0) {
            fprintf(
                stderr,
                "Usage: %s [OPTIONS]\n\n"
                "Options:\n"
                "  --help             Print this message\n"
                "  --verbose          Print info about options specified\n"
                "  --disk DISK        Specify a disk image to use\n"
                "  --rom ROM          Specify a ROM image to use\n"
                "  --debug            Enable debug output\n"
                "  --headless         Headless mode: don't open a window\n"
                "  --memory SIZE      Specify RAM size in MiB\n"
                "  --scale MULT       Scale display by MULT (default multiplier is %d)\n"
                "  --filtering MODE   Set scale filtering mode for high DPI displays\n"
                "                       0 = nearest pixel (default)\n"
                "                       1 = linear filtering\n",
                argv[0], SCREEN_ZOOM
            );
            return 0;
        } else if (strcmp(argv[i], "--verbose") == 0) {
            should_log = true;
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
                rom_path = argv[i + 1];
                i++;
            } else {
                fprintf(stderr, "no ROM image specified\n");
                return 1;
            }
        } else if (strcmp(argv[i], "--debug") == 0) {
            debug = true;
        } else if (strcmp(argv[i], "--headless") == 0) {
            headless = true;
        } else if (strcmp(argv[i], "--memory") == 0) {
            if (i + 1 < argc) {
                char *end_ptr;
                memory_size = (uint32_t) strtol(argv[i + 1], &end_ptr, 10) * 1024 * 1024;
                LOG("memory size: %u bytes\n", memory_size);
                if (end_ptr == argv[i + 1]) {
                    fprintf(stderr, "bad memory size specified\n");
                    return 1;
                }
                i++;
            } else {
                fprintf(stderr, "no memory size specified\n");
                return 1;
            }
        } else if (strcmp(argv[i], "--scale") == 0) {
            if (i + 1 < argc) {
                char *end_ptr;
                ScreenScale = (int) strtol(argv[i + 1], &end_ptr, 10);
                if (end_ptr == argv[i + 1]) {
                    fprintf(stderr, "bad scale multiplier specified\n");
                    return 1;
                }
                i++;
            } else {
                fprintf(stderr, "no scale multiplier specified\n");
                return 1;
            }
        } else if (strcmp(argv[i], "--filtering") == 0) {
            if (i + 1 < argc) {
                if (strcmp(argv[i + 1], "0") == 0) {
                    filtering_mode = 0; // nearest pixel filtering
                } else if (strcmp(argv[i + 1], "1") == 0) {
                    filtering_mode = 1; // linear filtering
                } else {
                    fprintf(stderr, "incorrect scale filtering mode specified\n");
                    return 1;
                }
                i++;
            } else {
                fprintf(stderr, "no scale filtering mode specified\n");
                return 1;
            }
        } else {
            fprintf(stderr, "unrecognized option %s\n", argv[i]);
            return 1;
        }
    }
#endif

    fox32_init(&vm, memory_size);
    vm.io_read = bus_io_read;
    vm.io_write = bus_io_write;
    vm.halted = false;
    vm.debug = debug;
    vm.headless = headless;

#ifdef __EMSCRIPTEN__
    new_disk("fox32os.img", disk_id++);
#endif

    if (rom_path)
        load_rom(rom_path);
    else
        memcpy(vm.memory_rom, fox32rom, sizeof(fox32rom));

    if (!vm.headless) {
        if (SDL_Init(SDL_INIT_VIDEO | SDL_INIT_AUDIO) != 0) {
            fprintf(stderr, "unable to initialize SDL: %s", SDL_GetError());
            return 1;
        }

        SDL_ShowCursor(SDL_DISABLE);

        ScreenCreate(
            FRAMEBUFFER_WIDTH, FRAMEBUFFER_HEIGHT,
            filtering_mode,
            draw_framebuffer,
            key_pressed,
            key_released,
            mouse_pressed,
            mouse_released,
            mouse_moved,
            drop_file
        );
        sound_init();

        ScreenInit();
        ScreenDraw();
    }

    serial_init();

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

    fox32_exit(&vm);
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
    LOG("using %s as boot ROM\n", filename);

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
