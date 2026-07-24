#pragma once

#define FOX32_AUDIO_CHANNELS 8
#define FOX32_AUDIO_BUFFER_SIZE 32768
#define FOX32_AUDIO_BUF0_IRQ 0xFD
#define FOX32_AUDIO_BUF1_IRQ 0xFE

typedef struct {
    uint32_t start;
    uint32_t end;
    uint32_t loop_start;
    uint32_t loop_end;
    bool loop;
    bool enable;
    bool last_enable;
    
    uint8_t volume;
    uint8_t left_volume;
    uint8_t right_volume;
    
    uint32_t accumulator;
    uint32_t frequency;
    
    bool bits16;
    
    uint32_t position;
    int16_t data;
} sound_channel_t;

typedef struct {
    sound_channel_t channel[FOX32_AUDIO_CHANNELS];
    uint32_t base;
    int32_t out_left;
    int32_t out_right;
    
    bool buffer;
    bool buf0_refill_pending;
    bool buf1_refill_pending;
    uint32_t buf0_base;
    uint32_t buf1_base;
    uint8_t active_buffer;
    uint8_t buffer_mode;
    uint8_t buffer_phase;
    uint8_t buffer_rate;
    uint32_t buffer_size;
    uint32_t buffer_pos;
} sound_t;

void sound_init();
void sound_sync(uint32_t cycles_executed);