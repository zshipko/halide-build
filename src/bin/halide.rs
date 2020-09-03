use halide_build::*;

use clap::{App, Arg, SubCommand};

use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

static mut QUIET: bool = false;

macro_rules! log {
    ($fmt:expr, $($arg:tt)*) => {
        unsafe {
            if !QUIET {
                eprintln!($fmt, $($arg)*);
            }
        }
    };

    ($fmt:expr) => {
        unsafe {
            if !QUIET {
                eprintln!($fmt);
            }
        }
    }
}

fn relative_to_home<P: AsRef<Path>>(path: P) -> PathBuf {
    let home = PathBuf::from(env::var("HOME").expect("Cannot find HOME directory"));
    home.join(path.as_ref())
}

fn src_command<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("src")
        .about("Download, build and update halide source")
        .arg(
            Arg::with_name("make")
                .short("m")
                .long("make")
                .default_value("make")
                .help("Make executable"),
        )
        .arg(
            Arg::with_name("source")
                .long("url")
                .default_value("https://github.com/halide/halide")
                .help("Halide respository"),
        )
        .arg(
            Arg::with_name("branch")
                .long("branch")
                .default_value("master")
                .help("Halide source branch"),
        )
}

fn build_command<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("build")
        .about("Build Halide source files")
        .arg(
            Arg::with_name("cxx")
                .long("cxx")
                .env("CXX")
                .default_value("c++")
                .help("Set c++ compiler"),
        )
        .arg(
            Arg::with_name("cxxflags")
                .env("CXXFLAGS")
                .long("cxxflags")
                .help("Set c++ compile flags"),
        )
        .arg(
            Arg::with_name("ldflags")
                .env("LDFLAGS")
                .long("ldflags")
                .help("Set c++ link flags"),
        )
        .arg(
            Arg::with_name("name")
                .required(true)
                .help("Output executable name"),
        )
        .arg(
            Arg::with_name("input")
                .multiple(true)
                .required(true)
                .help("Input files"),
        )
        .arg(
            Arg::with_name("args")
                .multiple(true)
                .raw(true)
                .takes_value(true)
                .help("Arguments to executable"),
        )
        .arg(
            Arg::with_name("generator")
                .long("generator")
                .short("g")
                .help("Link with GenGen.cpp"),
        )
        .arg(
            Arg::with_name("shared")
                .long("shared")
                .takes_value(true)
                .help("Compile shared library"),
        )
}

fn run_command<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("run")
        .about("Build and run Halide source files")
        .arg(
            Arg::with_name("cxx")
                .long("cxx")
                .env("CXX")
                .default_value("c++")
                .help("Set c++ compiler"),
        )
        .arg(
            Arg::with_name("cxxflags")
                .env("CXXFLAGS")
                .long("cxxflags")
                .help("Set c++ compile flags"),
        )
        .arg(
            Arg::with_name("ldflags")
                .env("LDFLAGS")
                .long("ldflags")
                .help("Set c++ link flags"),
        )
        .arg(
            Arg::with_name("keep")
                .long("keep")
                .short("k")
                .help("Keep generated executables"),
        )
        .arg(
            Arg::with_name("generator")
                .long("generator")
                .short("g")
                .help("Link with GenGen.cpp"),
        )
        .arg(
            Arg::with_name("input")
                .multiple(true)
                .required(true)
                .help("Input files"),
        )
        .arg(
            Arg::with_name("args")
                .multiple(true)
                .raw(true)
                .takes_value(true)
                .help("Arguments to executable"),
        )
        .arg(
            Arg::with_name("shared")
                .long("shared")
                .takes_value(true)
                .help("Compile shared library"),
        )
}

fn new_command<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("new")
        .about("Create new Halide genertor")
        .arg(Arg::with_name("path").required(true))
}

