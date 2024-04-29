use core::alloc::Layout;
use core::mem::{align_of, size_of};
use core::ptr::{self, write_bytes};
use core::{u32, usize};

use alloc::alloc::alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use spin::Mutex;
use x86_64::{PhysAddr, VirtAddr};

use crate::driver::pci;
use crate::memory::paging::Page;
use crate::memory::Frame;
use crate::utils::oserror::OSError;
use crate::utils::VolatileCell;
use crate::{get_physical, println, EntryFlags, MemoryController};

use super::Drive;

pub static DRIVER: OnceCell<Mutex<AhciController>> = OnceCell::uninit();

pub const ABAR_START: usize = 0xFFFFFFFF00000000;
pub const ABAR_SIZE: usize = size_of::<HbaMem>();

const HBA_PORT_DET_PRESENT: u32 = 3;
const AHCI_DEV_NULL: u32 = 0;
const AHCI_DEV_SATA: u32 = 1;
const AHCI_DEV_SATAPI: u32 = 4;
const AHCI_DEV_SEMB: u32 = 2;
const AHCI_DEV_PM: u32 = 3;
const HBA_PORT_IPM_ACTIVE: u32 = 1;
const SATA_SIG_ATA: u32 = 0x00000101;
const SATA_SIG_ATAPI: u32 = 0xEB140101;
const SATA_SIG_SEMB: u32 = 0xC33C0101;
const SATA_SIG_PM: u32 = 0x96690101;
const HBA_PXCMD_ST: u32 = 1;
const HBA_PXCMD_FRE: u32 = 1 << 4;
const HBA_PXCMD_FR: u32 = 0x4000;
const HBA_PXCMD_CR: u32 = 1 << 15;
const FIS_TYPE_REG_H2D: u8 = 0x27;
const HBA_PXIS_TFES: u32 = 1 << 30;

#[repr(C)]
pub struct HbaCmdHeader {
    // DWORD 0
    bit_0_7: VolatileCell<u8>,
    bit_8_15: VolatileCell<u8>,
    prdtl: VolatileCell<u16>,
    // DWORD 1
    prdbc: VolatileCell<u32>,
    ctba: VolatileCell<PhysAddr>,
    rsv1: [VolatileCell<u32>; 4],
}

impl HbaCmdHeader {
    pub fn set_cfl(&mut self, value: &u8) {
        if *value > 0b00001111 {
            return;
        }

        self.bit_0_7.set(self.bit_0_7.get() & !(0b00001111));
        self.bit_0_7.set(self.bit_0_7.get() | (*value & 0b00001111));
    }

    pub fn set_a(&mut self, value: &bool) {
        if *value {
            self.bit_0_7.set(self.bit_0_7.get() | (1 << 5));
        } else {
            self.bit_0_7.set(self.bit_0_7.get() & !(1 << 5));
        }
    }

    pub fn set_w(&mut self, value: &bool) {
        if *value {
            self.bit_0_7.set(self.bit_0_7.get() | (1 << 6));
        } else {
            self.bit_0_7.set(self.bit_0_7.get() & !(1 << 6));
        }
    }

    pub fn set_p(&mut self, value: &bool) {
        if *value {
            self.bit_0_7.set(self.bit_0_7.get() | (1 << 7));
        } else {
            self.bit_0_7.set(self.bit_0_7.get() & !(1 << 7));
        }
    }

    pub fn set_r(&mut self, value: &bool) {
        if *value {
            self.bit_8_15.set(self.bit_8_15.get() | (1 << 0));
        } else {
            self.bit_8_15.set(self.bit_8_15.get() & !(1 << 0));
        }
    }
    pub fn set_b(&mut self, value: &bool) {
        if *value {
            self.bit_8_15.set(self.bit_8_15.get() | (1 << 1));
        } else {
            self.bit_8_15.set(self.bit_8_15.get() & !(1 << 1));
        }
    }
    pub fn set_c(&mut self, value: &bool) {
        if *value {
            self.bit_8_15.set(self.bit_8_15.get() | (1 << 2));
        } else {
            self.bit_8_15.set(self.bit_8_15.get() & !(1 << 2));
        }
    }

