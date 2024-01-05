section .multiboot_header
header_start:
    dd 0xe85250d6                ; magic number (multiboot 2)
    dd 0                         ; architecture 0 (protected mode i386)
    dd header_end - header_start ; header length
    ; checksum
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))
    
    ;align 8
    ;dw 5
    ;dw 0 ;instead of 0, you can specify your flags
    ;dd 20
    ;dd 1024 ; 160
    ;dd 768 ; 100
    ;dd 32 ;instead of 32, you can specify your BPP
    ;align 8
    ; required end tag
    dw 0    ; type
    dw 0    ; flags
    dd 8    ; size
header_end:
