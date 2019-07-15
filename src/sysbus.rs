use std::io;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::{cartridge::Cartridge, ioregs::IoRegs};

use super::arm7tdmi::bus::{Bus, MemoryAccess, MemoryAccessWidth};
use super::arm7tdmi::Addr;

const VIDEO_RAM_SIZE: usize = 128 * 1024;
const WORK_RAM_SIZE: usize = 256 * 1024;
const INTERNAL_RAM: usize = 32 * 1024;
const PALETTE_RAM_SIZE: usize = 1 * 1024;
const OAM_SIZE: usize = 1 * 1024;

#[derive(Debug)]
pub struct BoxedMemory(Box<[u8]>, WaitState);

impl BoxedMemory {
    pub fn new(boxed_slice: Box<[u8]>) -> BoxedMemory {
        BoxedMemory(boxed_slice, Default::default())
    }

    pub fn new_with_waitstate(boxed_slice: Box<[u8]>, ws: WaitState) -> BoxedMemory {
        BoxedMemory(boxed_slice, ws)
    }
}

#[derive(Debug)]
pub struct WaitState {
    pub access8: usize,
    pub access16: usize,
    pub access32: usize,
}

impl WaitState {
    pub fn new(access8: usize, access16: usize, access32: usize) -> WaitState {
        WaitState {
            access8,
            access16,
            access32,
        }
    }
}

impl Default for WaitState {
    fn default() -> WaitState {
        WaitState::new(1, 1, 1)
    }
}

impl Bus for BoxedMemory {
    fn read_32(&self, addr: Addr) -> u32 {
        (&self.0[addr as usize..])
            .read_u32::<LittleEndian>()
            .unwrap()
    }

    fn read_16(&self, addr: Addr) -> u16 {
        (&self.0[addr as usize..])
            .read_u16::<LittleEndian>()
            .unwrap()
    }

    fn read_8(&self, addr: Addr) -> u8 {
        (&self.0[addr as usize..])[0]
    }

    fn write_32(&mut self, addr: Addr, value: u32) {
        (&mut self.0[addr as usize..])
            .write_u32::<LittleEndian>(value)
            .unwrap()
    }

    fn write_16(&mut self, addr: Addr, value: u16) {
        (&mut self.0[addr as usize..])
            .write_u16::<LittleEndian>(value)
            .unwrap()
    }

    fn write_8(&mut self, addr: Addr, value: u8) {
        (&mut self.0[addr as usize..]).write_u8(value).unwrap()
    }

    fn get_bytes(&self, addr: Addr) -> &[u8] {
        &self.0[addr as usize..]
    }

    fn get_bytes_mut(&mut self, addr: Addr) -> &mut [u8] {
        &mut self.0[addr as usize..]
    }

    fn get_cycles(&self, _addr: Addr, access: MemoryAccess) -> usize {
        match access.1 {
            MemoryAccessWidth::MemoryAccess8 => self.1.access8,
            MemoryAccessWidth::MemoryAccess16 => self.1.access16,
            MemoryAccessWidth::MemoryAccess32 => self.1.access32,
        }
    }
}

#[derive(Debug)]
struct DummyBus([u8; 4]);

impl Bus for DummyBus {
    fn read_32(&self, _addr: Addr) -> u32 {
        0
    }

    fn read_16(&self, _addr: Addr) -> u16 {
        0
    }

    fn read_8(&self, _addr: Addr) -> u8 {
        0
    }

    fn write_32(&mut self, _addr: Addr, _value: u32) {}

    fn write_16(&mut self, _addr: Addr, _value: u16) {}

    fn write_8(&mut self, _addr: Addr, _value: u8) {}

    fn get_bytes(&self, _addr: Addr) -> &[u8] {
        &self.0
    }

    fn get_bytes_mut(&mut self, _addr: Addr) -> &mut [u8] {
        &mut self.0
    }

    fn get_cycles(&self, _addr: Addr, _access: MemoryAccess) -> usize {
        1
    }
}

