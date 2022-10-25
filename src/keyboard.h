#pragma once

typedef unsigned char keycode_t;

keycode_t key_take(void);
void key_put(keycode_t code);

keycode_t key_convert(int sdlcode);
void key_pressed(int sdlcode);
void key_released(int sdlcode);
