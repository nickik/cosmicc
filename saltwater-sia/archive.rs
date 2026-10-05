//! Lazy static libraries in ordinary Unix ar envelopes; members are SIA bundles.
use crate::{Artifact, Error};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct StaticArchive {
    pub members: Vec<(String, Artifact)>,
}
fn error(message: impl std::fmt::Display) -> Error {
    Error::Codegen(format!("SIA archive: {message}"))
}
impl StaticArchive {
    pub fn from_bytes(bytes: &[u8], max_bytes: usize) -> Result<Self, Error> {
        if bytes.len() > max_bytes || !bytes.starts_with(b"!<arch>\n") {
            return Err(error(
                "invalid archive magic or byte budget (thin archives unsupported)",
            ));
        }
        let mut cursor: usize = 8;
        let mut names: &[u8] = &[];
        let mut members = Vec::new();
        while cursor < bytes.len() {
            let header = bytes
                .get(
                    cursor
                        ..cursor
                            .checked_add(60)
                            .ok_or_else(|| error("header overflow"))?,
                )
                .ok_or_else(|| error("truncated member header"))?;
            if &header[58..60] != b"`\n" {
                return Err(error("invalid member trailer"));
            }
            let size: usize = std::str::from_utf8(&header[48..58])
                .map_err(error)?
                .trim()
                .parse()
                .map_err(error)?;
            cursor += 60;
            let end = cursor
                .checked_add(size)
                .ok_or_else(|| error("member size overflow"))?;
            let mut body = bytes
                .get(cursor..end)
                .ok_or_else(|| error("truncated member data"))?;
            cursor = end;
            if size % 2 != 0 {
                if bytes.get(cursor) != Some(&b'\n') {
                    return Err(error("missing member padding"));
                }
                cursor += 1;
            }
            let field = std::str::from_utf8(&header[..16]).map_err(error)?.trim();
            if field == "//" {
                names = body;
                continue;
            }
            if field == "/" || field == "/SYM64/" || field.starts_with("__.SYMDEF") {
                continue;
            }
            let name = if let Some(length) = field.strip_prefix("#1/") {
                let length: usize = length.parse().map_err(error)?;
                let raw = body
                    .get(..length)
                    .ok_or_else(|| error("truncated BSD member name"))?;
                body = &body[length..];
                std::str::from_utf8(raw)
                    .map_err(error)?
                    .trim_end_matches('\0')
                    .to_owned()
            } else if let Some(offset) = field.strip_prefix('/') {
                let offset: usize = offset.parse().map_err(error)?;
                let tail = names
                    .get(offset..)
                    .ok_or_else(|| error("invalid GNU name offset"))?;
                let end = tail
                    .windows(2)
                    .position(|w| w == b"/\n")
                    .ok_or_else(|| error("unterminated GNU name"))?;
                std::str::from_utf8(&tail[..end]).map_err(error)?.to_owned()
            } else {
                field.trim_end_matches('/').to_owned()
            };
            if name.starts_with("__.SYMDEF") {
                continue;
            }
            if name.is_empty() || name.contains('\0') {
                return Err(error("invalid member name"));
            }
            let object = if body.starts_with(b"\x7fELF") {
                Artifact::from_elf_bytes(body)
            } else {
                Artifact::from_bytes(body)
            }
            .map_err(|e| error(format!("member {name}: {e}")))?;
            if object.local_symbols.is_none() {
                return Err(error("archive members require v4 binding metadata"));
            }
            members.push((name, object));
        }
        Ok(Self { members })
    }

