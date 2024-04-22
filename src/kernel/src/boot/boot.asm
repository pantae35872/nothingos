global boot_start
extern start

section .text
bits 64
boot_start:
  mov rsp, stack_top
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
  mov eax, p4_table
  or eax, 0b11 
  mov [p4_table + 511 * 8], eax

  mov eax, p3_table
  or eax, 0b11 
  mov [p4_table], eax

  mov rcx, 0

.map_p3_table
  mov rsi, 4096
  imul rsi, rcx
  mov eax, p2_table
  add rax, rsi
  or eax, 0b11
  mov [p3_table + rcx * 8], eax
  inc rcx
  mov rdx, [rdi]
  cmp rcx, rdx
  jne .map_p3_table


  mov rbx, 0
  mov rdx, p2_table
.map_p2_1g
  mov rcx, 0 

  .map_p2_table:
    mov eax, 0x200000 
    mov rsi, rdx
    mul rcx
    push rsi
    push rax
    mov rax, 0x40000000
    mul rbx
    mov rsi, rax 
    pop rax
    add rax, rsi
    or eax, 0b10000011
    pop rsi
    mov [rsi + rcx * 8], eax
    mov rdx, rsi

    inc rcx           
    cmp rcx, 512       
    jne .map_p2_table 

  inc rbx
  mov rsi, 4096
  imul rsi, rbx
  mov rax, p2_table
  add rax, rsi
  mov rdx, rax
  mov rax, [rdi]
  cmp rbx, rax
  jne .map_p2_1g
  ret

enable_paging:
  mov rax, p4_table
  mov cr3, rax

  ret

section .bss
align 4096
p4_table:
  resb 4096
p3_table:
  resb 4096
p2_table:
  resb 4096 * 32
stack_bottom:
  resb 4096 * 32
stack_top:
