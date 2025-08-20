#include <stdio.h>
#include <stdbool.h>
#include <stdlib.h>

static bool is_terminal = false;

#if !defined(WINDOWS)
#include <sys/select.h>
#include <unistd.h>
#include <termios.h>

static struct termios saved_tios;

static void exit_handler(void) {
    if (is_terminal)
        tcsetattr(0, TCSANOW, &saved_tios);
}

void serial_init(void) {
    struct termios tios;

    if (tcgetattr(0, &tios) != -1) {
        is_terminal = 1;
        saved_tios = tios;

        atexit(exit_handler);
    }

    if (is_terminal) {
        tios.c_lflag &= ~(ICANON|ECHO);
        tcsetattr(0, TCSANOW, &tios);
    }
}

int serial_get(void) {
    fd_set readfds;
    int fd = STDIN_FILENO;
    struct timeval tm = { 0, 0 };

    FD_ZERO(&readfds);
    FD_SET(fd, &readfds);

    int ready = select(fd + 1, &readfds, NULL, NULL, &tm);

    if (ready == 1 && FD_ISSET(fd, &readfds)) {
        char c;
        int ret = read(fd, &c, 1);

        if (ret == 1)
            return c;
    }

    return 0;
}

#elif defined(WINDOWS)
#include <Windows.h>

static DWORD old_mode;
static HANDLE hStdin;

static void exit_handler(void) {
    if (is_terminal)
        SetConsoleMode(hStdin, old_mode);
}

void serial_init(void) {
    hStdin = GetStdHandle(STD_INPUT_HANDLE);
    if (GetConsoleMode(hStdin, &old_mode)) {
        is_terminal = true;
        atexit(exit_handler);
        SetConsoleMode(hStdin, old_mode & ~(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT));
    }
}

int serial_get(void) {
    if (!is_terminal) {
        return 0;
    }
    
    DWORD events;
    INPUT_RECORD buffer;
    DWORD eventsRead;
    if (!GetNumberOfConsoleInputEvents(hStdin, &events) || events == 0) {
        return 0;
    }
    if (!ReadConsoleInput(hStdin, &buffer, 1, &eventsRead) || eventsRead == 0) {
        return 0;
    }
    if (buffer.EventType == KEY_EVENT && buffer.Event.KeyEvent.bKeyDown) {
        unsigned char c = buffer.Event.KeyEvent.uChar.AsciiChar;
        if (c != 0) {
            if (c == '\r') {
                // man why does Windows have to return a Carriage Return here
                // sh.fxf (and presumably other programs) only checks for a Line Feed
                return '\n';
            }
            return c;
        }
    }
    return 0;
}

#endif

void serial_put(int value) {
    putchar((int) value);
    fflush(stdout);
}
