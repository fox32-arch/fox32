#pragma once

typedef struct {
    uint32_t virtual_page;
    uint32_t physical_address;
    bool present;
    bool rw;
} mmu_page_t;

void set_and_flush_tlb(uint32_t virtual_address);
void flush_single_page(uint32_t virtual_address);
mmu_page_t *get_present_page(uint32_t virtual_address);
bool insert_tlb_entry_from_tables(uint32_t page_directory_index, uint32_t page_table_index);
