#pragma once

typedef struct {
	uint32_t start;
	uint32_t end;
	uint32_t loop_start;
	uint32_t loop_end;
	bool loop;
	bool enable;
	bool last_enable;
	
	uint8_t volume;
	
	uint32_t accumulator;
	uint32_t frequency;
	
	bool bits16;
	
	uint32_t position;
	int16_t data;
} sound_channel_t;

typedef struct {
	sound_channel_t channel[4];
	uint32_t base;
	int32_t out;
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
	bit 15:9 - 0
	bit 9 - 8/16-bit PCM select (0 = 8-bit, 1 = 16-bit)
	bit 8 - enable (1=sound on, 0=sound off)
	bit 7 - loop
	bit 6:0 - volume
0x80000680 - AUDBASE

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