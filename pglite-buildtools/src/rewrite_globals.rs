use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process;
use std::io::Read;
use structopt::StructOpt;
use serde::{Serialize, Deserialize};

use crate::util::path::{self, RelPath};

#[derive(StructOpt)]
#[structopt(flatten)]
pub enum Opt {
    Main(MainOpt),
    Worker(WorkerOpt),
}

#[derive(StructOpt, Clone, Serialize, Deserialize)]
pub struct MainOpt {
    #[structopt(short = "I")]
    pub include: Vec<std::path::PathBuf>,

    #[structopt(short = "c")]
    pub source: Vec<std::path::PathBuf>,

    #[structopt(short = "r")]
    pub source_root: std::path::PathBuf,
}

#[derive(StructOpt)]
pub struct WorkerOpt {
    #[structopt(long)]
    pub opts: String,
}

type WorkerResult = Vec<FileRewrite>;

pub fn main(log: slog::Logger, opt: MainOpt) -> anyhow::Result<()> {
    use std::fs;

    let file_rewrites = do_parallel(&log, opt)?;
    let condensed = condense_rewrites(&log, file_rewrites)?;

    for (file, rewrites) in condensed {
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

pub fn worker(log: slog::Logger, opt: WorkerOpt) -> anyhow::Result<()> {
    let opt = serde_json::from_str::<MainOpt>(&opt.opts)?;

    let clang = clang::Clang::new()
        .map_err(|e| anyhow::anyhow!("clang::Clang::new: {:?}", e))?;

    let index = clang::Index::new(&clang, false, true);

    let include_flags = opt.include.iter()
        .flat_map(|path| path::to_str(&log, path))
        .map(|path| format!("-I{}", path))
        .collect::<Vec<_>>();

    let clang_ctx = ClangCtx {
        index,
        clang_args: include_flags,
    };

    let mut all_rewrites = Vec::new();

    for path in opt.source {
        let log = log.new(slog::o!("path" => RelPath(path.clone())));

        let ctx = Ctx {
            log,
            clang: &clang_ctx,
            source_root: &opt.source_root,
        };

        all_rewrites.extend(do_file(&ctx, &path)?);
    }

    println!("{}", serde_json::to_string(&all_rewrites)?);
    Ok(())
}

fn do_parallel(_log: &slog::Logger, opt: MainOpt) -> anyhow::Result<Vec<FileRewrite>> {
    let ncpus = num_cpus::get();
    let sources_per_cpu = (opt.source.len() + (ncpus - 1)) / ncpus;

    Ok(opt.source.chunks(sources_per_cpu)
        .map(|chunk| -> anyhow::Result<process::Child> {
            let worker_opt = MainOpt {
                source: chunk.to_vec(),
                ..opt.clone()
            };

            let worker_opt_ser = serde_json::to_string(&worker_opt)?;

            let worker = process::Command::new(std::env::current_exe()?)
                .args(&["rewrite-globals-worker", "--opts", &worker_opt_ser])
                .stdout(process::Stdio::piped())
                .spawn()?;

            Ok(worker)
        })
        .collect::<anyhow::Result<Vec<process::Child>>>()?
        .into_iter()
        .map(|mut worker| -> anyhow::Result<WorkerResult> {
            let mut worker_result_ser = String::new();
            worker.stdout.take().unwrap().read_to_string(&mut worker_result_ser)?;

            if !worker.wait()?.success() {
                anyhow::bail!("worker failed");
            }

            let worker_result = serde_json::from_str::<WorkerResult>(&worker_result_ser)?;

            Ok(worker_result)
        })
        .collect::<anyhow::Result<Vec<WorkerResult>>>()?
        .into_iter()
        .flat_map(|rws| rws)
        .collect::<Vec<FileRewrite>>())
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Rewrite {
    pub offset: usize,
    pub length: usize,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
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

struct ClangCtx<'a> {
    index: clang::Index<'a>,
    clang_args: Vec<String>,
}

impl<'a> ClangCtx<'a> {
    pub fn parse(&self, path: &Path) -> anyhow::Result<clang::TranslationUnit> {
        Ok(self.index.parser(path)
            .keep_going(true)
            .arguments(&self.clang_args)
            .parse()?)
    }
}

struct Ctx<'a> {
    log: slog::Logger,
    clang: &'a ClangCtx<'a>,
    source_root: &'a Path,
}

fn do_file(ctx: &Ctx, path: &Path) -> anyhow::Result<Vec<FileRewrite>> {
    slog::info!(ctx.log, "parsing file");

    let tu = ctx.clang.parse(path)?;

    let mut rewrites = Vec::new();

    tu.get_entity().visit_children(|node, parent| {
        if let Some(loc) = node.get_location() {
            let file = loc.get_spelling_location().file.unwrap();

            if !file.get_path().starts_with(ctx.source_root) {
                return clang::EntityVisitResult::Recurse;
            }
        }

        match visit_node(&ctx, &mut rewrites, &node, &parent) {
            Ok(()) => {}
            Err(e) => {
                slog::error!(ctx.log, "error visiting {:?} node: {:?}", node.get_kind(), e);
            }
        }

        return clang::EntityVisitResult::Recurse;
    });

    Ok(rewrites)
}

fn visit_node(
    ctx: &Ctx,
    rewrites: &mut Vec<FileRewrite>,
    node: &clang::Entity,
    parent: &clang::Entity,
) -> anyhow::Result<()> {
    match (node.get_kind(), parent.get_kind()) {
        // global variables:
        (clang::EntityKind::VarDecl, clang::EntityKind::TranslationUnit) => {
            // skip no_such_variable declarations - this is some postgres
            // macro hacks we don't need to touch
            if node.get_name().as_deref() == Some("no_such_variable") {
                return Ok(());
            }

            handle_var_decl(&ctx.log, rewrites, &node)
        }

        // static local variables:
        (clang::EntityKind::VarDecl, _) if is_static(node) => {
            handle_var_decl(&ctx.log, rewrites, &node)
        }

        _ => Ok(())
    }
}

fn handle_var_decl(log: &slog::Logger, rewrites: &mut Vec<FileRewrite>, node: &clang::Entity) -> anyhow::Result<()> {
    // no need to make constants TLS
    if is_decl_constant(&node)? {
        return Ok(());
    }

    // assert nothing we're touching is already TLS
    if node.get_tls_kind().is_some() {
        slog::error!(log, "vardecl already has TLS kind";
            "var" => node.get_name().unwrap(),
            "loc" => Loc(&node),
        );
    }

    let loc = find_thread_kw_insert_loc(&node)?;

    // insert __thread keyword
    rewrites.push(FileRewrite {
        path: loc.file.unwrap().get_path(),
        rewrite: Rewrite {
            offset: usize::try_from(loc.offset).unwrap(),
            length: 0,
            text: "__thread ".into(),
        },
    });

    Ok(())
}

fn is_static(node: &clang::Entity) -> bool {
    node.get_storage_class() == Some(clang::StorageClass::Static)
}

fn is_decl_constant(var_decl: &clang::Entity) -> anyhow::Result<bool> {
    let Some(ty) = var_decl.get_type() else {
        anyhow::bail!("no type");
    };

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
