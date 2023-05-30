#include <stdio.h>
#ifndef WINDOWS
#include <sys/select.h>
#include <unistd.h>
#include <termios.h>
#include <stdbool.h>
#include <stdlib.h>

static struct termios saved_tios;
static bool is_terminal = false;

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

#endif

void serial_put(int value) {
    putchar((int) value);
    fflush(stdout);
}
