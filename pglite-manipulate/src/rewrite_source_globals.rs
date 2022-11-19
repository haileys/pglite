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
        cursor = rewrite.offset + rewrite.length;
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
    slog::info!(log, "parsing file");

    let mut rewrites = Vec::new();

    let tu = ctx.index.parser(path)
        .skip_function_bodies(true)
        .include_attributed_types(true)
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
                // skip no_such_variable declarations - this is some postgres
                // macro hacks we don't need to touch
                if node.get_name().as_deref() == Some("no_such_variable") {
                    return clang::EntityVisitResult::Recurse;
                }

                // no need to make constants TLS
                match is_decl_constant(&node) {
                    Ok(true) => { return clang::EntityVisitResult::Recurse; }
                    Ok(false) => {}
                    Err(e) => {
                        slog::error!(log, "could not determine decl constness: {:?}", e);
                        return clang::EntityVisitResult::Recurse;
                    }
                }

                // assert nothing we're touching is already TLS
                if node.get_tls_kind().is_some() {
                    slog::error!(log, "vardecl already has TLS kind";
                        "var" => node.get_name().unwrap(),
                        "loc" => Loc(&node),
                    );
                }

                if let Ok(loc) = find_thread_kw_insert_loc(&node) {
                    // insert __thread keyword
                    rewrites.push(FileRewrite {
                        path: loc.file.unwrap().get_path(),
                        rewrite: Rewrite {
                            offset: usize::try_from(loc.offset).unwrap(),
                            length: 0,
                            text: "__thread ".into(),
                        },
                    });
                } else {
                    slog::error!(log, "couldn't find __thread insert loc";
                        "var" => node.get_name().unwrap(),
                        "loc" => Loc(&node),
                    );
                }
            }
            _ => {}
        }

        return clang::EntityVisitResult::Recurse;
    });

    Ok(rewrites)
}

fn is_decl_constant(var_decl: &clang::Entity) -> anyhow::Result<bool> {
    let Some(ty) = var_decl.get_type() else {
        anyhow::bail!("no type");
    };

    // println!("{:?}", ty.get_kind());

    if ty.is_const_qualified() {
        return Ok(true);
    }

    match ty.get_kind() {
        | clang::TypeKind::ConstantArray
        | clang::TypeKind::IncompleteArray => {
            if ty.get_element_type().unwrap().is_const_qualified() {
                return Ok(true);
            }
        }
        _ => {}
    }

    Ok(false)
}

fn find_thread_kw_insert_loc<'a>(var_decl: &clang::Entity<'a>)
    -> anyhow::Result<clang::source::Location<'a>>
{
    let Some(range) = var_decl.get_range() else {
        anyhow::bail!("no range for var decl");
    };

    let code = source_fragment(&range)?;

    // unfortunately libclang does not provide an API to find the range of the
    // type in a variable declaration.
    //
    // additionally, the tokenizer seems to not like the 'bool' keyword,
    // returning an empty token list for declarations of type bool (but working
    // for declarations of other types).
    //
    // so we have to do some dirty strings instead. I would love to fix this,
    // but it works for now.

    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(
            r"^\s*((const|static|extern|NON_EXEC_STATIC)\s*)*"
        ).unwrap();
    }

    let Some(mat) = RE.find(&code) else {
        anyhow::bail!("this should always match?");
    };


    let mut start = range.get_start().get_file_location();
    start.offset += u32::try_from(mat.end()).unwrap();
    Ok(start)
}

fn source_fragment(range: &clang::source::SourceRange)
    -> anyhow::Result<String>
{
    let start = range.get_start().get_file_location();
    let end = range.get_end().get_file_location();

    let Some(file) = start.file.clone() else {
        anyhow::bail!("no file");
    };

    anyhow::ensure!(file.get_id() == end.file.unwrap().get_id(),
        "start and end are in different files: {:?} and {:?}", start, end);

    let Some(contents) = file.get_contents() else {
        anyhow::bail!("no file contents");
    };

    let start_off = usize::try_from(start.offset).unwrap();
    let end_off = usize::try_from(end.offset).unwrap();

    Ok(contents[start_off..end_off].to_owned())
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
