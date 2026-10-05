//! Bounded static linking with explicit translation-unit-local binding metadata.
mod format;
use crate::{Artifact, Error, RelocationArtifact};
use std::collections::BTreeMap;
use std::convert::TryFrom;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRegion {
    pub name: String,
    pub address: u32,
    pub size: u32,
    pub read_only: bool,
    pub executable: bool,
}

/// A fully relocated, flat target image. Permissions describe the intended
/// mapping; copying bytes into simulator RAM does not enforce them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedImage {
    pub base: u32,
    pub entry: u32,
    pub bytes: Vec<u8>,
    pub symbols: BTreeMap<String, u32>,
    pub regions: Vec<ImageRegion>,
}

fn fail(message: impl Into<String>) -> Error {
    Error::Codegen(format!("SIA image link: {}", message.into()))
}

fn reserve(cursor: &mut u64, size: usize, align: u32, limit: u64) -> Result<u32, Error> {
    let start = cursor
        .checked_add(u64::from(align) - 1)
        .map(|value| value & !(u64::from(align) - 1))
        .ok_or_else(|| fail("alignment overflow"))?;
    let end = start
        .checked_add(size as u64)
        .ok_or_else(|| fail("image size overflow"))?;
    if start > u64::from(u32::MAX) || end > limit {
        return Err(fail("image exceeds its address range or byte budget"));
    }
    *cursor = end;
    Ok(start as u32)
}

