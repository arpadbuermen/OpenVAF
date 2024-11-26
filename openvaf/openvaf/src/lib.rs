use std::fs::{create_dir_all, remove_file};
use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result};
use basedb::diagnostics::{ConsoleSink, DiagnosticSink};
pub use basedb::lints::{builtin as builtin_lints, LintLevel};
use basedb::BaseDB;
use camino::Utf8PathBuf;
use hir::CompilationDB;
use linker::link;
pub use llvm_sys::target_machine::LLVMCodeGenOptLevel;
use mir_llvm::LLVMBackend;
pub use paths::AbsPathBuf;
use sim_back::collect_modules;
pub use target::host_triple;
pub use target::spec::{get_target_names, Target};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod cache;

#[derive(Debug, Clone)]
pub enum CompilationDestination {
    Path { lib_file: Utf8PathBuf },
    Cache { cache_dir: Utf8PathBuf },
}

pub enum CompilationTermination {
    Compiled { lib_file: Utf8PathBuf },
    FatalDiagnostic,
}

#[derive(Debug, Clone)]
pub struct Opts {
    pub dry_run: bool,
    pub defines: Vec<String>,
    pub codegen_opts: Vec<String>,
    pub lints: Vec<(String, LintLevel)>,
    pub input: Utf8PathBuf,
    pub output: CompilationDestination,
    pub include: Vec<AbsPathBuf>,
    pub opt_lvl: LLVMCodeGenOptLevel,
    pub target: Target,
    pub target_cpu: String,
}
// pub fn dump_json(opts: &Opts) -> Result<CompilationTermination> {
//     let input =
//         opts.input.canonicalize().with_context(|| format!("failed to resolve {}", opts.input))?;
//     let input = AbsPathBuf::assert(input);
//     let db = CompilationDB::new_fs(input, &opts.include, &opts.defines, &opts.lints)?;
//     let modules = if let Some(modules) = collect_modules(&db, true, &mut ConsoleSink::new(&db)) {
//         modules
//     } else {
//         return Ok(CompilationTermination::FatalDiagnostic);
//     };
//     for module in modules {
//         let (func, intern, strings, cfg) = module.build_opvar_mir(&db);
//         let json = func.to_json(
//             &cfg,
//             &strings,
//             |param| match *intern.params.get_index(param).unwrap().0 {
//                 ParamKind::Param(param) => ("parameters", param.name(&db)),
//                 ParamKind::Abstime => ("sim_state", "$abstime".to_owned()),
//                 ParamKind::EnableIntegration => todo!(),
//                 ParamKind::Voltage { hi, lo: Some(lo) } => {
//                     ("voltages", format!("({}, {})", &hi.name(&db), &lo.name(&db)))
//                 }
//                 ParamKind::Voltage { hi, lo: None } => ("voltages", format!("({})", &hi.name(&db))),
//                 ParamKind::Current(hir_lower::CurrentKind::Unnamed { hi, lo: Some(lo) }) => {
//                     ("currents", format!("({}, {})", &hi.name(&db), &lo.name(&db)))
//                 }
//                 ParamKind::Current(hir_lower::CurrentKind::Unnamed { hi, lo: None }) => {
//                     ("currents", format!("({})", hi.name(&db)))
//                 }
//                 ParamKind::Current(hir_lower::CurrentKind::Branch(br)) => {
//                     ("currents", br.name(&db))
//                 }
//                 ParamKind::Temperature => ("sim_state", "$temperature".to_owned()),
//                 ParamKind::ParamGiven { param } => ("param_given", param.name(&db)),
//                 ParamKind::PortConnected { port } => ("port_connected", port.name(&db).to_string()),
//                 ParamKind::ParamSysFun(param) => ("params", format!("${param:?}")),
//                 _ => unreachable!(),
//             },
//             intern.outputs.iter().filter_map(|(kind, val)| {
//                 let name = match *kind {
//                     PlaceKind::Var(var) => var.name(&db).to_string(),
//                     _ => return None,
//                 };
//                 Some((name, val.expand()?))
//             }),
//         );
//         let path = opts.input.with_file_name(format!(
//             "{}_{}.json",
//             opts.input.file_stem().unwrap(),
//             module.module.name(&db)
//         ));
//         if !opts.dry_run {
//             std::fs::write(path, json)?;
//         }
//     }
//     Ok(CompilationTermination::Compiled { lib_file: Utf8PathBuf::default() })
// }

