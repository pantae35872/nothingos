.PHONY: debug release clean fat run

NAME := nothingos

iso:
	mkdir -p iso

run: debug
	qemu-system-x86_64 -cdrom os.iso -m 1G -bios OVMF.fd -drive id=disk,file=disk.disk,if=none,format=qcow2 -device ahci,id=ahci -device ide-hd,drive=disk,bus=ahci.0 -boot d -machine kernel_irqchip=split -serial stdio -S -s -no-reboot #-enable-kvm -cpu host,+rdrand

debug: fat iso
	cd src/kernel && cargo build
	cd src/bootloader && cargo build --target x86_64-unknown-uefi && cp target/x86_64-unknown-uefi/debug/$(NAME).efi target/x86_64-unknown-uefi/debug/bootx64.efi
	mcopy -i fat.img src/bootloader/target/x86_64-unknown-uefi/debug/bootx64.efi ::/efi/boot
	cp src/kernel/target/x86_64/debug/${NAME} kernel.bin
	stat -c %s kernel.bin > filesize.inf
	mcopy -i fat.img kernel.bin ::/boot 
	mcopy -i fat.img filesize.inf ::/boot
	cp fat.img iso
	xorriso -as mkisofs -R -f -e fat.img -no-emul-boot -o os.iso iso

release: fat iso
	cd src/kernel && cargo build --release
	cd src/bootloader && cargo build --release --target x86_64-unknown-uefi && cp target/x86_64-unknown-uefi/release/$(NAME).efi target/x86_64-unknown-uefi/release/bootx64.efi
	mcopy -i fat.img src/bootloader/target/x86_64-unknown-uefi/release/bootx64.efi ::/efi/boot
	cp src/kernel/target/x86_64/release/${NAME} kernel.bin
	stat -c %s kernel.bin > filesize.inf
	mcopy -i fat.img kernel.bin ::/boot 
	mcopy -i fat.img filesize.inf ::/boot
	cp fat.img iso
	xorriso -as mkisofs -R -f -e fat.img -no-emul-boot -o os.iso iso

fat:
	dd if=/dev/zero of=fat.img bs=1M count=16
	mkfs.vfat fat.img
	mmd -i fat.img ::/efi
	mmd -i fat.img ::/efi/boot
	mmd -i fat.img ::/boot

clean:
	cd src/bootloader && cargo clean
	cd src/kernel && cargo clean
	rm -rf iso
	rm -rf filesize.inf
	rm -rf kernel.bin
	rm -rf fat.img
	rm -rf os.iso