    pub fn set_pmp(&mut self, value: &u8) {
        if *value > 0b00001111 {
            return;
        }

        self.bit_8_15.set(self.bit_8_15.get() & !(0b11110000));
        self.bit_8_15
            .set(self.bit_8_15.get() | !(*value & 0b00001111));
    }
}

#[repr(C)]
pub struct FisRegH2D {
    // DWORD 0
    fis_type: VolatileCell<u8>, // FIS_TYPE_REG_H2D
    byte_1: VolatileCell<u8>,   // Port multiplier
    command: VolatileCell<u8>,  // Command register
    featurel: VolatileCell<u8>, // Feature register, 7:0

    // DWORD 1
    lba0: VolatileCell<u8>,   // LBA low register, 7:0
    lba1: VolatileCell<u8>,   // LBA mid register, 15:8
    lba2: VolatileCell<u8>,   // LBA high register, 23:16
    device: VolatileCell<u8>, // Device register

    // DWORD 2
    lba3: VolatileCell<u8>,     // LBA register, 31:24
    lba4: VolatileCell<u8>,     // LBA register, 39:32
    lba5: VolatileCell<u8>,     // LBA register, 47:40
    featureh: VolatileCell<u8>, // Feature register, 15:8

    // DWORD 3
    countl: VolatileCell<u8>,  // Count register, 7:0
    counth: VolatileCell<u8>,  // Count register, 15:8
    icc: VolatileCell<u8>,     // Isochronous command completion
    control: VolatileCell<u8>, // Control register

    // DWORD 4
    rsv1: [VolatileCell<u8>; 4], // Reserved
}

impl FisRegH2D {
    pub fn set_pmport(&mut self, value: &u8) {
        if *value > 0b00001111 {
            return;
        }

        self.byte_1.set(self.byte_1.get() & !(0b00001111));
        self.byte_1.set(self.byte_1.get() | (*value & 0b00001111));
    }

    pub fn set_c(&mut self, value: &bool) {
        if *value {
            self.byte_1.set(self.byte_1.get() | (1 << 7));
        } else {
            self.byte_1.set(self.byte_1.get() & !(1 << 7));
        }
    }
}

#[repr(C)]
pub struct HbaMem {
    // 0x00 - 0x2B, Generic Host Control
    cap: VolatileCell<u32>,     // 0x00, Host capability
    ghc: VolatileCell<u32>,     // 0x04, Global host control
    is: VolatileCell<u32>,      // 0x08, Interrupt status
    pi: VolatileCell<u32>,      // 0x0C, Port implemented
    vs: VolatileCell<u32>,      // 0x10, Version
    ccc_ctl: VolatileCell<u32>, // 0x14, Command completion coalescing control
    ccc_pts: VolatileCell<u32>, // 0x18, Command completion coalescing ports
    em_loc: VolatileCell<u32>,  // 0x1C, Enclosure management location
    em_ctl: VolatileCell<u32>,  // 0x20, Enclosure management control
    cap2: VolatileCell<u32>,    // 0x24, Host capabilities extended
    bohc: VolatileCell<u32>,    // 0x28, BIOS/OS handoff control and status

    // 0x2C - 0x9F, Reserved
    rsv: [VolatileCell<u8>; 0xA0 - 0x2C],

    // 0xA0 - 0xFF, Vendor specific registers
    vendor: [VolatileCell<u8>; 0x100 - 0xA0],
}

#[repr(C)]
pub struct HbaPRDTEntry {
    dba: VolatileCell<PhysAddr>,
    rsv0: VolatileCell<u32>, // Reserved

    // DW3
    dw3: VolatileCell<u32>, // Byte count, 4M max
                            //rsv1: u32, // Reserved
                            //i: u32,    // Interrupt on completion
}

