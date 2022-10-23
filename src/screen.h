#pragma once

#include <SDL2/SDL.h>

struct Screen;

typedef void (*ScreenDrawF)(struct Screen *screen);
typedef void (*ScreenKeyPressedF)(struct Screen *screen, int sdlscancode);
typedef void (*ScreenKeyReleasedF)(struct Screen *screen, int sdlscancode);
typedef void (*ScreenMousePressedF)(struct Screen *screen, int button);
typedef void (*ScreenMouseReleasedF)(struct Screen *screen, int button);
typedef void (*ScreenMouseMovedF)(struct Screen *screen, int dx, int dy);

struct Screen {
    int Width;
    int Height;
    SDL_Texture *Texture;

    ScreenDrawF Draw;
    ScreenKeyPressedF KeyPressed;
    ScreenKeyReleasedF KeyReleased;
    ScreenMousePressedF MousePressed;
    ScreenMouseReleasedF MouseReleased;
    ScreenMouseMovedF MouseMoved;
};

void ScreenInit();

void ScreenDraw();

int ScreenProcessEvents();

struct SDL_Texture *ScreenGetTexture(struct Screen *screen);

void ScreenCreate(
    int w, int h,
    ScreenDrawF draw,
    ScreenKeyPressedF keypressed,
    ScreenKeyReleasedF keyreleased,
    ScreenMousePressedF mousepressed,
    ScreenMouseReleasedF mousereleased,
    ScreenMouseMovedF mousemoved
);