fn main() {
    let default_halide_path = relative_to_home("halide");
    let mut app = App::new("halide")
        .version("0.1")
        .author("Zach Shipko <zachshipko@gmail.com>")
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .help("Disable logging to stdout/stderr"),
        )
        .arg(
            Arg::with_name("halide-path")
                .short("p")
                .env("HALIDE_PATH")
                .default_value(default_halide_path.to_str().expect("Invalid path"))
                .help("Path to Halide directory"),
        )
        .subcommand(src_command())
        .subcommand(build_command())
        .subcommand(run_command())
        .subcommand(new_command());

    let mut help = Vec::new();
    let _ = app.write_long_help(&mut help);
    let matches = app.get_matches();

    unsafe {
        QUIET = matches.is_present("quiet");
    }

    let halide_path = Path::new(
        matches
            .value_of("halide-path")
            .expect("Invalid HALIDE_PATH"),
    );

    if let Some(src) = matches.subcommand_matches("src") {
        let source = Source {
            halide_path: halide_path.to_owned(),
            repo: src
                .value_of("source")
                .expect("Invalid source repository")
                .to_string(),
            branch: src
                .value_of("branch")
                .expect("Invalid branch name")
                .to_string(),
            make: src
                .value_of("make")
                .expect("Invalid make executable")
                .to_string(),
            make_flags: src
                .values_of("make-flags")
                .unwrap_or(clap::Values::default())
                .map(|s| s.to_string())
                .collect(),
        };

        if halide_path.exists() {
            log!(
                "Updating Halide source in {}",
                halide_path.to_string_lossy()
            );
            if !source.update().expect("Error updating git repository") {
                log!("Failed to update git repository");
                exit(1)
            }
        } else {
            log!(
                "Downloading Halide source to {}",
                halide_path.to_string_lossy()
            );
            if !source.download().expect("Error downloading git repository") {
                log!("Failed to clone git repository");
                exit(1)
            }
        }

        if !source.build().expect("Error building Halide source") {
            log!("Halide build failed");
            exit(1)
        } else {
            log!(
                "Halide built successfully in {}",
                halide_path.to_string_lossy()
            );
        }
    } else if let Some(b) = matches.subcommand_matches("build") {
        let build = Build {
            halide_path: halide_path.to_owned(),
            cxx: b.value_of("cxx"),
            cxxflags: b.value_of("cxxflags"),
            ldflags: b.value_of("ldflags"),
            output: PathBuf::from(b.value_of("name").expect("Invalid output path")),
            src: b
                .values_of("input")
                .expect("Invalid input files")
                .map(|x| PathBuf::from(x))
                .collect(),
            keep: true,
            build_args: b
                .values_of("args")
                .unwrap_or(clap::Values::default())
                .collect(),
            run_args: vec![],
            generator: b.is_present("generator"),
        };

        log!("Compiling {:?} to {:?}", build.src, build.output);
        if !build
            .build()
            .expect(format!("Error building {:?}", build.output).as_str())
        {
            log!("Unable to build {:?}", build.output);
            exit(1)
        }

        if let Some(x) = b.value_of("shared") {
            let f = std::path::PathBuf::from(x);
            let f =
                f.with_file_name(String::from("lib") + f.file_name().unwrap().to_str().unwrap());
            let f = f.with_extension("so");

            log!("Building shared library: {} -> {}", x, f.display());
            compile_shared_library(b.value_of("cxx"), f.to_str().unwrap(), &[x])
                .expect("Unable to compile shared library");
        }
    } else if let Some(b) = matches.subcommand_matches("run") {
        let start = SystemTime::now();
        let ts = start.duration_since(UNIX_EPOCH).unwrap();
        let ms = ts.as_secs() * 1000 + ts.subsec_nanos() as u64 / 1000000;
        let build = Build {
            halide_path: halide_path.to_owned(),
            cxx: b.value_of("cxx"),
            cxxflags: b.value_of("cxxflags"),
            ldflags: b.value_of("ldflags"),
            output: PathBuf::from(format!("./halide-{}", ms)),
            src: b
                .values_of("input")
                .expect("Invalid input files")
                .map(|x| PathBuf::from(x))
                .collect(),
            keep: b.is_present("keep"),
            run_args: b
                .values_of("args")
                .unwrap_or(clap::Values::default())
                .collect(),
            build_args: vec![],
            generator: b.is_present("generator"),
        };

        let output = build.output.to_owned();

        log!("Compiling {:?} to {:?}", build.src, output);
        if !build
            .build()
            .expect(format!("Error building {:?}", build.src).as_str())
        {
            log!("Failure building {:?}", build.src);
            exit(1)
        }

        log!("Running {:?}", build.output);
        if !build
            .run()
            .expect(format!("Error running {:?}", build.output).as_str())
        {
            log!("Failure while running {:?}", build.output);
            exit(1)
        }

        if let Some(x) = b.value_of("shared") {
            let f = std::path::PathBuf::from(x);
            let f =
                f.with_file_name(String::from("lib") + f.file_name().unwrap().to_str().unwrap());
            let f = f.with_extension("so");

            log!("Building shared library: {} -> {}", x, f.display());
            compile_shared_library(b.value_of("cxx"), f.to_str().unwrap(), &[x])
                .expect("Unable to compile shared library");
        }
    } else if let Some(b) = matches.subcommand_matches("new") {
        let dest = b.value_of("path").unwrap();
        let mut f = std::fs::File::create(dest).expect("Unable to open output file");
        let s = "
#include <Halide.h>
using namespace Halide;

class Filter: public Generator<Filter> {
public:
    Var x, y, c;
    Input<Buffer<float>> input{\"input\", 3};
    Output<Buffer<float>> output{\"output\", 3};
    void generate(){

    }

    void schedule(){

    }
};

HALIDE_REGISTER_GENERATOR(Filter, filter);";
        if let Err(e) = f.write(s.as_bytes()) {
            log!("Unable to write new file: {:?}", e);
        }
    } else {
        eprintln!("{}", String::from_utf8_lossy(help.as_ref()));
    }
}
