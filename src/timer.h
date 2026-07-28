#pragma once

#define FOX32_TIMER_IRQ 0xF8
#define FOX32_TIMER_CHANNELS 4

typedef struct {
    uint32_t period;
    uint32_t counter;
    bool enable;
    bool int_pending;
    bool clock;
    bool reload;
    bool cause_interrupt;
} timer_channel_t;

typedef struct {
    timer_channel_t timer[FOX32_TIMER_CHANNELS];
    uint32_t control;
    uint32_t tick_base;
    uint32_t tick_int_count;
    uint32_t tick_counter;
    uint32_t last_update;
} fox32_timer_t;

void timer_sync(uint32_t current_instruction);
