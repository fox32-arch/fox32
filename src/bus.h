#pragma once

int bus_io_read(void *user, uint32_t *value, uint32_t port);
int bus_io_write(void *user, uint32_t value, uint32_t port);
void drop_file(char *filename);
