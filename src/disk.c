#include <SDL2/SDL.h>
#include <getopt.h>
#include <math.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#include <emscripten/html5.h>
#endif

#include <SDL2/SDL_thread.h>

#include <unistd.h>

#include "cpu.h"
#include "disk.h"

#include "log.h"

disk_controller_t disk_controller;

extern fox32_vm_t vm;
extern bool should_log;

#ifdef __EMSCRIPTEN__
EMSCRIPTEN_KEEPALIVE
#endif

// some kind of magneto-optical disk
#define RPM 800.0
#define RPS (RPM/60.0)
#define MS_PER_ROTATION (1000.0/RPS)

#define SECTORS_PER_TRACK 17
#define TRACKS_PER_CYLINDER 1

#define SETTLE_TIME_MS 3.0

#define SEEK_PER_CYL_MS 0.1

#define SECTOR_PER_MS (SECTORS_PER_TRACK / MS_PER_ROTATION)

#define LBA_TO_SECTOR(_lba) ((_lba) % SECTORS_PER_TRACK)
#define LBA_TO_TRACK(_lba) ((_lba) / SECTORS_PER_TRACK)
#define LBA_TO_CYLINDER(_lba) (LBA_TO_TRACK(_lba) / TRACKS_PER_CYLINDER)


enum disk_worker_commands_e {
    IDLE = 0,
    READ,
    WRITE,
    EXIT,
};

struct disk_worker_mailbox_s {
    uint8_t command;
    uint32_t data;
    uint32_t dest_ptr;
    uint32_t ret;
} disk_worker_mailbox;
static SDL_mutex *disk_worker_mutex;

// stolen from xremu https://github.com/xrarch/xremu/blob/4050c40271198b7e946df3d36c70750d38eb880d/src/dks.c
// this is probably extremely incorrect, i think i fucked this up. but it seems to produce good enough timing
double calculate_delay(uint32_t disk_id, uint32_t prev_lba) {
    uint32_t current_lba = ftell(disk_controller.disks[disk_id].file) / 512;
    double cylseek = ((int) LBA_TO_CYLINDER(current_lba) - (int) LBA_TO_CYLINDER(prev_lba));

    if (cylseek < 0) {
        // The head has to seek either back or forth so this should be an
        // absolute value to reflect the cylinder distance.
        cylseek = -cylseek;
    }

    // Multiply by the number of milliseconds it takes to seek one cylinder.
    cylseek *= SEEK_PER_CYL_MS;

    if (LBA_TO_CYLINDER(current_lba) != LBA_TO_CYLINDER(prev_lba)) {
        // We moved cylinders, so add a settle time.
        cylseek += SETTLE_TIME_MS;
    }

    // Calculate how many sectors the platter has to rotate by.
    double sectorseek = ((int)LBA_TO_SECTOR(current_lba) - (int)LBA_TO_SECTOR(prev_lba));

    if (sectorseek < 0) {
        // The platter is circular, so a negative sector seek means that the
        // sector is "behind". Therefore we have to wait that many sectors less
        // than a full rotation.
        sectorseek += SECTORS_PER_TRACK;
    }

    // Divide by the number of sectors that the platter rotates by per
    // millisecond.
    sectorseek /= SECTOR_PER_MS;

    // Set the operation interval to the platter rotation time plus the
    // head seek time.
    return sectorseek + cylseek;
}
int disk_worker(void *data) {
    struct disk_worker_mailbox_s *mailbox = data;
    while (true) {
        SDL_LockMutex(disk_worker_mutex);
        uint8_t command = mailbox->command;
        SDL_UnlockMutex(disk_worker_mutex);
        switch (command) {
            case IDLE:
                break;
            case READ: {
                SDL_LockMutex(disk_worker_mutex);
                (void) fread(&vm.memory_ram[mailbox->dest_ptr], 1, 512, disk_controller.disks[mailbox->data].file);
                mailbox->command = IDLE;

                // wait a bit to simulate seek times
                int ms = (int) calculate_delay(mailbox->data, disk_controller.disks[mailbox->data].previous_seek_offset / 512);
                SDL_UnlockMutex(disk_worker_mutex);
                struct timespec ts = {
                    ms / 1000, /* seconds */
                    (ms % 1000) * 1000 * 1000 /* nano seconds */
                };
                nanosleep(&ts, NULL);

                // signal that we're done
                SDL_LockMutex(disk_worker_mutex);
                mailbox->ret++;
                SDL_UnlockMutex(disk_worker_mutex);

                break;
            }
            case WRITE: {
                SDL_LockMutex(disk_worker_mutex);
                fwrite(&vm.memory_ram[mailbox->dest_ptr], 1, 512, disk_controller.disks[mailbox->data].file);
                mailbox->command = IDLE;

                // wait a bit to simulate seek times
                int ms = (int) calculate_delay(mailbox->data, disk_controller.disks[mailbox->data].previous_seek_offset / 512);
                SDL_UnlockMutex(disk_worker_mutex);
                struct timespec ts = {
                    ms / 1000, /* seconds */
                    (ms % 1000) * 1000 * 1000 /* nano seconds */
                };
                nanosleep(&ts, NULL);

                // signal that we're done
                SDL_LockMutex(disk_worker_mutex);
                mailbox->ret++;
                SDL_UnlockMutex(disk_worker_mutex);
                break;
            }
            case EXIT:
                return 0;
        }
    }
}

