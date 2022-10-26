#include <SDL2/SDL.h>
#include <getopt.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>

#include "bus.h"
#include "cpu.h"
#include "disk.h"
#include "framebuffer.h"
#include "keyboard.h"
#include "mouse.h"
#include "screen.h"

#include "../fox32rom.h"

#define FPS 60
#define TPF 1
#define TPS (FPS * TPF)

fox32_vm_t vm;

extern disk_controller_t disk_controller;

uint32_t tick_start;
uint32_t tick_end;
int ticks = 0;
bool done = false;

struct timeval rtc_current_time;
uint32_t rtc_uptime;

void main_loop(void);

int main(int argc, char *argv[]) {
    if (SDL_Init(SDL_INIT_VIDEO) != 0) {
        fprintf(stderr, "unable to initialize SDL: %s", SDL_GetError());
        return 1;
    }

    fox32_init(&vm);
    vm.io_read = bus_io_read;
    vm.io_write = bus_io_write;
    vm.halted = false;
    //vm.debug = true;

    memcpy(vm.memory_rom, fox32rom, sizeof(fox32rom));

    if (argc > 1) {
        for (int i = 0; i + 1 < argc; i++) {
            new_disk(argv[i + 1], i);
        }
    }

    ScreenCreate(
        FRAMEBUFFER_WIDTH, FRAMEBUFFER_HEIGHT,
        draw_framebuffer,
        key_pressed,
        key_released,
        mouse_pressed,
        mouse_released,
        mouse_moved
    );

    ScreenInit();
    ScreenDraw();

    tick_start = SDL_GetTicks();
    tick_end = SDL_GetTicks();

    while (!done) {
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
    int dt = SDL_GetTicks() - tick_start;
    tick_start = SDL_GetTicks();
    if (!dt)
        dt = 1;

    int cycles_per_tick = FOX32_CPU_HZ / TPS / dt;
    int extra_cycles = FOX32_CPU_HZ / TPS - (cycles_per_tick * dt);

    fox32_err_t error = FOX32_ERR_OK;

    for (int i = 0; i < dt; i++) {
        rtc_uptime += 1;
        gettimeofday(&rtc_current_time, 0);

        int cycles_left = cycles_per_tick;

        if (i == dt - 1)
            cycles_left += extra_cycles;

        while (cycles_left > 0) {
            uint32_t executed = 0;

            error = fox32_resume(&vm, cycles_left, &executed);
            if (error != FOX32_ERR_OK) {
                //puts(fox32_strerr(error));
                error = fox32_recover(&vm, error);
                if (error != FOX32_ERR_OK)
                    break;
            }

            cycles_left -= executed;
        }
    }

    if ((ticks % TPF) == 0) {
        ScreenDraw();
        fox32_raise(&vm, VSYNC_INTERRUPT_VECTOR);
        vm.halted = false;
    }

    done = ScreenProcessEvents();

    ticks++;
}