    /// Append only members satisfying unresolved externals. Repeat within this
    /// archive to cover backwards dependencies. Caller processes archives in
    /// command order after explicit objects; cross-archive groups are unsupported.
    pub fn extract_needed(
        &self,
        objects: &mut Vec<Artifact>,
        entry: &str,
    ) -> Result<Vec<String>, Error> {
        let mut selected = BTreeSet::new();
        let mut extracted = Vec::new();
        loop {
            let mut definitions = BTreeSet::new();
            let mut references = BTreeSet::from([entry.to_owned()]);
            for object in objects.iter() {
                object.validate()?;
                let locals = object
                    .local_symbols
                    .as_ref()
                    .ok_or_else(|| error("archive linking requires v4 objects"))?;
                definitions.extend(
                    object
                        .functions
                        .iter()
                        .map(|f| &f.name)
                        .chain(object.data.iter().map(|d| &d.name))
                        .filter(|n| !locals.contains(*n))
                        .cloned(),
                );
                references.extend(
                    object
                        .functions
                        .iter()
                        .flat_map(|f| &f.relocations)
                        .chain(object.data.iter().flat_map(|d| &d.relocations))
                        .filter(|r| !locals.contains(&r.target))
                        .map(|r| r.target.clone()),
                );
            }
            references.retain(|name| !definitions.contains(name));
            let candidate = self
                .members
                .iter()
                .enumerate()
                .find(|(index, (_, object))| {
                    !selected.contains(index)
                        && object
                            .functions
                            .iter()
                            .map(|f| &f.name)
                            .chain(object.data.iter().map(|d| &d.name))
                            .any(|name| {
                                !object.local_symbols.as_ref().unwrap().contains(name)
                                    && references.contains(name)
                            })
                });
            let Some((index, (name, object))) = candidate else {
                break;
            };
            selected.insert(index);
            extracted.push(name.clone());
            objects.push(object.clone());
        }
        Ok(extracted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_default;
    fn archive(members: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut bytes = b"!<arch>\n".to_vec();
        for (name, data) in members {
            bytes.extend(
                format!(
                    "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
                    name,
                    0,
                    0,
                    0,
                    0,
                    data.len()
                )
                .as_bytes(),
            );
            bytes.extend(data);
            if data.len() % 2 != 0 {
                bytes.push(b'\n');
            }
        }
        bytes
    }
    #[test]
    fn lazy_extraction_resolves_backwards_edges_and_ignores_unused_imports() {
        let a = compile_default("int leaf(void){return 7;}").unwrap();
        let b = compile_default("extern int leaf(void); int used(void){return leaf();}").unwrap();
        let dead = compile_default("extern int missing(void); int unused(void){return missing();}")
            .unwrap();
        let bytes = archive(&[
            ("leaf.sia/", a.to_bytes().unwrap()),
            ("used.sia/", b.to_bytes().unwrap()),
            ("dead.sia/", dead.to_bytes().unwrap()),
        ]);
        let library = StaticArchive::from_bytes(&bytes, bytes.len()).unwrap();
        let mut objects =
            vec![compile_default("extern int used(void); int main(void){return used();}").unwrap()];
        assert_eq!(
            library.extract_needed(&mut objects, "main").unwrap(),
            ["used.sia", "leaf.sia"]
        );
        let image = Artifact::link_images(&objects, 0x1000, "main", 0x1000).unwrap();
        assert!(!image.symbols.contains_key("unused"));
        for n in 0..bytes.len() {
            if ![
                8,
                8 + 60 + a.to_bytes().unwrap().len() + (a.to_bytes().unwrap().len() % 2),
                8 + 120
                    + a.to_bytes().unwrap().len()
                    + a.to_bytes().unwrap().len() % 2
                    + b.to_bytes().unwrap().len()
                    + b.to_bytes().unwrap().len() % 2,
            ]
            .contains(&n)
            {
                assert!(StaticArchive::from_bytes(&bytes[..n], bytes.len()).is_err());
            }
        }
    }

    #[test]
    fn lazy_extraction_accepts_relocatable_sia_elf_archive_members() {
        let main = compile_default("extern int helper(void); int main(void){return helper();}")
            .unwrap()
            .to_elf_bytes()
            .unwrap();
        let helper = compile_default("int helper(void){return 0;}")
            .unwrap()
            .to_elf_bytes()
            .unwrap();
        let mut objects = vec![Artifact::from_elf_bytes(&main).unwrap()];
        let archive = StaticArchive::from_bytes(&archive(&[("helper.o", helper)]), 4096).unwrap();
        assert_eq!(
            archive.extract_needed(&mut objects, "main").unwrap(),
            ["helper.o"]
        );
        assert_eq!(objects.len(), 2);
    }
    #[test]
    fn accepts_bsd_embedded_names_and_skips_embedded_symbol_indexes() {
        let mut index = b"__.SYMDEF\0\0\0".to_vec();
        index.extend([0; 4]);
        let mut member = b"long-name-object.sia".to_vec();
        member.extend(
            compile_default("int main(void){return 0;}")
                .unwrap()
                .to_bytes()
                .unwrap(),
        );
        let bytes = archive(&[("#1/12", index), ("#1/20", member)]);
        let library = StaticArchive::from_bytes(&bytes, bytes.len()).unwrap();
        assert_eq!(library.members.len(), 1);
        assert_eq!(library.members[0].0, "long-name-object.sia");
    }
    #[test]
    fn accepts_gnu_long_names_and_rejects_truncated_members() {
        let object = compile_default("int main(void){return 0;}")
            .unwrap()
            .to_bytes()
            .unwrap();
        let bytes = archive(&[("//", b"long-object-name.sia/\n".to_vec()), ("/0", object)]);
        let library = StaticArchive::from_bytes(&bytes, bytes.len()).unwrap();
        assert_eq!(library.members[0].0, "long-object-name.sia");
        assert!(StaticArchive::from_bytes(&bytes, bytes.len() - 1).is_err());
        assert!(StaticArchive::from_bytes(b"!<thin>\n", 8).is_err());
    }
}
