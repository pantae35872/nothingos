global boot_start
extern start

section .text
bits 64
boot_start:
  call set_up_page_tables
  call enable_paging
  mov ax, 0
  mov ss, ax
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax
  jmp start
  hlt

set_up_page_tables:
  ; map P4 table recursively
  mov eax, p4_table
  or eax, 0b11 ; present + writable
  mov [p4_table + 511 * 8], eax

  ; map first P4 entry to P3 table
  mov eax, p3_table
  or eax, 0b11 ; present + writable
  mov [p4_table], eax

  ; map first P3 entry to P2 table
  mov eax, p2_table
  or eax, 0b11 ; present + writable
  mov [p3_table], eax

  ; map each P2 entry to a huge 2MiB page
  mov ecx, 0         ; counter variable

.map_p2_table:
    ; map ecx-th P2 entry to a huge page that starts at address 2MiB*ecx
  mov eax, 0x200000  ; 2MiB
  mul ecx            ; start address of ecx-th page
  or eax, 0b10000011 ; present + writable + huge
  mov [p2_table + ecx * 8], eax ; map ecx-th entry

  inc ecx            ; increase counter
  cmp ecx, 512       ; if counter == 512, the whole P2 table is mapped
  jne .map_p2_table  ; else map the next entry

  ret

enable_paging:
  ; load P4 to cr3 register (cpu uses this to access the P4 table)
  mov rax, p4_table
  mov cr3, rax

  ; enable PAE-flag in cr4 (Physical Address Extension)
  mov rax, cr4
  or rax, 1 << 5
  mov cr4, rax

  ; enable paging in the cr0 register
  ;mov rax, cr0
  ;or rax, 1 << 31
  ;mov cr0, rax

  ret

section .bss
align 4096
p4_table:
  resb 4096
p3_table:
  resb 4096
p2_table:
  resb 4096