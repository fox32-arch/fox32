#pragma once

#include <SDL2/SDL.h>

struct Screen;

typedef void (*ScreenDrawF)(struct Screen *screen);
typedef void (*ScreenKeyPressedF)(int sdl_scancode);
typedef void (*ScreenKeyReleasedF)(int sdl_scancode);
typedef void (*ScreenMousePressedF)(int button);
typedef void (*ScreenMouseReleasedF)(int button);
typedef void (*ScreenMouseMovedF)(int dx, int dy);
typedef void (*ScreenDropFileF)(char *filename);

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
    ScreenDropFileF DropFile;
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
    ScreenMouseMovedF mousemoved,
    ScreenDropFileF dropfile
);
