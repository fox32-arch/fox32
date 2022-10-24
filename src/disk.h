#pragma once

typedef struct {
    FILE *file;
    uint64_t size;
} disk_t;

typedef struct {
    disk_t disks[4];
    size_t buffer_pointer;
} disk_controller_t;

void new_disk(const char *filename, size_t id);
void insert_disk(disk_t disk, size_t id);
void remove_disk(size_t id);
uint64_t get_disk_size(size_t id);
void set_disk_sector(size_t id, uint64_t sector);
size_t read_disk_into_memory(size_t id);