#[derive(Debug)]
pub struct SysBus {
    bios: BoxedMemory,
    onboard_work_ram: BoxedMemory,
    internal_work_ram: BoxedMemory,
    /// Currently model the IOMem as regular buffer, later make it into something more sophisticated.
    pub ioregs: IoRegs,
    palette_ram: BoxedMemory,
    vram: BoxedMemory,
    oam: BoxedMemory,
    gamepak: Cartridge,
    dummy: DummyBus,
}

impl SysBus {
    pub fn new(bios_rom: Vec<u8>, gamepak: Cartridge) -> SysBus {
        SysBus {
            bios: BoxedMemory::new(bios_rom.into_boxed_slice()),
            onboard_work_ram: BoxedMemory::new_with_waitstate(
                vec![0; WORK_RAM_SIZE].into_boxed_slice(),
                WaitState::new(3, 3, 6),
            ),
            internal_work_ram: BoxedMemory::new(vec![0; INTERNAL_RAM].into_boxed_slice()),
            ioregs: IoRegs::default(),
            palette_ram: BoxedMemory::new_with_waitstate(
                vec![0; PALETTE_RAM_SIZE].into_boxed_slice(),
                WaitState::new(1, 1, 2),
            ),
            vram: BoxedMemory::new_with_waitstate(
                vec![0; VIDEO_RAM_SIZE].into_boxed_slice(),
                WaitState::new(1, 1, 2),
            ),
            oam: BoxedMemory::new(vec![0; OAM_SIZE].into_boxed_slice()),
            gamepak: gamepak,
            dummy: DummyBus([0; 4]),
        }
    }
}

macro_rules! call_bus_method {
    ($sysbus:expr, $addr:expr, $func:ident, $($args:expr),*) => {
        match $addr as usize {
            0x0000_0000...0x0000_3fff => $sysbus.bios.$func($($args,)*),
            0x0200_0000...0x0203_ffff => $sysbus.onboard_work_ram.$func($($args,)*),
            0x0300_0000...0x0300_7fff => $sysbus.internal_work_ram.$func($($args,)*),
            0x0400_0000...0x0400_03fe => $sysbus.ioregs.$func($($args,)*),
            0x0500_0000...0x0500_03ff => $sysbus.palette_ram.$func($($args,)*),
            0x0600_0000...0x0601_7fff => $sysbus.vram.$func($($args,)*),
            0x0700_0000...0x0700_03ff => $sysbus.oam.$func($($args,)*),
            0x0800_0000...0x09ff_ffff => $sysbus.gamepak.$func($($args,)*),
            _ => $sysbus.dummy.$func($($args,)*),
        }
    };
}

impl Bus for SysBus {
    fn read_32(&self, addr: Addr) -> u32 {
        call_bus_method!(self, addr, read_32, addr & 0xff_ffff)
    }

    fn read_16(&self, addr: Addr) -> u16 {
        call_bus_method!(self, addr, read_16, addr & 0xff_ffff)
    }

    fn read_8(&self, addr: Addr) -> u8 {
        call_bus_method!(self, addr, read_8, addr & 0xff_ffff)
    }

    fn write_32(&mut self, addr: Addr, value: u32) {
        call_bus_method!(self, addr, write_32, addr & 0xff_ffff, value)
    }

    fn write_16(&mut self, addr: Addr, value: u16) {
        call_bus_method!(self, addr, write_16, addr & 0xff_ffff, value)
    }

    fn write_8(&mut self, addr: Addr, value: u8) {
        call_bus_method!(self, addr, write_8, addr & 0xff_ffff, value)
    }

    fn get_bytes(&self, addr: Addr) -> &[u8] {
        call_bus_method!(self, addr, get_bytes, addr & 0xff_ffff)
    }

    fn get_bytes_mut(&mut self, addr: Addr) -> &mut [u8] {
        call_bus_method!(self, addr, get_bytes_mut, addr & 0xff_ffff)
    }

    fn get_cycles(&self, addr: Addr, access: MemoryAccess) -> usize {
        call_bus_method!(self, addr, get_cycles, addr & 0xff_ffff, access)
    }
}
