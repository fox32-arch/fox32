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
#include "sound.h"

sound_t snd;
extern fox32_vm_t vm;

void sound_step() {
    snd.out = 0;
    for (int i=0; i<4; i++) {
        if (snd.channel[i].enable && !snd.channel[i].last_enable) {
            snd.channel[i].position = snd.channel[i].start;
        } else if (!snd.channel[i].enable && snd.channel[i].last_enable) {
            snd.channel[i].position = snd.channel[i].end;
            snd.channel[i].data = 0;
        }
        
        if (snd.channel[i].enable) {
            /* phase accumulator for handling sample rates that are not multiple of 48kHz */
            snd.channel[i].accumulator += snd.channel[i].frequency;
            if (snd.channel[i].accumulator >= (1 << 16)) {
                snd.channel[i].accumulator -= (1 << 16);
                if (snd.channel[i].position < snd.channel[i].end) {
                    uint32_t abs = snd.base + snd.channel[i].position;
                    if (snd.channel[i].bits16) {
                        /* 16-bit PCM mode */
                        snd.channel[i].data = (vm.memory_ram[abs]) | (vm.memory_ram[abs+1] << 8);
                        snd.channel[i].position += 2;
                    } else {
                        /* 8-bit PCM mode */
                        snd.channel[i].data = (int16_t)(vm.memory_ram[abs] << 8);
                        snd.channel[i].position++;
                    }
                    /* loop mode */
                    if (snd.channel[i].loop && snd.channel[i].position >= snd.channel[i].loop_end) {
                        snd.channel[i].position = snd.channel[i].loop_start;
                    }
                } else {
                    /* silence the output to ensure we do not output a dangling sample */
                    snd.channel[i].enable = false;
                    snd.channel[i].last_enable = false;
                    snd.channel[i].data = 0;
                }
            }
        } else {
            /* output nothing */
            snd.channel[i].data = 0;
        }
        snd.channel[i].last_enable = snd.channel[i].enable;
        snd.out += snd.channel[i].data * ((float)(snd.channel[i].volume & 0x7f) / 127.0f);
    }
}

void sound_callback(void* userdata, uint8_t* stream, int len) {
    (void)userdata; /* all warnings on, userdata is unused, suppress that warning */
    int16_t* buffer = (int16_t*)stream;
    for (int i=0; i<len/2; i++) {
        sound_step();
        buffer[i] = snd.out >> 1;
    }
}

void sound_init() {
    memset(&snd, 0, sizeof(snd));
    SDL_AudioSpec spec = {0};
    spec.freq = 48000;
    spec.format = AUDIO_S16SYS;
    spec.channels = 1;
    spec.samples = 4096;
    spec.callback = sound_callback;
    SDL_OpenAudio(&spec, 0);
    SDL_PauseAudio(0); 
}