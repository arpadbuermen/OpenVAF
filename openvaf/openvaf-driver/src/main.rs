use std::io::Write;
use std::process::exit;
use std::sync::Mutex;

use anyhow::{bail, Result};
use camino::Utf8PathBuf;
use clap::ArgMatches;
use cli_def::{main_command, INPUT};
use mimalloc::MiMalloc;
use openvaf::{compile, expand, CompilationDestination, CompilationTermination, Opts};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::cli_def::{DUMP_JSON, PRINT_EXPANSION};
use crate::cli_process::matches_to_opts;

mod cli_def;
mod cli_process;
mod crash_report;

static ARGS: Mutex<Option<Opts>> = Mutex::new(None);
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub fn main() {
    let matches = main_command().get_matches();
    crash_report::install_panic_handler();
    let input: Utf8PathBuf = matches.get_one(INPUT).cloned().unwrap_or_else(Utf8PathBuf::new);
    let env = env_logger::Env::default().filter("OPENVAF_LOG").write_style("OPENVAF_LOG_STYLE");
    env_logger::Builder::new()
        .format_timestamp(None)
        .filter(Some("salsa"), log::LevelFilter::Off)
        .filter_level(log::LevelFilter::Off)
        .parse_env(env)
        .init();
    match wrapped_main(matches) {
        Ok(err_code) => exit(err_code),
        Err(err) => {
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);

            for cause in err.chain() {
                stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true)).unwrap();
                write!(&mut stderr, "error").unwrap();
                stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
                write!(&mut stderr, ":").unwrap();
                stderr.set_color(&ColorSpec::new()).unwrap();
                writeln!(&mut stderr, " {cause}").unwrap();
            }

            stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true)).unwrap();
            write!(&mut stderr, "error").unwrap();
            stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
            write!(&mut stderr, ":").unwrap();
            stderr.set_color(&ColorSpec::new()).unwrap();
            writeln!(&mut stderr, " failed to compile {input}").unwrap();
        }
    }
}

pub const DATA_ERROR: i32 = 65;

fn wrapped_main(matches: ArgMatches) -> Result<i32> {
    let print_expansion = matches.get_flag(PRINT_EXPANSION);
    let dump_json_ = matches.get_flag(DUMP_JSON);
    let opts = matches_to_opts(matches)?;
    *ARGS.lock().unwrap() = Some(opts.clone());
    if print_expansion {
        let res = match expand(&opts)? {
            CompilationTermination::Compiled { .. } => 0,
            CompilationTermination::FatalDiagnostic => DATA_ERROR,
        };
        return Ok(res);
    }
    if dump_json_ {
        bail!("currently unimplemented");
        // let res = match dump_json(&opts)? {
        //     CompilationTermination::Compiled { .. } => 0,
        //     CompilationTermination::FatalDiagnostic => DATA_ERROR,
        // };
        // return Ok(res);
    }

    let res = match compile(&opts)? {
        CompilationTermination::Compiled { lib_file } => {
            if matches!(opts.output, CompilationDestination::Cache { .. }) {
                println!("{lib_file}");
            }
            0
        }
        CompilationTermination::FatalDiagnostic => DATA_ERROR,
    };

    Ok(res)
}
