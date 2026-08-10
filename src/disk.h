#pragma once

#include <SDL2/SDL_thread.h>

#define DISK_INTERRUPT_VECTOR 0xF7

typedef struct {
    FILE *file;
    uint64_t size;
    uint32_t previous_seek_offset;
} disk_t;

typedef struct {
    disk_t disks[4];
    size_t buffer_pointer;
} disk_controller_t;

SDL_Thread *start_disk_worker();
void exit_disk_worker();
bool is_disk_interrupt_pending(bool pop);

void new_disk(const char *filename, size_t id);
void insert_disk(disk_t disk, size_t id);
void remove_disk(size_t id);
uint64_t get_disk_size(size_t id);
void set_disk_sector(size_t id, uint64_t sector);
size_t read_disk_into_memory(size_t id);
size_t write_disk_from_memory(size_t id);
