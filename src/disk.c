#include <SDL2/SDL.h>
#include <getopt.h>
#include <math.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cpu.h"
#include "disk.h"

disk_controller_t disk_controller;

extern fox32_vm_t vm;

void new_disk(const char *filename, size_t id) {
    if (id > 3) { puts("attempting to insert disk with ID > 3"); return; }
    printf("inserting %s as disk ID %d\n", filename, (int) id);
    disk_controller.disks[id].file = fopen(filename, "r+b");
    if (!disk_controller.disks[id].file) {
        fprintf(stderr, "couldn't open disk file\n");
        exit(1);
    }
    fseek(disk_controller.disks[id].file, 0, SEEK_END);
    disk_controller.disks[id].size = ftell(disk_controller.disks[id].file);
    rewind(disk_controller.disks[id].file);
}

void insert_disk(disk_t disk, size_t id) {
    if (id > 3) { puts("attempting to insert disk with ID > 3"); return; }
    if (disk_controller.disks[id].size > 0) remove_disk(id);
    printf("inserting disk ID %d\n", (int) id);
    disk_controller.disks[id] = disk;
}

void remove_disk(size_t id) {
    if (id > 3) { puts("attempting to remove disk with ID > 3"); return; }
    if (disk_controller.disks[id].file) {
        printf("removing disk ID %d\n", (int) id);
        fclose(disk_controller.disks[id].file);
        disk_controller.disks[id].file = NULL;
        disk_controller.disks[id].size = 0;
    }
}

uint64_t get_disk_size(size_t id) {
    if (id > 3) { puts("attempting to access disk size with ID > 3"); return 0; }
    return disk_controller.disks[id].size;
}

void set_disk_sector(size_t id, uint64_t sector) {
    if (id > 3) { puts("attempting to set disk sector with ID > 3"); return; }
    if (disk_controller.disks[id].file) {
        fseek(disk_controller.disks[id].file, sector * 512, 0);
    }
}

size_t read_disk_into_memory(size_t id) {
    if (id > 3) { puts("attempting to read disk with ID > 3"); return 0; }
    if (disk_controller.disks[id].file) {
        return fread(&vm.memory_ram[disk_controller.buffer_pointer], 1, 512, disk_controller.disks[id].file);
    } else {
        return 0;
    }
}

size_t write_disk_from_memory(size_t id) {
    if (id > 3) { puts("attempting to write disk with ID > 3"); return 0; }
    if (disk_controller.disks[id].file) {
        return fwrite(&vm.memory_ram[disk_controller.buffer_pointer], 1, 512, disk_controller.disks[id].file);
    } else {
        return 0;
    }
}
