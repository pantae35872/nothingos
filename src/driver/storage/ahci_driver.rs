use core::mem::size_of;
use core::ptr::write_bytes;

use crate::driver::pci::PCIControler;
use crate::memory::paging::ActivePageTable;
use crate::utils::port::Port32Bit;
use crate::{inline_if, print, println};

pub const AHCI_START: usize = 0xFFFFFFFF00000000;
pub const AHCI_SIZE: usize = size_of::<HbaMem>();

const AHCI_BASE: u32 = 0x400000;
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
const HBA_PXCMD_ST: u32 = 0x0001;
const HBA_PXCMD_FRE: u32 = 0x0010;
const HBA_PXCMD_FR: u32 = 0x4000;
const HBA_PXCMD_CR: u32 = 0x8000;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HbaMem {
    // 0x00 - 0x2B, Generic Host Control
    cap: u32,     // 0x00, Host capability
    ghc: u32,     // 0x04, Global host control
    is: u32,      // 0x08, Interrupt status
    pi: u32,      // 0x0C, Port implemented
    vs: u32,      // 0x10, Version
    ccc_ctl: u32, // 0x14, Command completion coalescing control
    ccc_pts: u32, // 0x18, Command completion coalescing ports
    em_loc: u32,  // 0x1C, Enclosure management location
    em_ctl: u32,  // 0x20, Enclosure management control
    cap2: u32,    // 0x24, Host capabilities extended
    bohc: u32,    // 0x28, BIOS/OS handoff control and status

    // 0x2C - 0x9F, Reserved
    rsv: [u8; 0xA0 - 0x2C],

    // 0xA0 - 0xFF, Vendor specific registers
    vendor: [u8; 0x100 - 0xA0],

    // 0x100 - 0x10FF, Port control registers
    ports: [HbaPort; 32], // 1 ~ 32
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HbaPort {
    clb: u32,         // 0x00, command list base address, 1K-byte aligned
    clbu: u32,        // 0x04, command list base address upper 32 bits
    fb: u32,          // 0x08, FIS base address, 256-byte aligned
    fbu: u32,         // 0x0C, FIS base address upper 32 bits
    is: u32,          // 0x10, interrupt status
    ie: u32,          // 0x14, interrupt enable
    cmd: u32,         // 0x18, command and status
    rsv0: u32,        // 0x1C, Reserved
    tfd: u32,         // 0x20, task file data
    sig: u32,         // 0x24, signature
    ssts: u32,        // 0x28, SATA status (SCR0:SStatus)
    sctl: u32,        // 0x2C, SATA control (SCR2:SControl)
    serr: u32,        // 0x30, SATA error (SCR1:SError)
    sact: u32,        // 0x34, SATA active (SCR3:SActive)
    ci: u32,          // 0x38, command issue
    sntf: u32,        // 0x3C, SATA notification (SCR4:SNotification)
    fbs: u32,         // 0x40, FIS-based switch control
    rsv1: [u32; 11],  // 0x44 ~ 0x6F, Reserved
    vendor: [u32; 4], // 0x70 ~ 0x7F, vendor specific
}

#[repr(C)]
struct HbaCmdHeader {
    // DW0
    cfl: u8, // Command FIS length in DWORDS, 2 ~ 16
    a: u8,   // ATAPI
    w: u8,   // Write, 1: H2D, 0: D2H
    p: u8,   // Prefetchable

    r: u8,    // Reset
    b: u8,    // BIST
    c: u8,    // Clear busy upon R_OK
    rsv0: u8, // Reserved
    pmp: u8,  // Port multiplier port

    prdtl: u16, // Physical region descriptor table length in entries

    // DW1
    prdbc: u32, // Physical region descriptor byte count transferred

    // DW2, 3
    ctba: u32,  // Command table descriptor base address
    ctbau: u32, // Command table descriptor base address upper 32 bits

    // DW4 - 7
    rsv1: [u32; 4], // Reserved
}

pub struct AHCIDrive {
    abar: HbaMem,
}

impl HbaPort {
    pub fn new() -> Self {
        Self {
            clb: 0,
            clbu: 0,
            fb: 0,
            fbu: 0,
            is: 0,
            ie: 0,
            cmd: 0,
            rsv0: 0,
            tfd: 0,
            sig: 0,
            ssts: 0,
            sctl: 0,
            serr: 0,
            sact: 0,
            ci: 0,
            sntf: 0,
            fbs: 0,
            rsv1: [0u32; 11],
            vendor: [0u32; 4],
        }
    }
}

impl HbaMem {
    pub fn new() -> Self {
        Self {
            cap: 0,
            ghc: 0,
            is: 0,
            pi: 0,
            vs: 0,
            ccc_ctl: 0,
            ccc_pts: 0,
            em_loc: 0,
            em_ctl: 0,
            cap2: 0,
            bohc: 0,
            rsv: [0u8; 0xA0 - 0x2C],
            vendor: [0u8; 0x100 - 0xA0],
            ports: [HbaPort::new(); 32],
        }
    }
}

impl AHCIDrive {
    pub async fn new() -> Self {
        let mut temp = Self {
            abar: HbaMem::new(),
        };

        temp.probe_port().await;

        temp
    }

    pub async fn probe_port(&mut self) {
        self.abar = unsafe { *(((AHCI_START) as *mut u8) as *mut HbaMem) };

        println!("{:#x}", self.abar.ports[0].sig);
        let mut pi = self.abar.pi;
        let mut i: u32 = 0;

        while i < 32 {
            if (pi & 1) != 0 {
                let dt = Self::check_type(&self.abar.ports[i as usize]).await;

                if dt == AHCI_DEV_SATA {
                    println!("SATA drive found at port {}", i);
                    self.port_rebase(i).await;
                    return;
                } else if dt == AHCI_DEV_SATAPI {
                    println!("SATAPI drive found at port {}", i);
                } else if dt == AHCI_DEV_SEMB {
                    println!("SEMB drive found at port {}", i);
                } else if dt == AHCI_DEV_PM {
                    println!("PM drive found at port {}", i)
                } else {
                    println!("Drive not found at port {}", i);
                }
            }
            pi >>= 1;
            i += 1;
        }
        //let pi = self.abar.pi;
    }

    async fn port_rebase(&mut self, port_number: u32) {
        let port: &mut HbaPort = &mut self.abar.ports[port_number as usize];
        Self::stop_cmd(port).await;

        port.clb = AHCI_BASE + (port_number << 10);
        port.clbu = 0;
        unsafe {
            write_bytes(port.clb as *mut u8, 0, 1024);
        }

        port.fb = AHCI_BASE + (32 << 10) + (port_number << 8);
        port.fbu = 0;
        unsafe {
            write_bytes(port.fb as *mut u8, 0, 256);
        }

        let cmdheader: &mut [HbaCmdHeader] =
            unsafe { &mut *(((port.clb) as *mut u8) as *mut [HbaCmdHeader; 32]) };

        for i in 0..32 {
            cmdheader[i].prdtl = 8;
            cmdheader[i].ctba = AHCI_BASE + (40 << 10) + (port_number << 13) + ((i as u32) << 8);
            cmdheader[i].ctbau = 0;
            unsafe {
                write_bytes(cmdheader[i].ctba as *mut u8, 0, 256);
            }
        }

        Self::start_cmd(port).await;
    }

    async fn start_cmd(port: &mut HbaPort) {
        while (port.cmd & HBA_PXCMD_CR) != 0 {}

        port.cmd |= HBA_PXCMD_FRE;
        port.cmd |= HBA_PXCMD_ST;
    }

    async fn stop_cmd(port: &mut HbaPort) {
        port.cmd &= !(HBA_PXCMD_ST);
        port.cmd &= !(HBA_PXCMD_FRE);

        loop {
            if (port.cmd & HBA_PXCMD_FR) != 0 {
                continue;
            }
            if (port.cmd & HBA_PXCMD_CR) != 0 {
                continue;
            }
            break;
        }
    }

    pub async fn check_type(port: &HbaPort) -> u32 {
        let ssts = port.ssts;

        let ipm = (ssts >> 8) & 0x0F;
        let det = ssts & 0x0F;

        if det != HBA_PORT_DET_PRESENT {
            return AHCI_DEV_NULL;
        }
        if ipm != HBA_PORT_IPM_ACTIVE {
            return AHCI_DEV_NULL;
        }

        match port.sig {
            SATA_SIG_ATAPI => AHCI_DEV_SATAPI,
            SATA_SIG_SEMB => AHCI_DEV_SEMB,
            SATA_SIG_PM => AHCI_DEV_PM,
            _ => AHCI_DEV_SATA,
        }
    }

    pub fn get_ahci_address() -> Option<u64> {
        let mut pci = PCIControler::new();
        let mut vendor: u32;
        let mut device: u32;
        for bus in 0..256 {
            for slot in 0..32 {
                vendor = pci.read(&bus, &slot, &0, &0x00);
                device = pci.read(&bus, &slot, &0, &0x02);
                if vendor == 0x29228086 && device == 0x2922 {
                    return Some(pci.read(&bus, &slot, &0, &0x24).into());
                }
            }
        }
        None
    }
}