impl HbaPRDTEntry {
    pub fn set_i(&mut self, value: &bool) {
        if *value {
            self.dw3.set(self.dw3.get() | (1 << 31));
        } else {
            self.dw3.set(self.dw3.get() & !(1 << 31));
        }
    }

    pub fn set_dbc(&mut self, value: &u32) {
        if *value > 0b00000000001111111111111111111111 {
            return;
        }

        self.dw3
            .set(self.dw3.get() & !(0b00000000001111111111111111111111));
        self.dw3
            .set(self.dw3.get() | (*value & 0b00000000001111111111111111111111));
    }
}

#[repr(C)]
pub struct HbaCmdTbl {
    // 0x00
    cfis: [VolatileCell<u8>; 64], // Command FIS

    // 0x40
    acmd: [VolatileCell<u8>; 16], // ATAPI command, 12 or 16 bytes

    // 0x50
    rsv: [VolatileCell<u8>; 48], // Reserved

    // 0x80
    prdt_entry: [HbaPRDTEntry; 1], // Physical region descriptor table entries, 0 ~ 65535
}

impl HbaCmdTbl {
    pub fn get_prdt_entry(&mut self, index: usize) -> &mut HbaPRDTEntry {
        unsafe { &mut *self.prdt_entry.as_mut_ptr().add(index) }
    }
}

#[repr(C)]
pub struct HbaPort {
    clb: VolatileCell<PhysAddr>,
    fb: VolatileCell<PhysAddr>,
    is: VolatileCell<u32>,          // 0x10, interrupt status
    ie: VolatileCell<u32>,          // 0x14, interrupt enable
    cmd: VolatileCell<u32>,         // 0x18, command and status
    rsv0: VolatileCell<u32>,        // 0x1C, Reserved
    tfd: VolatileCell<u32>,         // 0x20, task file data
    sig: VolatileCell<u32>,         // 0x24, signature
    ssts: VolatileCell<u32>,        // 0x28, SATA status (SCR0:SStatus)
    sctl: VolatileCell<u32>,        // 0x2C, SATA control (SCR2:SControl)
    serr: VolatileCell<u32>,        // 0x30, SATA error (SCR1:SError)
    sact: VolatileCell<u32>,        // 0x34, SATA active (SCR3:SActive)
    ci: VolatileCell<u32>,          // 0x38, command issue
    sntf: VolatileCell<u32>,        // 0x3C, SATA notification (SCR4:SNotification)
    fbs: VolatileCell<u32>,         // 0x40, FIS-based switch control
    rsv1: [VolatileCell<u32>; 11],  // 0x44 ~ 0x6F, Reserved
    vendor: [VolatileCell<u32>; 4], // 0x70 ~ 0x7F, vendor specific
}

pub struct AhciPort {
    hba_address: VirtAddr,
    clb: VirtAddr,
    fb: VirtAddr,
    ctba: [VirtAddr; 32],
    cap: usize,
}

impl AhciPort {
    fn hba_port(&self) -> &mut HbaPort {
        unsafe { &mut *(self.hba_address.as_mut_ptr::<HbaPort>()) }
    }

    fn cmd_header(&self) -> &mut HbaCmdHeader {
        unsafe { &mut *(self.clb.as_mut_ptr::<HbaCmdHeader>()) }
    }

    fn cmd_tbl(&self) -> &mut HbaCmdTbl {
        let slot = self.find_cmdslot().expect("Cannot get Cmd Tbl");
        unsafe { &mut *(self.ctba[slot].as_mut_ptr::<HbaCmdTbl>()) }
    }

    fn start_cmd(&mut self) {
        let port = self.hba_port();
        while (port.cmd.get() & HBA_PXCMD_CR) != 0 {}

        port.cmd.set(port.cmd.get() | HBA_PXCMD_FRE);
        port.cmd.set(port.cmd.get() | HBA_PXCMD_ST);
    }

