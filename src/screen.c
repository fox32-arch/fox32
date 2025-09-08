#include <SDL2/SDL.h>
#include <getopt.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "screen.h"

struct Screen MainScreen;

int WindowWidth = 0;
int WindowHeight = 0;

int ScreenScale = SCREEN_ZOOM;

bool ScreenFirstDraw = true;

SDL_Window *ScreenWindow;
SDL_Renderer *ScreenRenderer;

SDL_Rect WindowRect;

void ScreenInit() {
    ScreenWindow = SDL_CreateWindow(
        "fox32 emulator",
        SDL_WINDOWPOS_UNDEFINED, SDL_WINDOWPOS_UNDEFINED,
        (int)(WindowWidth * ScreenScale),
        (int)(WindowHeight * ScreenScale),
        SDL_WINDOW_HIDDEN | SDL_WINDOW_ALLOW_HIGHDPI
    );

    if (!ScreenWindow) {
        fprintf(stderr, "failed to create window\n");
        exit(1);
    }

    SDL_SetHint(SDL_HINT_WINDOWS_DPI_AWARENESS, "permonitor");
    SDL_SetHint(SDL_HINT_WINDOWS_DPI_SCALING, "1");

    // set scale filtering mode
    char filtering[2];
    sprintf(filtering, "%d", MainScreen.ScaleFiltering);
    SDL_SetHint(SDL_HINT_RENDER_SCALE_QUALITY, filtering); // 0: point, 1 = linear

    ScreenRenderer = SDL_CreateRenderer(ScreenWindow, -1, 0);

    if (!ScreenRenderer) {
        fprintf(stderr, "failed to create renderer\n");
        exit(1);
    }

    SDL_RenderSetLogicalSize(ScreenRenderer, WindowWidth, WindowHeight);

    WindowRect = (SDL_Rect) {
        .w = WindowWidth,
        .h = WindowHeight
    };
}

void ScreenDraw() {
    MainScreen.Draw(&MainScreen);

    SDL_Rect screenrect = {
        .w = MainScreen.Width,
        .h = MainScreen.Height,
    };

    SDL_Rect winrect = {
        .w = MainScreen.Width,
        .h = MainScreen.Height,
        .x = 0,
        .y = 0
    };

    if ((WindowRect.w != screenrect.w) || (WindowRect.h != screenrect.h)) {
        int oldx;
        int oldy;

        SDL_GetWindowPosition(ScreenWindow, &oldx, &oldy);

        oldx += (WindowRect.w - screenrect.w)/2;
        oldy += (WindowRect.h - screenrect.h)/2;

        SDL_SetWindowSize(ScreenWindow, screenrect.w, screenrect.h);
        SDL_SetWindowPosition(ScreenWindow, oldx, oldy);

        WindowRect.w = screenrect.w;
        WindowRect.h = screenrect.h;
    }

    SDL_RenderClear(ScreenRenderer);
    SDL_RenderCopy(ScreenRenderer, MainScreen.Texture, &screenrect, &winrect);
    SDL_RenderPresent(ScreenRenderer);

    if (ScreenFirstDraw) {
        SDL_ShowWindow(ScreenWindow);
        ScreenFirstDraw = false;
    }
}

int ScreenProcessEvents() {
    SDL_Event event;
    while (SDL_PollEvent(&event)) {
        switch (event.type) {
            case SDL_QUIT: {
                return 1;
            }

            case SDL_WINDOWEVENT: {
                break;
            }

            case SDL_MOUSEMOTION: {
                if (MainScreen.MouseMoved)
                    MainScreen.MouseMoved(event.motion.x, event.motion.y);
                break;
            }

            case SDL_MOUSEBUTTONDOWN: {
                if (MainScreen.MousePressed)
                    MainScreen.MousePressed(event.button.button);
                break;
            }

            case SDL_MOUSEBUTTONUP: {
                if (MainScreen.MouseReleased)
                    MainScreen.MouseReleased(event.button.button);
                break;
            }

            case SDL_KEYDOWN:
                if (MainScreen.KeyPressed)
                    MainScreen.KeyPressed(event.key.keysym.scancode);
                break;

            case SDL_KEYUP:
                if (MainScreen.KeyReleased)
                    MainScreen.KeyReleased(event.key.keysym.scancode);
                break;

            case SDL_DROPFILE:
                if (MainScreen.DropFile) {
                    char *file = event.drop.file;
                    MainScreen.DropFile(file);
                    SDL_free(file);
                }
                break;
        }
    }

    return 0;
}

struct SDL_Texture *ScreenGetTexture(struct Screen *screen) {
    if (screen->Texture) {
        return screen->Texture;
    }

    screen->Texture = SDL_CreateTexture(
        ScreenRenderer,
        SDL_PIXELFORMAT_ABGR8888,
        SDL_TEXTUREACCESS_STREAMING,
        screen->Width,
        screen->Height
    );

    return screen->Texture;
}

void ScreenCreate(
    int w, int h,
    int filtering,
    ScreenDrawF draw,
    ScreenKeyPressedF keypressed,
    ScreenKeyReleasedF keyreleased,
    ScreenMousePressedF mousepressed,
    ScreenMouseReleasedF mousereleased,
    ScreenMouseMovedF mousemoved,
    ScreenDropFileF dropfile
) {

    if (w > WindowWidth)
        WindowWidth = w;

    if (h > WindowHeight)
        WindowHeight = h;

    MainScreen.Width = w;
    MainScreen.Height = h;
    MainScreen.ScaleFiltering = filtering;
    MainScreen.Draw = draw;
    MainScreen.KeyPressed = keypressed;
    MainScreen.KeyReleased = keyreleased;
    MainScreen.MousePressed = mousepressed;
    MainScreen.MouseReleased = mousereleased;
    MainScreen.MouseMoved = mousemoved;
    MainScreen.DropFile = dropfile;
}
