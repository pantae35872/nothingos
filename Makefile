ifeq (test,$(firstword $(MAKECMDGOALS)))
  RUN_ARGS := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
  $(eval $(RUN_ARGS):;@:)
endif

.PHONY: debug release clean fat run maker-no-kernel os-runner test-run test

NAME := nothingos

iso:
	mkdir -p iso

run: 
	qemu-system-x86_64 -cdrom os.iso -m 1G -bios OVMF.fd -drive id=disk,file=disk.disk,if=none,format=qcow2 -device ahci,id=ahci -device ide-hd,drive=disk,bus=ahci.0 -boot d -machine kernel_irqchip=split -no-reboot -enable-kvm -cpu host,+rdrand

test-run:
	qemu-system-x86_64 -cdrom os.iso -m 1G -bios OVMF.fd -serial stdio -drive id=disk,file=disk.disk,if=none,format=qcow2 -device isa-debug-exit,iobase=0xf4,iosize=0x04 -device ahci,id=ahci -device ide-hd,drive=disk,bus=ahci.0 -boot d -machine kernel_irqchip=split -no-reboot -display none -enable-kvm -cpu host,+rdrand

os-runner:
	@cd src/os-runner && cargo build --release --quiet
	@cp src/os-runner/target/release/os-runner os-runner

maker-no-kernel: fat
	@cd src/bootloader && cargo build --target x86_64-unknown-uefi && cp target/x86_64-unknown-uefi/debug/$(NAME).efi target/x86_64-unknown-uefi/debug/bootx64.efi
	@mcopy -i fat.img src/bootloader/target/x86_64-unknown-uefi/debug/bootx64.efi ::/efi/boot
	@mcopy -i fat.img kernel.bin ::/boot 
	@mcopy -i fat.img bootinfo.toml ::/boot
	@mcopy -i fat.img kernel-font.ttf ::/boot
	@cp fat.img iso
	@xorriso -as mkisofs -quiet -R -f -e fat.img -no-emul-boot -o os.iso iso > /dev/null

debug-kernel: fat
	cd src/kernel && cargo build
	cd src/bootloader && cargo build --target x86_64-unknown-uefi && cp target/x86_64-unknown-uefi/debug/$(NAME).efi target/x86_64-unknown-uefi/debug/bootx64.efi
	mcopy -i fat.img src/bootloader/target/x86_64-unknown-uefi/debug/bootx64.efi ::/efi/boot
	cp src/kernel/target/x86_64/debug/${NAME} kernel.bin

test: os-runner
	cd src/kernel && cargo test $(RUN_ARGS)

release-kernel: fat
	cd src/kernel && cargo build --release
	cd src/bootloader && cargo build --release --target x86_64-unknown-uefi && cp target/x86_64-unknown-uefi/release/$(NAME).efi target/x86_64-unknown-uefi/release/bootx64.efi
	mcopy -i fat.img src/bootloader/target/x86_64-unknown-uefi/release/bootx64.efi ::/efi/boot
	cp src/kernel/target/x86_64/release/${NAME} kernel.bin

debug: iso debug-kernel
	mcopy -i fat.img kernel.bin ::/boot 
	mcopy -i fat.img bootinfo.toml ::/boot
	mcopy -i fat.img kernel-font.ttf ::/boot
	cp fat.img iso
	xorriso -as mkisofs -R -f -e fat.img -no-emul-boot -o os.iso iso

release: iso release-kernel
	mcopy -i fat.img kernel.bin ::/boot 
	mcopy -i fat.img bootinfo.toml ::/boot
	mcopy -i fat.img kernel-font.ttf ::/boot
	cp fat.img iso
	xorriso -as mkisofs -R -f -e fat.img -no-emul-boot -o os.iso iso

fat:
	@dd if=/dev/zero of=fat.img bs=1M count=16 status=none
	@mkfs.vfat fat.img
	@mmd -i fat.img ::/efi
	@mmd -i fat.img ::/efi/boot
	@mmd -i fat.img ::/boot

clean:
	cd src/bootloader && cargo clean
	cd src/kernel && cargo clean
	cd src/os-runner && cargo clean
	rm -rf iso
	rm -rf filesize.inf
	rm -rf kernel.bin
	rm -rf fat.img
	rm -rf os.iso
	rm -rf os-runner