    fn stop_cmd(&mut self) {
        let port = self.hba_port();
        port.cmd.set(port.cmd.get() & !(HBA_PXCMD_ST));
        port.cmd.set(port.cmd.get() & !(HBA_PXCMD_FRE));

        loop {
            if (port.cmd.get() & HBA_PXCMD_FR) != 0 {
                continue;
            }
            if (port.cmd.get() & HBA_PXCMD_CR) != 0 {
                continue;
            }
            break;
        }
    }

    fn check_type(&mut self) -> u32 {
        let port = self.hba_port();
        let ssts = port.ssts.get();

        let ipm = (ssts >> 8) & 0x0F;
        let det = ssts & 0x0F;

        if det != HBA_PORT_DET_PRESENT {
            return AHCI_DEV_NULL;
        }
        if ipm != HBA_PORT_IPM_ACTIVE {
            return AHCI_DEV_NULL;
        }

        match port.sig.get() {
            SATA_SIG_ATAPI => AHCI_DEV_SATAPI,
            SATA_SIG_SEMB => AHCI_DEV_SEMB,
            SATA_SIG_PM => AHCI_DEV_PM,
            _ => AHCI_DEV_SATA,
        }
    }

    fn rebase(&mut self, memory_controller: &mut MemoryController) {
        self.stop_cmd();
        let port = self.hba_port();
        let virt_clb: VirtAddr = unsafe {
            VirtAddr::new(alloc(Layout::from_size_align(4096, align_of::<u8>()).unwrap()) as u64)
        };

        unsafe {
            write_bytes(virt_clb.as_mut_ptr::<u8>(), 0, 1024);
        }
        port.clb
            .set(memory_controller.get_physical(virt_clb).unwrap());
        let virt_fb: VirtAddr = unsafe {
            VirtAddr::new(alloc(Layout::from_size_align(4096, align_of::<u8>()).unwrap()) as u64)
        };
        unsafe {
            write_bytes(virt_fb.as_mut_ptr::<u8>(), 0, 1024);
        }
        port.fb
            .set(memory_controller.get_physical(virt_clb).unwrap());
        self.fb = virt_fb;
        self.clb = virt_clb;
        let port = self.hba_port();

        port.serr.set(1);
        port.is.set(0);
        port.ie.set(0);
        let cmdheader = unsafe { &mut *(virt_clb.as_mut_ptr() as *mut [HbaCmdHeader; 32]) };
        for i in 0..32 {
            cmdheader[i].prdtl.set(8);
            let virt_ctba: VirtAddr = unsafe {
                VirtAddr::new(
                    alloc(Layout::from_size_align(4096, align_of::<u8>()).unwrap()) as u64,
                )
            };
            unsafe {
                ptr::write_bytes(virt_ctba.as_mut_ptr::<u8>(), 0, 4096);
            }
            self.ctba[i] = virt_ctba;
            cmdheader[i]
                .ctba
                .set(memory_controller.get_physical(virt_ctba).unwrap());
        }
        self.start_cmd();
        let port = self.hba_port();
        port.is.set(0);
        port.ie.set(0xffffffff);
    }

    fn find_cmdslot(&self) -> Result<usize, Box<OSError>> {
        let port = self.hba_port();
        let mut slots = port.sact.get() | port.ci.get();
        let num_of_slots = (self.cap & 0x0F00) >> 8;
        for i in 0..num_of_slots {
            if (slots & 1) == 0 {
                return Ok(i);
            }

            slots >>= 1;
        }
        Err(Box::new(OSError::new(
            "Cannot find free command list entry",
        )))
    }
}

pub struct AhciDrive {
    port: Mutex<AhciPort>,
    identifier: Option<[u8; 512]>,
}

pub struct AhciController {
    drives: Vec<AhciDrive>,
}