pub fn expand(opts: &Opts) -> Result<CompilationTermination> {
    let start = Instant::now();

    let input =
        opts.input.canonicalize().with_context(|| format!("failed to resolve {}", opts.input))?;
    let input = AbsPathBuf::assert(input);
    let db = CompilationDB::new_fs(input, &opts.include, &opts.defines, &opts.lints)?;
    let cu = db.compilation_unit();

    let preprocess = cu.preprocess(&db);
    for token in preprocess.ts.iter() {
        let span = token.span.to_file_span(&preprocess.sm);
        let text = db.file_text(span.file).unwrap();
        match token.kind {
            tokens::parser::SyntaxKind::COMMENT => {
                // Block comments are ok
                // Line comments should be dumped with a newline
                if !text[span.range].starts_with("/*") {
                    println!("{}", &text[span.range])
                } else {
                    print!("{}", &text[span.range])
                }
            }
            _ => {
                // Add a space after each token
                print!("{} ", &text[span.range])
            }
        };
    }
    println!();

    let mut sink = ConsoleSink::new(&db);
    sink.add_diagnostics(&*preprocess.diagnostics, cu.root_file(), &db);

    if sink.summary(&opts.input.file_name().unwrap()) {
        return Ok(CompilationTermination::FatalDiagnostic);
    }

    let seconds = Instant::elapsed(&start).as_secs_f64();
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(&mut stderr, "Finished")?;
    stderr.set_color(&ColorSpec::new())?;
    writeln!(&mut stderr, " preprocessing {} in {:.2}s", opts.input.file_name().unwrap(), seconds)?;

    Ok(CompilationTermination::Compiled { lib_file: Utf8PathBuf::default() })
}

pub fn compile(opts: &Opts) -> Result<CompilationTermination> {
    let start = Instant::now();

    let input =
        opts.input.canonicalize().with_context(|| format!("failed to resolve {}", opts.input))?;
    let input = AbsPathBuf::assert(input);
    let db = CompilationDB::new_fs(input, &opts.include, &opts.defines, &opts.lints)?;

    let lib_file = match &opts.output {
        CompilationDestination::Cache { cache_dir } => {
            let file_name = cache::file_name(&db, opts);
            let lib_file = cache_dir.join(file_name);
            if cfg!(not(debug_assertions)) && lib_file.exists() {
                return Ok(CompilationTermination::Compiled { lib_file });
            }
            create_dir_all(cache_dir).context("failed to create cache directory")?;
            lib_file
        }
        CompilationDestination::Path { lib_file } => lib_file.clone(),
    };

    let modules = if let Some(modules) = collect_modules(&db, false, &mut ConsoleSink::new(&db)) {
        modules
    } else {
        return Ok(CompilationTermination::FatalDiagnostic);
    };

    let back = LLVMBackend::new(&opts.codegen_opts, &opts.target, opts.target_cpu.clone(), &[]);
    if opts.dry_run {
        return Ok(CompilationTermination::Compiled { lib_file });
    }
    let paths = osdi::compile(&db, &modules, &lib_file, &opts.target, &back, true, opts.opt_lvl);
    // TODO configure linker
    link(None, &opts.target, lib_file.as_ref(), |linker| {
        for path in &paths {
            linker.add_object(path);
        }
    })?;

    for obj_file in paths {
        remove_file(obj_file).context("failed to delete intermediate compile artifact")?;
    }

    let seconds = Instant::elapsed(&start).as_secs_f64();
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(&mut stderr, "Finished")?;
    stderr.set_color(&ColorSpec::new())?;
    writeln!(&mut stderr, " building {} in {:.2}s", opts.input.file_name().unwrap(), seconds)?;

    Ok(CompilationTermination::Compiled { lib_file })
}
