use std::path::{Path, PathBuf};

use structopt::StructOpt;
use object::{ObjectSection, SectionKind, ObjectSymbol};
use object::read::Object;
use object::read::archive::ArchiveFile;

use crate::util::path::RelPath;

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(required = true)]
    objects: Vec<std::path::PathBuf>,
}

pub fn main(log: slog::Logger, opt: Opt) -> anyhow::Result<()> {
    for path in opt.objects {
        let log = log.new(slog::o!("path" => RelPath(path.clone())));
        match do_file(&log, &path) {
            Ok(()) => {}
            Err(e) => { slog::error!(log, "file error: {:?}", e); }
        }
    }

    Ok(())
}

fn do_file(log: &slog::Logger, path: &Path) -> anyhow::Result<()> {
    let data = std::fs::read(path)?;
    let objects = slog_scope::scope(&log, || read_objects(path, &data));

    for (path, object) in objects {
        let log = log.new(slog::o!("path" => RelPath(path.clone())));
        slog_scope::scope(&log, || {
            match do_object(&path, &object) {
                Ok(()) => {}
                Err(e) => { slog::error!(log, "object error: {:?}", e); }
            }
        });
    }

    Ok(())
}

fn read_objects<'a>(path: &Path, data: &'a [u8]) -> Vec<(PathBuf, object::read::NativeFile<'a>)> {
    // first try reading the file as an object:
    let initial_err = match object::read::NativeFile::parse(data) {
        Ok(object) => { return vec![(path.to_owned(), object)]; }
        Err(e) => e,
    };

    // if that fails, try as an archive but log original error in case
    // archive also fails:
    let archive = match ArchiveFile::parse(data) {
        Ok(archive) => archive,
        Err(_) => {
            log::error!("unknown file format: {:?}", initial_err);
            return vec![];
        }
    };

    archive.members()
        .filter_map(|member| match member {
            Ok(m) => Some(m),
            Err(e) => {
                log::warn!("failed to read member: {:?}", e);
                None
            }
        })
        .map(|member| {
            let name = String::from_utf8_lossy(member.name());
            let path = path.join(name.into_owned());
            (path, member)
        })
        .filter_map(|(path, member)| {
            let object = member.data(data)
                .and_then(object::read::NativeFile::parse);

            match object {
                Ok(object) => Some((path, object)),
                Err(e) => {
                    log::warn!("not an object: {:?}", e);
                    None
                }
            }
        })
        .collect()
}

fn do_object<'data: 'file, 'file, O: Object<'data, 'file>>(path: &Path, object: &'file O) -> anyhow::Result<()> {
    for symbol in object.symbols() {
        if symbol.is_common() {
            log::warn!("{}: common symbol: {}", RelPath(path), symbol.name()?);
            continue;
        }

        if symbol.is_undefined() {
            // this is an import
            continue;
        }

        let Some(section_idx) = symbol.section_index() else {
            log::warn!("{}: no section for symbol: {}", RelPath(path), symbol.name().unwrap());
            continue;
        };

        let section = object.section_by_index(section_idx).unwrap();

        match section.kind() {
            | SectionKind::Data
            | SectionKind::UninitializedData => {
                println!("{}: {}: {}",
                    RelPath(path), section.name()?, symbol.name()?);
            }
            _ => {}
        }
    }

    Ok(())
}