impl AhciDrive {
    pub fn new(port_addr: VirtAddr, cap: usize) -> Self {
        let port = Mutex::new(AhciPort {
            hba_address: port_addr,
            clb: VirtAddr::new(0),
            fb: VirtAddr::new(0),
            ctba: [VirtAddr::new(0); 32],
            cap,
        });
        Self {
            port,
            identifier: None,
        }
    }

    async fn run_command(
        &mut self,
        lba: u64,
        count: usize,
        buffer: &[u8],
        command: u8,
    ) -> Result<(), alloc::boxed::Box<crate::utils::oserror::OSError>> {
        let mut count = count;
        let port = self.port.lock();
        port.hba_port().is.set(u32::MAX);
        let cmd_header = port.cmd_header();
        cmd_header.set_cfl(&((size_of::<FisRegH2D>() / size_of::<u32>()) as u8));
        cmd_header.set_w(&false);
        cmd_header.prdtl.set((((count - 1) >> 4) + 1) as u16);

        let cmdtbl = port.cmd_tbl();
        let mut buf_address = get_physical(VirtAddr::new(buffer.as_ptr() as u64)).unwrap();
        let mut i: usize = 0;
        while i < cmd_header.prdtl.get() as usize - 1 {
            cmdtbl.get_prdt_entry(i).dba.set(buf_address);
            cmdtbl.get_prdt_entry(i).set_dbc(&(8 * 1024 - 1));
            cmdtbl.get_prdt_entry(i).set_i(&true);
            buf_address += (4 * 4024) as u64;
            count -= 16;
            i += 1;
        }

        cmdtbl.get_prdt_entry(i).dba.set(buf_address);
        cmdtbl
            .get_prdt_entry(i)
            .set_dbc(&(((count << 9) - 1) as u32));
        cmdtbl.get_prdt_entry(i).set_i(&true);

        let cmdfis = unsafe { &mut *(cmdtbl.cfis.as_mut_ptr() as *mut FisRegH2D) };
        cmdfis.fis_type.set(FIS_TYPE_REG_H2D);
        cmdfis.set_c(&true);
        cmdfis.command.set(command);

        let lower = lba & 0xFFFFFFFF;
        let upper = (lba & 0xFFFFFFFF00000000) >> 32;
        cmdfis.lba0.set(lower as u8);
        cmdfis.lba1.set((lower >> 8) as u8);
        cmdfis.lba2.set((lower >> 16) as u8);
        cmdfis.device.set(1 << 6);
        cmdfis.lba3.set((lower >> 24) as u8);
        cmdfis.lba4.set(upper as u8);
        cmdfis.lba5.set((upper >> 8) as u8);

        cmdfis.countl.set((count & 0xFF) as u8);
        cmdfis.counth.set(((count >> 8) & 0xFF) as u8);

        loop {
            if port.hba_port().tfd.get() & (0x80 | 0x08) != 0 {
                continue;
            }
            break;
        }

        port.hba_port().ci.set(1);

        loop {
            if (port.hba_port().ci.get() & 1) == 0 {
                break;
            };
            if (port.hba_port().is.get() & HBA_PXIS_TFES) != 0 {
                return Err(Box::new(OSError::new("Task file error")));
            }
        }

        if (port.hba_port().is.get() & HBA_PXIS_TFES) != 0 {
            return Err(Box::new(OSError::new("Task file error")));
        }

        return Ok(());
    }

    pub async fn identify(
        &mut self,
    ) -> Result<(), alloc::boxed::Box<crate::utils::oserror::OSError>> {
        let buf = [0u8; 512];
        self.run_command(0, 1, &buf, 0xEC).await?;
        self.identifier = Some(buf);
        Ok(())
    }
}

impl Drive for AhciDrive {
    fn lba_end(&self) -> u64 {
        if let Some(identifier) = self.identifier {
            return u64::from_le_bytes([
                identifier[100],
                identifier[101],
                identifier[102],
                identifier[103],
                0,
                0,
                0,
                0,
            ]) - 1;
        } else {
            panic!("Please identify the drive before accessing it");
        }
    }

