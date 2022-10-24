#pragma once

typedef struct {
    uint16_t x, y;
    bool clicked;
    bool released;
    bool held;
} mouse_t;

void mouse_moved(int dx, int dy);
void mouse_pressed(int button);
void mouse_released(int button);