SDL_Thread *start_disk_worker() {
    disk_worker_mailbox.command = IDLE;
    disk_worker_mailbox.ret = 0;
    disk_worker_mutex = SDL_CreateMutex();
    return SDL_CreateThread(disk_worker, "DiskWorker", &disk_worker_mailbox);
}
void exit_disk_worker() {
    SDL_LockMutex(disk_worker_mutex);
    disk_worker_mailbox.command = EXIT;
    SDL_UnlockMutex(disk_worker_mutex);
    SDL_DestroyMutex(disk_worker_mutex);
}
bool is_disk_interrupt_pending(bool pop) {
    SDL_LockMutex(disk_worker_mutex);
    bool is_pending = disk_worker_mailbox.ret != 0;
    if (pop) disk_worker_mailbox.ret = 0;
    SDL_UnlockMutex(disk_worker_mutex);
    return is_pending;
}

void new_disk(const char *filename, size_t id) {
    if (id > 3) { LOG0("attempting to insert disk with ID > 3"); return; }
    LOG("inserting %s as disk ID %d\n", filename, (int) id);
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
    if (id > 3) { LOG0("attempting to insert disk with ID > 3"); return; }
    if (disk_controller.disks[id].size > 0) remove_disk(id);
    LOG("inserting disk ID %d\n", (int) id);
    disk_controller.disks[id] = disk;
}

#ifdef __EMSCRIPTEN__
EMSCRIPTEN_KEEPALIVE
#endif
void remove_disk(size_t id) {
    if (id > 3) { LOG0("attempting to remove disk with ID > 3"); return; }
    if (disk_controller.disks[id].file) {
        LOG("removing disk ID %d\n", (int) id);
        fclose(disk_controller.disks[id].file);
        disk_controller.disks[id].file = NULL;
        disk_controller.disks[id].size = 0;
    }
}

uint64_t get_disk_size(size_t id) {
    if (id > 3) { LOG0("attempting to access disk size with ID > 3"); return 0; }
    return disk_controller.disks[id].size;
}

void set_disk_sector(size_t id, uint64_t sector) {
    if (id > 3) { LOG0("attempting to set disk sector with ID > 3"); return; }
    if (disk_controller.disks[id].file) {
        SDL_LockMutex(disk_worker_mutex);
        disk_controller.disks[id].previous_seek_offset = ftell(disk_controller.disks[id].file);
        fseek(disk_controller.disks[id].file, sector * 512, 0);
        SDL_UnlockMutex(disk_worker_mutex);
    }
}

size_t read_disk_into_memory(size_t id) {
    if (id > 3) { LOG0("attempting to read disk with ID > 3"); return 0; }
    if (disk_controller.disks[id].file) {
        //return fread(&vm.memory_ram[disk_controller.buffer_pointer], 1, 512, disk_controller.disks[id].file);
        SDL_LockMutex(disk_worker_mutex);
        disk_worker_mailbox.data = id;
        disk_worker_mailbox.dest_ptr = disk_controller.buffer_pointer;
        disk_worker_mailbox.command = READ;
        SDL_UnlockMutex(disk_worker_mutex);
        return 1;
    } else {
        return 0;
    }
}

size_t write_disk_from_memory(size_t id) {
    if (id > 3) { LOG0("attempting to write disk with ID > 3"); return 0; }
    if (disk_controller.disks[id].file) {
        //return fwrite(&vm.memory_ram[disk_controller.buffer_pointer], 1, 512, disk_controller.disks[id].file);
        SDL_LockMutex(disk_worker_mutex);
        disk_worker_mailbox.data = id;
        disk_worker_mailbox.dest_ptr = disk_controller.buffer_pointer;
        disk_worker_mailbox.command = WRITE;
        SDL_UnlockMutex(disk_worker_mutex);
        return 1;
    } else {
        return 0;
    }
}
