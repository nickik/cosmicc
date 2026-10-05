//! Freestanding, zero-argument C entry startup for byte-addressed RAM at zero.
use super::{fail, LinkedImage};
use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupState {
    pub entry: u32,
    pub registers: [u32; 16],
    pub return_address: u32,
}

impl LinkedImage {
    /// Validate the complete image/startup layout before changing RAM. Set r13
    /// to an aligned downward-growing stack and r14 to the caller's exit trap.
    /// This is a freestanding call ABI, not hosted argc/argv or an OS loader.
    pub fn load_into(
        &self,
        ram: &mut [u8],
        stack_bottom: u32,
        stack_top: u32,
        return_address: u32,
    ) -> Result<StartupState, Error> {
        self.validate_image(u32::MAX)?;
        let end = u64::from(self.base) + self.bytes.len() as u64;
        if end > ram.len() as u64
            || stack_bottom >= stack_top
            || stack_bottom % 16 != 0
            || stack_top % 16 != 0
            || u64::from(stack_top) > ram.len() as u64
            || (u64::from(stack_bottom) < end && stack_top > self.base)
            || return_address % 4 != 0
            || u64::from(return_address) >= ram.len() as u64
            || (return_address >= self.base && u64::from(return_address) < end)
            || (return_address >= stack_bottom && return_address < stack_top)
        {
            return Err(fail("invalid RAM, image, stack or return-trap layout"));
        }
        let start = self.base as usize;
        ram[start..end as usize].copy_from_slice(&self.bytes);
        for region in self.regions.iter().filter(|r| r.zero_fill) {
            ram[region.address as usize
                ..(u64::from(region.address) + u64::from(region.size)) as usize]
                .fill(0);
        }
        let mut registers = [0; 16];
        registers[13] = stack_top;
        registers[14] = return_address;
        Ok(StartupState {
            entry: self.entry,
            registers,
            return_address,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_default;

    #[test]
    fn loads_nonzero_entry_and_clears_bss_in_dirty_ram() {
        let image = compile_default("int helper(void){return 3;} int zero[32]; int initialized=17; int main(void){return zero[4]+initialized+helper();}")
            .unwrap().link_image(0x1000, "main", 0x1000).unwrap();
        assert_ne!(image.entry, image.base);
        let mut ram = vec![0xa5; 0x10000];
        let state = image.load_into(&mut ram, 0x8000, 0xf000, 0x7000).unwrap();
        assert_eq!(state.entry, image.symbols["main"]);
        assert_eq!(state.registers[13], 0xf000);
        assert_eq!(state.registers[14], 0x7000);
        let address = image.symbols["zero"] as usize;
        assert_eq!(&ram[address..address + 128], &[0; 128]);
        assert!(image
            .regions
            .iter()
            .any(|r| r.name == "zero" && r.zero_fill));
        assert_eq!(ram[image.symbols["initialized"] as usize], 17);
        assert_eq!(ram[0], 0xa5);
        assert_eq!(ram[0x8000], 0xa5);
    }

    #[test]
    fn rejects_bad_layout_without_partial_writes() {
        let image = compile_default("int main(void){return 0;}")
            .unwrap()
            .link_image(0x1000, "main", 0x1000)
            .unwrap();
        for (bottom, top, exit) in [
            (0x1000, 0x8000, 0x7000),
            (0x8001, 0xf000, 0x7000),
            (0x8000, 0x10010, 0x7000),
            (0x8000, 0xf000, 0x1000),
            (0x8000, 0xf000, 0x8000),
        ] {
            let mut ram = vec![0xa5; 0x10000];
            assert!(image.load_into(&mut ram, bottom, top, exit).is_err());
            assert!(ram.iter().all(|b| *b == 0xa5));
        }
    }
}
