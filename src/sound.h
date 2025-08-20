#pragma once

#define FOX32_AUDIO_CHANNELS 8
#define FOX32_AUDIO_BUFFER_SIZE 32768
#define FOX32_AUDIO_BUFFER_IRQ 0xFE

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
    bool refill_pending;
    uint8_t buffer_mode;
    uint8_t buffer_phase;
    uint8_t buffer_rate;
    uint32_t buffer_pos;
} sound_t;

void sound_init();

/*
audio ports start at 0x80000600

writing:
0x800006x0 - AUDxSTART (32-bit)
0x800006x1 - AUDxEND (32-bit)
0x800006x2 - AUDxLOOPSTART (32-bit)
0x800006x3 - AUDxLOOPEND (32-bit)
0x800006x4 - AUDxRATE (32-bit)
0x800006x5 - AUDxCONTROL
    bit 31:10 - 0
    bit 9 - 8/16-bit PCM select (0 = 8-bit, 1 = 16-bit)
    bit 8 - enable (1=sound on, 0=sound off)
    bit 7 - loop
    bit 6:0 - volume
0x800006x6 - AUDxPAN
    bit 15:8 - left volume 0-255
    bit 7:0 - right volume 0-255
0x80000680 - AUDBASE
0x80000681 - AUDCTL
    bit 15:8 - buffer rate
    bit 5:4 - buffer format
        00: mono 8-bit
        01: mono 16-bit
        10: stereo 8-bit
        11: stereo 16-bit
    bit 1 - sound refill pending flag
        write 0 here to acknowledge a refill IRQ
    bit 0 - buffer mode
when bit 0 of AUDCTL is 1, the channels are disabled, and the audio controller
now expects an audio buffer of 32768 bytes at the address specified in AUDBASE.
when the buffer position is half-way through the length (position >= 16384),
an IRQ is raised and the sound refill pending flag is set. the flag must then be
cleared in order for another IRQ to occur.

reading:
0x800006x0 - AUDxPOS (32-bit)
0x800006x1 - AUDxDAT (32-bit)
0x800006x2 - null 
0x800006x3 - null
0x800006x4 - AUDxRATE (32-bit)
0x800006x5 - AUDxCONTROL
    bit 31:10 - 0
    bit 9 - 8/16-bit PCM select (0 = 8-bit, 1 = 16-bit)
    bit 8 - enable (1=sound on, 0=sound off)
    bit 7 - loop
    bit 6:0 - volume
0x80000680 - AUDBASE
0x80000681 - AUDCTL
    bit 31:16 - 0
    bit 15:8 - buffer rate
    bit 5:4 - buffer format
    bit 1 - sound refill pending flag
        a value of 1 here indicates that the buffer refill IRQ
        has not yet been acknowledged
    bit 0 - buffer mode
*/