fn relocate(
    image: &mut [u8],
    base: u32,
    region: &ImageRegion,
    relocations: &[RelocationArtifact],
    symbols: &BTreeMap<String, u32>,
) -> Result<(), Error> {
    let mut occupied = Vec::new();
    for relocation in relocations {
        let end = relocation
            .offset
            .checked_add(4)
            .ok_or_else(|| fail("relocation offset overflow"))?;
        if end > region.size {
            return Err(fail(format!("relocation outside `{}`", region.name)));
        }
        if occupied
            .iter()
            .any(|&(start, previous_end)| relocation.offset < previous_end && end > start)
        {
            return Err(fail(format!(
                "overlapping relocations in `{}`",
                region.name
            )));
        }
        occupied.push((relocation.offset, end));
        let symbol = symbols.get(&relocation.target).ok_or_else(|| {
            fail(format!(
                "unresolved symbol `{}` in `{}`",
                relocation.target, region.name
            ))
        })?;
        let value = i64::from(*symbol)
            .checked_add(relocation.addend)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| {
                fail(format!(
                    "Abs4 relocation overflow for `{}`",
                    relocation.target
                ))
            })?;
        let offset =
            (u64::from(region.address) - u64::from(base) + u64::from(relocation.offset)) as usize;
        image[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    Ok(())
}

impl Artifact {
    /// Link v4 objects with explicit TU-local binding. Common/weak merging is unsupported.
    pub fn link_images(
        objects: &[Artifact],
        base: u32,
        entry: &str,
        max_bytes: u32,
    ) -> Result<LinkedImage, Error> {
        Self::merge_objects(objects, entry)?.link_image(base, entry, max_bytes)
    }

    /// Retain the entry and its transitive function/data relocation closure.
    /// Validation and duplicate-definition checks still inspect every input.
    pub fn link_reachable_images(
        objects: &[Artifact],
        base: u32,
        entry: &str,
        max_bytes: u32,
    ) -> Result<LinkedImage, Error> {
        let mut merged = Self::merge_objects(objects, entry)?;
        for object in objects {
            for (name, size, edges) in object
                .functions
                .iter()
                .map(|f| (&f.name, f.code.len(), &f.relocations))
                .chain(
                    object
                        .data
                        .iter()
                        .map(|d| (&d.name, d.bytes.len(), &d.relocations)),
                )
            {
                let mut occupied = Vec::new();
                for edge in edges {
                    let end = edge
                        .offset
                        .checked_add(4)
                        .ok_or_else(|| fail("relocation offset overflow"))?;
                    if end as usize > size {
                        return Err(fail(format!("relocation outside `{name}`")));
                    }
                    if occupied
                        .iter()
                        .any(|&(start, previous_end)| edge.offset < previous_end && end > start)
                    {
                        return Err(fail(format!("overlapping relocations in `{name}`")));
                    }
                    occupied.push((edge.offset, end));
                }
            }
        }
        let dependencies: BTreeMap<_, _> = merged
            .functions
            .iter()
            .map(|f| (f.name.clone(), &f.relocations))
            .chain(merged.data.iter().map(|d| (d.name.clone(), &d.relocations)))
            .collect();
        let mut live = std::collections::BTreeSet::new();
        let mut pending = vec![entry.to_owned()];
        while let Some(name) = pending.pop() {
            if !live.insert(name.clone()) {
                continue;
            }
            let edges = dependencies
                .get(&name)
                .ok_or_else(|| fail(format!("unresolved reachable symbol `{name}`")))?;
            pending.extend(edges.iter().map(|r| r.target.clone()));
        }
        merged.functions.retain(|f| live.contains(&f.name));
        merged.data.retain(|d| live.contains(&d.name));
        if let Some(locals) = merged.local_symbols.as_mut() {
            locals.retain(|name| live.contains(name));
        }
        merged.link_image(base, entry, max_bytes)
    }

    fn merge_objects(objects: &[Artifact], entry: &str) -> Result<Artifact, Error> {
        if objects.is_empty() {
            return Err(fail("no input objects"));
        }
        if objects.len() == 1 {
            objects[0].validate()?;
            return Ok(objects[0].clone());
        }
        let mut merged = Artifact {
            target: crate::TARGET,
            local_symbols: Some(Default::default()),
            functions: Vec::new(),
            data: Vec::new(),
        };
        let mut occupied = std::collections::BTreeSet::new();
        for object in objects {
            object.validate()?;
            let locals = object.local_symbols.as_ref().ok_or_else(|| {
                fail("multi-object linking requires v4 binding metadata; recompile legacy v3 input")
            })?;
            for name in object
                .functions
                .iter()
                .map(|f| &f.name)
                .chain(object.data.iter().map(|d| &d.name))
            {
                if !locals.contains(name) && !occupied.insert(name.clone()) {
                    return Err(fail(format!("duplicate external definition `{name}` (common/weak coalescing unsupported)")));
                }
            }
        }
        if !occupied.contains(entry) {
            return Err(fail(format!(
                "entry `{entry}` is not an external definition"
            )));
        }
        // Unresolved external spellings must never alias generated local names.
        for object in objects {
            let locals = object.local_symbols.as_ref().unwrap();
            for relocation in object
                .functions
                .iter()
                .flat_map(|f| &f.relocations)
                .chain(object.data.iter().flat_map(|d| &d.relocations))
            {
                if !locals.contains(&relocation.target) {
                    occupied.insert(relocation.target.clone());
                }
            }
        }
        for (ordinal, object) in objects.iter().enumerate() {
            let mut names = BTreeMap::new();
            for name in object.local_symbols.as_ref().unwrap() {
                let mut scoped = format!("@tu{ordinal}:{name}");
                while occupied.contains(&scoped) {
                    scoped.push('@');
                }
                occupied.insert(scoped.clone());
                merged
                    .local_symbols
                    .as_mut()
                    .unwrap()
                    .insert(scoped.clone());
                names.insert(name.clone(), scoped);
            }
            let rename = |name: &String| names.get(name).cloned().unwrap_or_else(|| name.clone());
            for function in &object.functions {
                let mut function = function.clone();
                function.name = rename(&function.name);
                for relocation in &mut function.relocations {
                    relocation.target = rename(&relocation.target);
                }
                merged.functions.push(function);
            }
            for data in &object.data {
                let mut data = data.clone();
                data.name = rename(&data.name);
                for relocation in &mut data.relocations {
                    relocation.target = rename(&relocation.target);
                }
                merged.data.push(data);
            }
        }
        Ok(merged)
    }

    /// Resolve one bundle's internal Abs4 references into a target RAM image.
    /// The caller supplies an explicit byte budget; no imports or host symbols
    /// are resolved. For separate TUs use link_images with explicit v4 binding metadata.
    pub fn link_image(&self, base: u32, entry: &str, max_bytes: u32) -> Result<LinkedImage, Error> {
        self.validate()?;
        if base % 4 != 0 {
            return Err(fail(
                "base must preserve four-byte SIA literal-pool alignment",
            ));
        }
        if !self.functions.iter().any(|function| function.name == entry) {
            return Err(fail(format!("entry `{entry}` is not a defined function")));
        }
        let limit = (u64::from(base) + u64::from(max_bytes)).min(1u64 << 32);
        let mut cursor = u64::from(base);
        let mut symbols = BTreeMap::new();
        let mut regions = Vec::new();
        for function in &self.functions {
            let address = reserve(&mut cursor, function.code.len(), 4, limit)?;
            symbols.insert(function.name.clone(), address);
            regions.push(ImageRegion {
                name: function.name.clone(),
                address,
                size: u32::try_from(function.code.len())
                    .map_err(|_| fail("function exceeds SIA32 size"))?,
                read_only: true,
                executable: true,
            });
        }
        for object in &self.data {
            let address = reserve(&mut cursor, object.bytes.len(), object.align, limit)?;
            symbols.insert(object.name.clone(), address);
            regions.push(ImageRegion {
                name: object.name.clone(),
                address,
                size: u32::try_from(object.bytes.len())
                    .map_err(|_| fail("data exceeds SIA32 size"))?,
                read_only: object.read_only,
                executable: false,
            });
        }
        let length = usize::try_from(cursor - u64::from(base))
            .map_err(|_| fail("image does not fit host address space"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| fail("cannot allocate image buffer"))?;
        bytes.resize(length, 0);
        for (index, function) in self.functions.iter().enumerate() {
            let region = &regions[index];
            let start = (region.address - base) as usize;
            bytes[start..start + function.code.len()].copy_from_slice(&function.code);
            relocate(&mut bytes, base, region, &function.relocations, &symbols)?;
        }
        for (index, object) in self.data.iter().enumerate() {
            let region = &regions[self.functions.len() + index];
            let start = (region.address - base) as usize;
            bytes[start..start + object.bytes.len()].copy_from_slice(&object.bytes);
            relocate(&mut bytes, base, region, &object.relocations, &symbols)?;
        }
        Ok(LinkedImage {
            base,
            entry: symbols[entry],
            bytes,
            symbols,
            regions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile_default, FunctionArtifact, TARGET};

    fn fixture() -> Artifact {
        Artifact {
            local_symbols: None,
            target: TARGET,
            functions: vec![FunctionArtifact {
                name: "main".into(),
                code: vec![0; 8],
                relocations: vec![RelocationArtifact {
                    offset: 2,
                    target: "main".into(),
                    addend: 2,
                }],
            }],
            data: vec![],
        }
    }

    #[test]
    fn links_compiled_calls_global_addresses_and_function_pointers() {
        let artifact = compile_default("int value = 7; int *ptr = &value; int get(void) { return *ptr; } int (*fn)(void) = get; int main(void) { return fn() + get(); }").unwrap();
        let image = artifact.link_image(0x8000, "main", 65536).unwrap();
        assert_eq!(image.entry, image.symbols["main"]);
        for object in &artifact.data {
            for relocation in &object.relocations {
                let offset =
                    (image.symbols[&object.name] - image.base + relocation.offset) as usize;
                assert_eq!(
                    &image.bytes[offset..offset + 4],
                    &(image.symbols[&relocation.target] as i64 + relocation.addend).to_le_bytes()
                        [..4]
                );
            }
        }
        assert_eq!(image, artifact.link_image(0x8000, "main", 65536).unwrap());
    }

    #[test]
    fn applies_unaligned_abs4_little_endian() {
        let image = fixture().link_image(0x8000, "main", 8).unwrap();
        assert_eq!(&image.bytes[2..6], &[2, 128, 0, 0]);
    }

    #[test]
    fn rejects_invalid_layout_and_relocations() {
        assert!(fixture().link_image(1, "main", 8).is_err());
        assert!(fixture().link_image(0, "missing", 8).is_err());
        assert!(fixture().link_image(0, "main", 7).is_err());
        assert!(fixture().link_image(u32::MAX - 1, "main", 8).is_err());
        for (offset, target, addend) in [
            (6, "main", 0),
            (0, "missing", 0),
            (0, "main", -1),
            (0, "main", i64::MAX),
        ] {
            let mut artifact = fixture();
            artifact.functions[0].relocations[0] = RelocationArtifact {
                offset,
                target: target.into(),
                addend,
            };
            assert!(artifact.link_image(0, "main", 8).is_err());
        }
        let mut artifact = fixture();
        artifact.functions[0].relocations.push(RelocationArtifact {
            offset: 4,
            target: "main".into(),
            addend: 0,
        });
        assert!(artifact.link_image(0, "main", 8).is_err());
    }
    #[test]
    fn scopes_static_functions_data_strings_and_resolves_externs() {
        let a = compile_default("static int value=3; static int helper(void){return value;} char *message(void){return \"a\";} int get(void){return helper();}").unwrap();
        let b = compile_default("static int value=7; static int helper(void){return value;} extern int get(void); extern char *message(void); int main(void){return get()+helper()+message()[0]-97;}").unwrap();
        let a = Artifact::from_bytes(&a.to_bytes().unwrap()).unwrap();
        let b = Artifact::from_bytes(&b.to_bytes().unwrap()).unwrap();
        let image = Artifact::link_images(&[a, b], 0x10000, "main", 65536).unwrap();
        assert!(image.symbols.contains_key("get"));
        assert_eq!(
            image
                .regions
                .iter()
                .filter(|r| r.name.contains("_value"))
                .count(),
            2
        );
        assert_eq!(
            image
                .regions
                .iter()
                .filter(|r| r.name.contains("_helper"))
                .count(),
            2
        );
        image.validate_image(65536).unwrap();
    }
    #[test]
    fn rejects_duplicate_unresolved_and_legacy_bindings() {
        let a = compile_default("int get(void){return 1;}").unwrap();
        let b = compile_default("int get(void){return 2;} int main(void){return get();}").unwrap();
        assert!(Artifact::link_images(&[a.clone(), b], 0, "main", 65536)
            .unwrap_err()
            .to_string()
            .contains("duplicate external"));
        let b =
            compile_default("extern int missing(void); int main(void){return missing();}").unwrap();
        assert!(
            Artifact::link_images(&[a.clone(), b.clone()], 0, "main", 65536)
                .unwrap_err()
                .to_string()
                .contains("unresolved symbol")
        );
        let mut legacy = a;
        legacy.local_symbols = None;
        assert!(Artifact::link_images(&[legacy, b], 0, "main", 65536)
            .unwrap_err()
            .to_string()
            .contains("v4 binding"));
        let a = compile_default("int common;\n").unwrap();
        let b = compile_default("int common; int main(void){return common;}").unwrap();
        assert!(Artifact::link_images(&[a, b], 0, "main", 65536)
            .unwrap_err()
            .to_string()
            .contains("common/weak"));
    }
    #[test]
    fn undefined_static_reference_cannot_bind_an_external_definition() {
        let error = compile_default("static int f(void); int main(void){return f();}").unwrap_err();
        assert!(error.to_string().contains("undefined TU-local symbol"));
        assert!(compile_default("static int unused(void); int main(void){return 0;}").is_ok());
        assert!(compile_default(
            "static int f(void); static int f(void){return 3;} int main(void){return f();}"
        )
        .is_ok());
    }
    #[test]
    fn reachable_link_retains_data_function_pointers_and_drops_dead_imports() {
        let a = crate::compile_default("extern int get(void);extern int unavailable(void);int unused(void){return unavailable();}int main(void){return get()-5;}").unwrap();
        let b = crate::compile_default("static int hidden(void){return 5;}static int (*callback)(void)=hidden;int get(void){return callback();}").unwrap();
        assert!(Artifact::link_images(&[a.clone(), b.clone()], 0x10000, "main", 65536).is_err());
        let image = Artifact::link_reachable_images(&[a, b], 0x10000, "main", 65536).unwrap();
        assert!(!image.symbols.contains_key("unused"));
        assert!(image.symbols.keys().any(|name| name.contains("_hidden")));
        assert!(image.regions.iter().any(|r| !r.executable && !r.read_only));
    }
    #[test]
    fn reachable_link_rejects_live_missing_symbols_and_duplicate_dead_definitions() {
        let a=crate::compile_default("extern int unavailable(void);int main(void){return unavailable();}int dead(void){return 0;}").unwrap();
        assert!(Artifact::link_reachable_images(&[a.clone()], 0, "main", 65536).is_err());
        let b = crate::compile_default("int dead(void){return 1;}").unwrap();
        assert!(format!(
            "{}",
            Artifact::link_reachable_images(&[a, b], 0, "main", 65536).unwrap_err()
        )
        .contains("duplicate external"));
    }

    #[test]
    fn reachable_link_validates_discarded_relocation_shapes() {
        let mut a = crate::compile_default(
            "extern int missing(void);int unused(void){return missing();}int main(void){return 0;}",
        )
        .unwrap();
        let unused = a.functions.iter_mut().find(|f| f.name == "unused").unwrap();
        unused.relocations[0].offset = u32::MAX;
        assert!(Artifact::link_reachable_images(&[a], 0, "main", 65536).is_err());
    }
}
