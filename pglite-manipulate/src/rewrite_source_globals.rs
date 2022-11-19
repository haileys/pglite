use std::fmt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

use crate::util::path::{self, RelPath};

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(short = "I")]
    include: Vec<std::path::PathBuf>,

    #[structopt(short = "c")]
    source: Vec<std::path::PathBuf>,

    #[structopt(short = "r")]
    source_root: std::path::PathBuf,
}

pub fn main(log: slog::Logger, opt: Opt) -> anyhow::Result<()> {
    use std::fs;

    for (file, rewrites) in process(&log, opt)? {
        let result = fs::read_to_string(&file)
            .map(|source| apply(&source, &rewrites))
            .and_then(|rewritten| fs::write(&file, rewritten));

        match result {
            Ok(()) => {
                slog::info!(log, "rewrote {}", RelPath(file.clone()));
            }
            Err(e) => {
                slog::info!(log, "error rewriting {}: {:?}", RelPath(file.clone()), e);
            }
        }
    }

    Ok(())
}

pub fn apply(source: &str, rewrites: &[Rewrite]) -> String {
    let mut out = String::new();
    let mut cursor = 0;

    for rewrite in rewrites {
        out += &source[cursor..rewrite.offset];
        cursor += rewrite.length;
        out += &rewrite.text;
    }

    out += &source[cursor..];
    out
}

#[derive(Debug)]
pub struct Rewrite {
    pub offset: usize,
    pub length: usize,
    pub text: String,
}

pub fn process(log: &slog::Logger, opt: Opt) -> anyhow::Result<HashMap<PathBuf, Vec<Rewrite>>> {
    let clang = clang::Clang::new()
        .map_err(|e| anyhow::anyhow!("clang::Clang::new: {:?}", e))?;

    let index = clang::Index::new(&clang, false, true);

    let include_flags = opt.include.iter()
        .flat_map(|path| path::to_str(&log, path))
        .map(|path| format!("-I{}", path))
        .collect::<Vec<_>>();

    let ctx = Ctx {
        index: &index,
        clang_args: &include_flags,
        source_root: &opt.source_root,
    };

    let mut all_rewrites = Vec::new();

    for path in opt.source {
        let log = log.new(slog::o!("path" => RelPath(path.clone())));
        all_rewrites.extend(do_file(&log, &ctx, &path)?);
    }

    condense_rewrites(&log, all_rewrites)
}

struct FileRewrite {
    pub path: PathBuf,
    pub rewrite: Rewrite,
}

fn condense_rewrites(log: &slog::Logger, file_rewrites: Vec<FileRewrite>) -> anyhow::Result<HashMap<PathBuf, Vec<Rewrite>>> {
    use std::collections::{BTreeMap, HashSet};

    #[derive(Hash, PartialEq, Eq, Debug)]
    struct Subst {
        pub length: usize,
        pub text: String,
    }

    let mut rewrites_by_path = HashMap::<PathBuf, BTreeMap<usize, HashSet<Subst>>>::new();

    for FileRewrite { path, rewrite } in file_rewrites {
        rewrites_by_path
            .entry(path)
            .or_default()
            .entry(rewrite.offset)
            .or_default()
            .insert(Subst {
                length: rewrite.length,
                text: rewrite.text,
            });
    }

    let mut result = Ok(HashMap::<PathBuf, Vec<Rewrite>>::new());

    for (path, rewrites) in rewrites_by_path {
        for (offset, substs) in rewrites {
            // assert no conflicting substitutions
            if substs.len() > 1 {
                slog::error!(log, "conflicting substitutions";
                    "path" => RelPath(path.clone()),
                    "offset" => offset,
                    "substitutions" => format!("{:?}", substs),
                );

                result = Err(anyhow::anyhow!("conflicting substitutions"));
            } else {
                // take the single subst
                let subst = substs.into_iter().next().unwrap();
                if let Some(condensed) = result.as_mut().ok() {
                    condensed
                        .entry(path.clone())
                        .or_default()
                        .push(Rewrite {
                            offset: offset,
                            length: subst.length,
                            text: subst.text,
                        })
                }
            }
        }
    }

    result
}

struct Ctx<'a> {
    index: &'a clang::Index<'a>,
    clang_args: &'a [String],
    source_root: &'a Path,
}

fn do_file(log: &slog::Logger, ctx: &Ctx, path: &Path) -> anyhow::Result<Vec<FileRewrite>> {
    let mut rewrites = Vec::new();

    let tu = ctx.index.parser(path)
        .skip_function_bodies(true)
        .keep_going(true)
        .arguments(&ctx.clang_args)
        .parse()?;

    tu.get_entity().visit_children(|node, _parent| {
        if let Some(loc) = node.get_location() {
            let file = loc.get_spelling_location().file.unwrap();

            if !file.get_path().starts_with(ctx.source_root) {
                return clang::EntityVisitResult::Recurse;
            }
        }

        match node.get_kind() {
            clang::EntityKind::VarDecl => {
                // assert nothing we're touching is already TLS
                if node.get_tls_kind().is_some() {
                    slog::error!(log, "vardecl already has TLS kind";
                        "var" => node.get_name().unwrap(),
                        "loc" => Loc(&node),
                    );
                }

                let range = node.get_range().unwrap();
                let start = range.get_start().get_file_location();
                let start_offset = usize::try_from(start.offset).unwrap();

                rewrites.push(FileRewrite {
                    path: start.file.unwrap().get_path(),
                    rewrite: Rewrite {
                        offset: start_offset,
                        length: 0,
                        text: "__thread ".into(),
                    },
                });
            }
            _ => {}
        }

        return clang::EntityVisitResult::Recurse;
    });

    Ok(rewrites)
}

struct Loc<'a>(&'a clang::Entity<'a>);

impl<'a> fmt::Display for Loc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Some(loc) = self.0.get_location() else {
            return write!(f, "<unk>");
        };

        let loc = loc.get_spelling_location();

        let file = loc.file
            .map(|file| file.get_path().to_string_lossy().into_owned())
            .unwrap_or("<unk>".into());

        write!(f, "{}:{}:{}", file, loc.line, loc.column)
    }
}

impl<'a> slog::Value for Loc<'a> {
    fn serialize(
        &self,
        _rec: &slog::Record,
        key: slog::Key,
        ser: &mut dyn slog::Serializer
    ) -> slog::Result {
        ser.emit_str(key, &self.to_string())
    }
}
