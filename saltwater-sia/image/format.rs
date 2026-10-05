//! Versioned local linked-image container. Not Cosmic's protected-loader ABI.
use super::{fail, ImageRegion, LinkedImage};
use crate::Error;
use std::collections::BTreeMap;
use std::convert::TryFrom;
const MAGIC: &[u8; 8] = b"CSIAIMG\0";
const VERSION: u32 = 2;
fn word(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn name(out: &mut Vec<u8>, value: &str) -> Result<(), Error> {
    word(
        out,
        u32::try_from(value.len()).map_err(|_| fail("name too long"))?,
    );
    out.extend_from_slice(value.as_bytes());
    Ok(())
}
struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, size: usize) -> Result<&'a [u8], Error> {
        let end = self
            .cursor
            .checked_add(size)
            .ok_or_else(|| fail("container length overflow"))?;
        let result = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| fail("truncated container"))?;
        self.cursor = end;
        Ok(result)
    }
    fn word(&mut self) -> Result<u32, Error> {
        let mut value = [0; 4];
        value.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(value))
    }
    fn name(&mut self) -> Result<String, Error> {
        let size = self.word()? as usize;
        let value = std::str::from_utf8(self.take(size)?).map_err(|_| fail("non-UTF8 name"))?;
        if value.is_empty() || value.contains('\0') {
            return Err(fail("invalid empty/NUL name"));
        }
        Ok(value.to_owned())
    }
}
impl LinkedImage {
    /// Validate before transport; mapping permissions still require an OS loader.
    pub fn validate_image(&self, max_payload: u32) -> Result<(), Error> {
        let size = u32::try_from(self.bytes.len()).map_err(|_| fail("payload exceeds SIA32"))?;
        if size == 0 || size > max_payload || self.base % 4 != 0 {
            return Err(fail("invalid payload budget/base"));
        }
        let end = u64::from(self.base) + u64::from(size);
        if end > u64::from(u32::MAX) + 1 {
            return Err(fail("payload address overflow"));
        }
        let mut spans = Vec::new();
        let mut names = std::collections::BTreeSet::new();
        let mut entry_valid = false;
        for r in &self.regions {
            if r.name.is_empty() || r.name.contains('\0') || !names.insert(&r.name) {
                return Err(fail("invalid/duplicate region name"));
            }
            let r_end = u64::from(r.address) + u64::from(r.size);
            if r.address < self.base || r_end > end {
                return Err(fail("region outside payload"));
            }
            if r.executable && (!r.read_only || r.address % 4 != 0 || r.size == 0) {
                return Err(fail("invalid executable region"));
            }
            if r.zero_fill
                && (r.read_only
                    || r.executable
                    || self.bytes
                        [(r.address - self.base) as usize..(r_end - u64::from(self.base)) as usize]
                        .iter()
                        .any(|b| *b != 0))
            {
                return Err(fail("invalid zero-fill storage"));
            }
            if r.size != 0 {
                if spans
                    .iter()
                    .any(|&(a, b)| u64::from(r.address) < b && r_end > a)
                {
                    return Err(fail("overlapping regions"));
                }
                spans.push((u64::from(r.address), r_end));
            }
            if r.executable
                && self.entry >= r.address
                && u64::from(self.entry) < r_end
                && self.entry % 4 == 0
            {
                entry_valid = true;
            }
        }
        if !entry_valid {
            return Err(fail("entry outside aligned executable region"));
        }
        for (name, address) in &self.symbols {
            if name.is_empty()
                || name.contains('\0')
                || *address < self.base
                || u64::from(*address) > end
            {
                return Err(fail("invalid symbol"));
            }
        }
        Ok(())
    }
    /// Encode deterministic little-endian v1 metadata followed by relocated bytes.
    pub fn to_image_bytes(&self) -> Result<Vec<u8>, Error> {
        self.validate_image(u32::MAX)?;
        let mut out = MAGIC.to_vec();
        for v in [
            VERSION,
            self.base,
            self.entry,
            self.bytes.len() as u32,
            u32::try_from(self.regions.len()).map_err(|_| fail("too many regions"))?,
            u32::try_from(self.symbols.len()).map_err(|_| fail("too many symbols"))?,
        ] {
            word(&mut out, v);
        }
        for r in &self.regions {
            word(&mut out, r.address);
            word(&mut out, r.size);
            word(
                &mut out,
                u32::from(r.read_only)
                    | (u32::from(r.executable) << 1)
                    | (u32::from(r.zero_fill) << 2),
            );
            name(&mut out, &r.name)?;
        }
        for (n, a) in &self.symbols {
            word(&mut out, *a);
            name(&mut out, n)?;
        }
        out.extend_from_slice(&self.bytes);
        Ok(out)
    }
    /// Parse with independent encoded-input and guest-payload budgets. No mapping
    /// or execution occurs. Counts are bounded by bytes before allocating.
    pub fn from_image_bytes(
        bytes: &[u8],
        max_container: usize,
        max_payload: u32,
    ) -> Result<Self, Error> {
        if bytes.len() > max_container {
            return Err(fail("container exceeds budget"));
        }
        let mut input = Reader { bytes, cursor: 0 };
        if input.take(8)? != MAGIC {
            return Err(fail("unknown image magic/version"));
        }
        let version = input.word()?;
        if version != 1 && version != VERSION {
            return Err(fail("unknown image version"));
        }
        let base = input.word()?;
        let entry = input.word()?;
        let payload = input.word()?;
        if payload > max_payload {
            return Err(fail("payload exceeds budget"));
        }
        let regions = input.word()?;
        let symbols = input.word()?;
        if u64::from(regions) * 16 + u64::from(symbols) * 8 + u64::from(payload)
            > (bytes.len() - input.cursor) as u64
        {
            return Err(fail("impossible metadata counts"));
        }
        let mut image = Self {
            base,
            entry,
            bytes: Vec::new(),
            symbols: BTreeMap::new(),
            regions: Vec::new(),
        };
        for _ in 0..regions {
            let address = input.word()?;
            let size = input.word()?;
            let flags = input.word()?;
            if flags & !(if version == 1 { 3 } else { 7 }) != 0 {
                return Err(fail("unknown region flags"));
            }
            image.regions.push(ImageRegion {
                name: input.name()?,
                address,
                size,
                read_only: flags & 1 != 0,
                executable: flags & 2 != 0,
                zero_fill: flags & 4 != 0,
            });
        }
        for _ in 0..symbols {
            let address = input.word()?;
            let name = input.name()?;
            if image.symbols.insert(name, address).is_some() {
                return Err(fail("duplicate symbol"));
            }
        }
        image.bytes = input.take(payload as usize)?.to_vec();
        if input.cursor != bytes.len() {
            return Err(fail("trailing container bytes"));
        }
        image.validate_image(max_payload)?;
        Ok(image)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> LinkedImage {
        LinkedImage {
            base: 0x10000,
            entry: 0x10000,
            bytes: vec![0; 8],
            symbols: [("main".to_owned(), 0x10000)].iter().cloned().collect(),
            regions: vec![ImageRegion {
                name: "main".into(),
                address: 0x10000,
                size: 8,
                read_only: true,
                executable: true,
                zero_fill: false,
            }],
        }
    }
    #[test]
    fn deterministic_roundtrip_and_budgets() {
        let image = fixture();
        let bytes = image.to_image_bytes().unwrap();
        assert_eq!(
            LinkedImage::from_image_bytes(&bytes, bytes.len(), 8).unwrap(),
            image
        );
        assert_eq!(image.to_image_bytes().unwrap(), bytes);
        assert!(LinkedImage::from_image_bytes(&bytes, bytes.len() - 1, 8).is_err());
        assert!(LinkedImage::from_image_bytes(&bytes, bytes.len(), 7).is_err());
        for n in 0..bytes.len() {
            assert!(LinkedImage::from_image_bytes(&bytes[..n], bytes.len(), 8).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(LinkedImage::from_image_bytes(&trailing, trailing.len(), 8).is_err());
    }
    #[test]
    fn compiler_image_roundtrip() {
        let artifact =
            crate::compile_default("int value=7; int main(void) { return value; }").unwrap();
        let image = artifact.link_image(0x10000, "main", 0x10000).unwrap();
        let bytes = image.to_image_bytes().unwrap();
        assert_eq!(
            LinkedImage::from_image_bytes(&bytes, bytes.len(), 0x10000).unwrap(),
            image
        );
    }
    #[test]
    fn retains_v1_reading_and_rejects_forged_zero_fill() {
        let image = fixture();
        let mut legacy = image.to_image_bytes().unwrap();
        legacy[8..12].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            LinkedImage::from_image_bytes(&legacy, legacy.len(), 8).unwrap(),
            image
        );
        let mut image = crate::compile_default("int bss[4]; int main(void){return bss[0];}")
            .unwrap()
            .link_image(0x1000, "main", 0x1000)
            .unwrap();
        let offset = (image.symbols["bss"] - image.base) as usize;
        image.bytes[offset] = 1;
        assert!(image.to_image_bytes().is_err());
        let mut ram = vec![0xa5; 0x10000];
        assert!(image.load_into(&mut ram, 0x8000, 0xf000, 0x7000).is_err());
        assert!(ram.iter().all(|b| *b == 0xa5));
    }
    #[test]
    fn malformed_metadata_rejected() {
        let bytes = fixture().to_image_bytes().unwrap();
        for (offset, value) in [(8, 3u32), (24, u32::MAX), (40, 8), (16, 0x10008)] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(LinkedImage::from_image_bytes(&bad, bad.len(), 8).is_err());
        }
        let mut image = fixture();
        image.regions.push(image.regions[0].clone());
        assert!(image.to_image_bytes().is_err());
        image = fixture();
        image.regions[0].read_only = false;
        assert!(image.to_image_bytes().is_err());
        image = fixture();
        image.base = 0xfffffffc;
        image.entry = image.base;
        assert!(image.to_image_bytes().is_err());
    }
}
