use alloc::format;
use common::{boot::BootInformation, toml::parser::TomlValue};
use uefi::table::{boot::MemoryType, Boot, SystemTable};

use crate::{
    boot_services::{read_config, read_file},
    elf_loader::load_elf,
};

pub fn load_kernel(
    system_table: &mut SystemTable<Boot>,
) -> (u64, &'static mut BootInformation, bool) {
    let config: TomlValue = read_config(system_table, "\\boot\\bootinfo.toml");

    let kernel_file: &str = config
        .get("kernel_file")
        .expect("No kernel file found in info file")
        .as_string()
        .expect("Kernel file is not a string value in file info");
    let kernel_font_file: &str = config
        .get("font_file")
        .expect("No font file found in info file")
        .as_string()
        .expect("Font file is not a string value in file info");

    let kernel_font_buffer = read_file(system_table, &format!("\\boot\\{}", kernel_font_file));
    let kernel_buffer = read_file(system_table, &format!("\\boot\\{}", kernel_file));

    let boot_info = unsafe {
        BootInformation::from_ptr_mut(
            system_table
                .boot_services()
                .allocate_pool(MemoryType::LOADER_CODE, size_of::<BootInformation>())
                .unwrap_or_else(|e| {
                    panic!("Failed to allocate memory for the boot information {}", e)
                }) as *mut BootInformation,
        )
    };

    let (entrypoint, kern_start, kern_end, elf) = load_elf(system_table, kernel_buffer);

    boot_info.init_kernel(
        kernel_font_buffer.as_ptr() as u64,
        kernel_font_buffer.len(),
        kern_start,
        (kern_end - kern_start) as usize,
        elf,
    );
    return (
        entrypoint,
        boot_info,
        config
            .get("any_key_boot")
            .expect("any_key_boot boot config not found")
            .as_bool()
            .expect("any_key_boot is not a boolean"),
    );
}
