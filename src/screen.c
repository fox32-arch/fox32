#include <SDL2/SDL.h>
#include <getopt.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "screen.h"

#define WINDOW_TITLE_UNGRABBED "fox32 emulator"
#define WINDOW_TITLE_GRABBED "fox32 emulator - strike F1 to uncapture mouse"

struct Screen MainScreen;

int WindowWidth = 0;
int WindowHeight = 0;

int ScreenZoom = 1;

bool ScreenFirstDraw = true;

bool ScreenMouseGrabbed = false;

SDL_Window *ScreenWindow;
SDL_Renderer *ScreenRenderer;

SDL_Rect WindowRect;

void ScreenInit() {
    ScreenWindow = SDL_CreateWindow(
        WINDOW_TITLE_UNGRABBED,
        SDL_WINDOWPOS_UNDEFINED, SDL_WINDOWPOS_UNDEFINED,
        (int)(WindowWidth * ScreenZoom),
        (int)(WindowHeight * ScreenZoom),
        SDL_WINDOW_HIDDEN
    );

    if (!ScreenWindow) {
        fprintf(stderr, "failed to create window\n");
        exit(1);
    }

    ScreenRenderer = SDL_CreateRenderer(ScreenWindow, -1, 0);

    if (!ScreenRenderer) {
        fprintf(stderr, "failed to create renderer\n");
        exit(1);
    }

    WindowRect = (SDL_Rect) {
        .w = WindowWidth,
        .h = WindowHeight
    };
}

void ScreenDraw() {
    MainScreen.Draw(MainScreen);

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
                if (ScreenMouseGrabbed) {
                    if (MainScreen.MouseMoved)
                        MainScreen.MouseMoved(MainScreen, event.motion.xrel, event.motion.yrel);
                }
                break;
            }

            case SDL_MOUSEBUTTONDOWN: {
                if (!ScreenMouseGrabbed) {
                    SDL_SetWindowGrab(ScreenWindow, true);
                    SDL_ShowCursor(false);
                    SDL_SetWindowTitle(ScreenWindow, WINDOW_TITLE_GRABBED);
                    SDL_SetRelativeMouseMode(true);
                    ScreenMouseGrabbed = true;
                    break;
                }

                if (MainScreen.MousePressed)
                    MainScreen.MousePressed(MainScreen, event.button.button);
                break;
            }


            case SDL_MOUSEBUTTONUP: {
                if (MainScreen.MouseReleased)
                    MainScreen.MouseReleased(MainScreen, event.button.button);
                break;
            }

            case SDL_KEYDOWN:
                if ((event.key.keysym.scancode == SDL_SCANCODE_F1) && ScreenMouseGrabbed) {
                    SDL_SetWindowGrab(ScreenWindow, false);
                    SDL_ShowCursor(true);
                    SDL_SetWindowTitle(ScreenWindow, WINDOW_TITLE_UNGRABBED);
                    SDL_SetRelativeMouseMode(false);
                    ScreenMouseGrabbed = false;
                    break;
                }

                if (MainScreen.KeyPressed)
                    MainScreen.KeyPressed(MainScreen, event.key.keysym.scancode);
                break;

            case SDL_KEYUP:
                if (MainScreen.KeyReleased)
                    MainScreen.KeyReleased(MainScreen, event.key.keysym.scancode);
                break;
        }
    }

    return 0;
}

struct SDL_Texture *ScreenGetTexture(struct Screen screen) {
    if (screen.Texture) {
        return screen.Texture;
    }

    screen.Texture = SDL_CreateTexture(
        ScreenRenderer,
        SDL_PIXELFORMAT_ABGR32,
        SDL_TEXTUREACCESS_STREAMING,
        screen.Width,
        screen.Height
    );

    SDL_SetTextureScaleMode(screen.Texture, SDL_ScaleModeNearest);

    return screen.Texture;
}

struct Screen ScreenCreate(
    int w, int h, char *title,
    ScreenDrawF draw,
    ScreenKeyPressedF keypressed,
    ScreenKeyReleasedF keyreleased,
    ScreenMousePressedF mousepressed,
    ScreenMouseReleasedF mousereleased,
    ScreenMouseMovedF mousemoved
) {

    if (w > WindowWidth)
        WindowWidth = w;

    if (h > WindowHeight)
        WindowHeight = h;

    MainScreen.Width = w;
    MainScreen.Height = h;
    MainScreen.Title = title;

    MainScreen.Draw = draw;
    MainScreen.KeyPressed = keypressed;
    MainScreen.KeyReleased = keyreleased;
    MainScreen.MousePressed = mousepressed;
    MainScreen.MouseReleased = mousereleased;
    MainScreen.MouseMoved = mousemoved;
}
