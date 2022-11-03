SECTION .text
USE16

stage2.entry:
    ; check for required features
    call cpuid_check

    ; enable A20-Line via IO-Port 92, might not work on all motherboards
    in al, 0x92
    or al, 2
    out 0x92, al

    mov dword [protected_mode.func], stage3.entry
    jmp protected_mode.entry

%include "cpuid.asm"
%include "gdt.asm"
%include "long_mode.asm"
%include "protected_mode.asm"
%include "thunk.asm"

USE32
stage3.load:
    ; Load address
    mov edi, 0x100000
    ; LBA
    mov eax, (stage3 - stage1) / 512
    ; Count
    mov ecx, (stage3.end - stage3) / 512

.loop:
    push ecx
    cmp ecx, 127
    jbe .good_size
    mov ecx, 127

.good_size:
    ; Load at 0x70000 (512KiB minus 64KiB disk buffer)
    mov ebx, 0 ; Lower 4 bits of address should be zero
    mov edx, 0x7000 ; Segment of address

    mov dword [THUNK_ES], 0
    mov dword [THUNK_EDI], 0
    mov dword [THUNK_ESI], 0
    mov dword [THUNK_EBP], 0
    mov dword [THUNK_EBX], ebx
    mov dword [THUNK_EDX], edx
    mov dword [THUNK_ECX], ecx
    mov dword [THUNK_EAX], eax
    mov dword [thunk.func], load
    call thunk.enter
    ; TODO: evaluate result!

    ; Copy from disk buffer to load address
    mov esi, 0x70000
    shl ecx, 9 ; Multiply sectors by 512
    cld
    rep movsb

    pop ecx
    cmp ecx, 127
    jbe .finished
    add eax, 127
    sub ecx, 127
    jmp .loop

.finished:
    ret

stage3.entry:
    ; stage3 stack at 448 KiB (512KiB minus 64KiB disk buffer)
    mov esp, 0x70000

    call stage3.load

    ; push arguments
    mov eax, thunk.int16
    push eax
    mov eax, thunk.int15
    push eax
    mov eax, thunk.int13
    push eax
    mov eax, thunk.int10
    push eax
    xor eax, eax
    mov al, [disk]
    push eax
    mov eax, kernel.entry
    push eax
    mov eax, [0x100000 + 0x18]
    call eax
.halt:
    cli
    hlt
    jmp .halt

kernel:
.stack: dq 0
.func: dq 0
.args: dq 0

.entry:
    ; page_table: usize
    mov eax, [esp + 4]
    mov [long_mode.page_table], eax

    ; stack: u64
    mov eax, [esp + 8]
    mov [.stack], eax
    mov eax, [esp + 12]
    mov [.stack + 4], eax

    ; func: u64
    mov eax, [esp + 16]
    mov [.func], eax
    mov eax, [esp + 20]
    mov [.func + 4], eax

    ; args: *const KernelArgs
    mov eax, [esp + 24]
    mov [.args], eax

    ; long_mode: usize
    mov eax, [esp + 28]
    test eax, eax
    jz .inner32

    mov eax, .inner64
    mov [long_mode.func], eax
    jmp long_mode.entry

.inner32:
    ; disable paging
    mov eax, cr0
    and eax, 0x7FFFFFFF
    mov cr0, eax

    ;TODO: PAE (1 << 5)
    ; enable FXSAVE/FXRSTOR, Page Global, and Page Size Extension
    mov eax, cr4
    or eax, 1 << 9 | 1 << 7 | 1 << 4
    mov cr4, eax

    ; set page table
    mov eax, [long_mode.page_table]
    mov cr3, eax

    ; enabling paging and protection simultaneously
    mov eax, cr0
    ; Bit 31: Paging, Bit 16: write protect kernel, Bit 0: Protected Mode
    or eax, 1 << 31 | 1 << 16 | 1
    mov cr0, eax
    
    ; enable FPU
    ;TODO: move to Rust
    mov eax, cr0
    and al, 11110011b ; Clear task switched (3) and emulation (2)
    or al, 00100010b ; Set numeric error (5) monitor co-processor (1)
    mov cr0, eax
    fninit

    mov esp, [.stack]
    mov eax, [.args]
    push eax
    mov eax, [.func]
    call eax
.halt32:
    cli
    hlt
    jmp .halt32

USE64

.inner64:
    mov rsp, [.stack]
    mov rax, [.func]
    mov rdi, [.args]
    call rax
.halt64:
    cli
    hlt
    jmp .halt64
