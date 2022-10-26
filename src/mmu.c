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
#include "mmu.h"

mmu_page_t mmu_tlb[64];

extern fox32_vm_t vm;

static size_t find_free_tlb_entry_index() {
    for (size_t i = 0; i < 64; i++) {
        if (!mmu_tlb[i].present) {
            return i;
        }
    }
    return 0;
}

void set_and_flush_tlb(uint32_t virtual_address) {
    vm.pointer_page_directory = virtual_address;
    for (size_t i = 0; i < 64; i++) {
        mmu_tlb[i] = (mmu_page_t) {
            .physical_address = 0,
            .virtual_page = 0,
            .present = false,
            .rw = false
        };
    }
    //printf("flushed TLB and set page directory pointer to %X\n", virtual_address);
}

void flush_single_page(uint32_t virtual_address) {
    uint32_t virtual_page = virtual_address & 0xFFFFF000;
    //printf("flushing single page %X\n", virtual_page);
    for (size_t i = 0; i < 64; i++) {
        if (mmu_tlb[i].virtual_page == virtual_page) {
            mmu_tlb[i].physical_address = 0;
            mmu_tlb[i].virtual_page = 0;
            mmu_tlb[i].present = false;
            mmu_tlb[i].rw = false;
            //printf("flushed\n");
            break;
        }
    }
}

mmu_page_t *get_present_page(uint32_t virtual_address) {
    uint32_t virtual_page = virtual_address & 0xFFFFF000;
    //printf("attempting to fetch physical address for virtual address %X (page %X)\n", virtual_address, virtual_page);
    mmu_page_t *physical_page = NULL;
    for (size_t i = 0; i < 64; i++) {
        if (mmu_tlb[i].virtual_page == virtual_page) {
            physical_page = &mmu_tlb[i];
            break;
        }
    }
    if (physical_page == NULL) {
        // we didn't find an entry for this page in the TLB, try to insert it from the tables in memory
        //printf("couldn't find an entry for this virtual page in the TLB, attempting to cache from tables\n");
        uint32_t page_directory_index = virtual_address >> 22;
        uint32_t page_table_index = (virtual_address >> 12) & 0x03FF;
        bool directory_present = insert_tlb_entry_from_tables(page_directory_index, page_table_index);
        if (!directory_present) return NULL;
        // try again after possibly inserting the TLB entry
        for (size_t i = 0; i < 64; i++) {
            if (mmu_tlb[i].virtual_page == virtual_page) {
                physical_page = &mmu_tlb[i];
                break;
            }
        }
        // if we still can't find the page, return NULL
        if (physical_page == NULL) return NULL;
    }
    // if the page is present, return it, otherwise return NULL
    if (physical_page->present) {
        //printf("found physical address: %X\n", physical_page->physical_address);
        return physical_page;
    } else {
        return NULL;
    }
}

bool insert_tlb_entry_from_tables(uint32_t page_directory_index, uint32_t page_table_index) {
    uint32_t directory =
        vm.memory_ram[vm.pointer_page_directory + (page_directory_index * 4)] |
        vm.memory_ram[vm.pointer_page_directory + (page_directory_index * 4) + 1] <<  8 |
        vm.memory_ram[vm.pointer_page_directory + (page_directory_index * 4) + 2] << 16 |
        vm.memory_ram[vm.pointer_page_directory + (page_directory_index * 4) + 3] << 24;
    //printf("operating on directory at %X\n", vm.pointer_page_directory + (page_directory_index * 4));
    bool directory_present = (directory & 0b1) != 0;
    uint32_t directory_address = directory & 0xFFFFF000;
    //printf("directory_present: %X, directory_address: %X\n", directory_present, directory_address);
    if (directory_present) {
        uint32_t table =
            vm.memory_ram[directory_address + (page_table_index * 4)] |
            vm.memory_ram[directory_address + (page_table_index * 4) + 1] <<  8 |
            vm.memory_ram[directory_address + (page_table_index * 4) + 2] << 16 |
            vm.memory_ram[directory_address + (page_table_index * 4) + 3] << 24;
        bool table_present = (table & 0b01) != 0;
        bool table_rw = (table & 0b10) != 0;
        uint32_t table_address = table & 0xFFFFF000;
        if (table_present) {
            mmu_page_t entry = {
                .physical_address = table_address,
                .virtual_page = (page_directory_index << 22) | (page_table_index << 12),
                .present = table_present,
                .rw = table_rw
            };
            size_t entry_index = find_free_tlb_entry_index();
            mmu_tlb[entry_index] = entry;
            //printf("inserting virtual page %X into the TLB\n", (page_directory_index << 22) | (page_table_index << 12));
        }
    }
    return directory_present;
}
