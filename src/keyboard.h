#pragma once

typedef unsigned char key_t;

key_t key_take(void);
void key_put(key_t code);

key_t key_convert(int sdlcode);
void key_pressed(int sdlcode);
void key_released(int sdlcode);
