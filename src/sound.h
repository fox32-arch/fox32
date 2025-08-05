#pragma once

#define FOX32_AUDIO_CHANNELS 8
#define FOX32_AUDIO_BUFFER_SIZE 16384
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
	
	bool inhibit;
	uint32_t active_base;
	uint32_t active_pos;
	bool alternate;
	bool refill_pending;
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
    bit 15:10 - 0
    bit 9 - 8/16-bit PCM select (0 = 8-bit, 1 = 16-bit)
    bit 8 - enable (1=sound on, 0=sound off)
    bit 7 - loop
    bit 6:0 - volume
0x800006x6 - AUDxPAN
    bit 15:8 - left volume 0-255
    bit 7:0 - right volume 0-255
0x80000680 - AUDBASE
0x80000681 - AUDCTL
	bit 0: disable audio controller
when bit 0 of AUDCTL is 1, the channels are disabled, and the audio controller
now expects an area of 32768 bytes for 2 buffers of 16384 bytes (16-bit PCM,
interleaved stereo). 
in this mode, the audio controller employs double audio buffers. it reads from
the active buffer, then when it is half-way read, an IRQ is raised in order to
let the processor know it is time to fill the other buffer. after the controller
is done reading that buffer, it then reads the second buffer, then raises an IRQ
to refill the first buffer, and this alternates back and forth.

reading:
0x800006x0 - AUDxPOS (32-bit)
0x800006x1 - AUDxDAT (32-bit)
0x800006x2 - null 
0x800006x3 - null
0x800006x4 - AUDxRATE (32-bit)
0x800006x5 - AUDxCONTROL
    bit 15:10 - 0
    bit 9 - 8/16-bit PCM select (0 = 8-bit, 1 = 16-bit)
    bit 8 - enable (1=sound on, 0=sound off)
    bit 7 - loop
    bit 6:0 - volume
0x80000680 - AUDBASE
*/