    async fn read(
        &mut self,
        from_sector: u64,
        buffer: &mut [u8],
        count: usize,
    ) -> Result<(), alloc::boxed::Box<crate::utils::oserror::OSError>> {
        return self.run_command(from_sector, count, buffer, 0x25).await;
    }

    async fn write(
        &mut self,
        from_sector: u64,
        buffer: &[u8],
        count: usize,
    ) -> Result<(), alloc::boxed::Box<crate::utils::oserror::OSError>> {
        return self.run_command(from_sector, count, buffer, 0x35).await;
    }
}

impl AhciController {
    pub fn new() -> Self {
        Self { drives: Vec::new() }
    }

    pub fn probe_port(&mut self, memory_controller: &mut MemoryController) {
        let abar = unsafe { &mut *((ABAR_START as *mut u8) as *mut HbaMem) };
        let mut pi = abar.pi.get();
        if abar.bohc.get() & 2 == 0 {
            abar.bohc.set(abar.bohc.get() | 0b10);
            let mut spin = 0;
            while abar.bohc.get() & 1 != 0 && spin < 50000 {
                spin += 1;
            }
            if abar.bohc.get() & 1 != 0 {
                abar.bohc.set(2);
                abar.bohc.set(abar.bohc.get() | 8);
            }
        }

        for i in 0..32 {
            let port_address =
                VirtAddr::new((ABAR_START as u64) + 0x100 + (i * size_of::<HbaPort>()) as u64);
            let drive = AhciDrive::new(port_address, abar.cap.get() as usize);
            if (pi & 1) != 0 {
                let dt = drive.port.lock().check_type();

                if dt == AHCI_DEV_SATA {
                    drive.port.lock().rebase(memory_controller);
                    self.drives.push(drive);
                }
            }
            pi >>= 1;
        }
    }

    pub async fn get_drive(&mut self, index: &usize) -> Result<&mut AhciDrive, Box<OSError>> {
        if let Some(drive) = self.drives.get_mut(*index) {
            Ok(drive)
        } else {
            Err(Box::new(OSError::new("Drive not found")))
        }
    }

    pub fn get_ahci_address() -> Option<u64> {
        let mut pci = pci::DRIVER
            .get()
            .expect("PCI driver is not initialize")
            .lock();
        let mut vendor: u32;
        let mut device: u32;
        for bus in 0..256 {
            for slot in 0..32 {
                vendor = pci.read(&bus, &slot, &0, &0x00);
                device = pci.read(&bus, &slot, &0, &0x02);
                if (vendor == 0x29228086 && device == 0x2922)
                    || (vendor == 0x28298086 && device == 0x2829)
                    || (vendor == 0xa1038086 && device == 0xa103)
                {
                    return Some(pci.read(&bus, &slot, &0, &0x24).into());
                }
            }
        }
        None
    }
}

pub fn init(memory_controller: &mut MemoryController) {
    let abar_start_page = Page::containing_address(ABAR_START);
    let abar_end_page = Page::containing_address(ABAR_START + ABAR_SIZE);
    let abar_address = AhciController::get_ahci_address();
    if let Some(abar_address) = abar_address {
        for (page, frame) in
            Page::range_inclusive(abar_start_page, abar_end_page).zip(Frame::range_inclusive(
                Frame::containing_address(abar_address as usize),
                Frame::containing_address(abar_address as usize + ABAR_SIZE),
            ))
        {
            memory_controller.active_table.map_to(
                page,
                frame,
                EntryFlags::PRESENT
                    | EntryFlags::NO_CACHE
                    | EntryFlags::WRITABLE
                    | EntryFlags::WRITE_THROUGH,
                &mut memory_controller.frame_allocator,
            );
        }
        DRIVER.init_once(|| {
            let mut controller = AhciController::new();
            controller.probe_port(memory_controller);
            Mutex::from(controller)
        });
    }
}
