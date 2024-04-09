.PHONY: run debug release 

NAME := nothingos

run:
	qemu-system-x86_64 -cdrom os.iso -m 1G -bios OVMF.fd -drive id=disk,file=disk.disk,if=none,format=qcow2 -device ahci,id=ahci -device ide-hd,drive=disk,bus=ahci.0 -boot d -enable-kvm -cpu host,+rdrand -machine kernel_irqchip=split -serial stdio

debug:
	cargo build
	cp target/x86_64/debug/${NAME} iso/boot/kernel.bin
	grub-mkrescue -o os.iso iso

release:
	cargo build --release
	cp target/x86_64/release/${NAME} iso/boot/kernel.bin
	grub-mkrescue -o os.iso iso
