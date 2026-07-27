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
#include "timer.h"

timer_t tim;
extern fox32_vm_t vm;

void timer_step() {
    bool pulse = false;
    if (tim.tick_base > 0) {
        if (tim.tick_int_count > 0) {
            tim.tick_int_count--;
        }
        
        if (tim.tick_int_count == 0) {
            tim.tick_int_count = tim.tick_base;
            tim.tick_counter++;
            pulse = true;
        }
    }
    for (int i=0; i<FOX32_TIMER_CHANNELS; i++) {
        if (!tim.timer[i].enable) continue;
        bool should_tick = tim.timer[i].clock ? true : pulse;
        if (!should_tick) continue;
        if (tim.timer[i].counter > 0) tim.timer[i].counter--;
        if (tim.timer[i].counter == 0) {
            if (tim.timer[i].reload && tim.timer[i].period > 0) {
                tim.timer[i].counter = tim.timer[i].period;
            } else {
                tim.timer[i].enable = false;
            }
            if (tim.timer[i].cause_interrupt && !tim.timer[i].int_pending) {
                // interrupt main VM
                fox32_raise(&vm, FOX32_TIMER_IRQ+i);
                //printf("timer int %d\n", i);
                tim.timer[i].int_pending = true;
            }
        }
    }
}

uint32_t timer_read_control(void) {
    uint32_t control = 0;

    for (int i = 0; i < FOX32_TIMER_CHANNELS; i++) {
        if (tim.timer[i].enable)          control |= (1 << i);          // 3:0
        if (tim.timer[i].int_pending)     control |= (1 << (4 + i));    // 7:4
        if (tim.timer[i].clock)           control |= (1 << (8 + i));    // 11:8
        if (tim.timer[i].reload)          control |= (1 << (12 + i));   // 15:12
        /* 19:16 are write-strobe, and should read 0 */
        if (tim.timer[i].cause_interrupt) control |= (1 << (20 + i));   // 23:20
    }
    return control;
}

void timer_sync(uint32_t current_instruction) {
    if (tim.last_update == 0 && current_instruction > 0) {
        tim.last_update = current_instruction;
        return;
    }
    uint32_t elapsed = current_instruction - tim.last_update;
    if (elapsed == 0) return;
    for (uint32_t i = 0; i < elapsed; i++) {
        timer_step(); 
    }
    tim.last_update = current_instruction;
}