//! halide-build is used to compile [Halide](https://github.com/halide/halide) kernels

use std::env;
use std::fs::remove_file;
use std::io;
use std::path::PathBuf;
use std::process::Command;

static CARGO_LINK_SEARCH: &'static str = "cargo:rustc-link-search=native=";
static CARGO_LINK_LIB: &'static str = "cargo:rustc-link-lib=";

/// Link a library, specified by path and name
pub fn link_lib(path: Option<&str>, name: &str) {
    if let Some(path) = path {
        println!("{}{}", CARGO_LINK_SEARCH, path);
    }

    println!("{}{}", CARGO_LINK_LIB, name);
}

/// Link a library, specified by filename
pub fn link<P: AsRef<std::path::Path>>(filename: P) {
    let mut filename = filename.as_ref().to_path_buf();
    let name = filename.file_stem().expect("Invalid filename");
    let s = String::from(name.to_str().expect("Invalid filename"));
    let mut tmp: &str = &s;

    if s.starts_with("lib") {
        tmp = &s[3..]
    }

    if s.ends_with(".a") {
        tmp = &tmp[..tmp.len() - 2];
    } else if s.ends_with(".so") {
        tmp = &tmp[..tmp.len() - 3];
    } else if s.ends_with(".dylib") {
        tmp = &tmp[..tmp.len() - 6];
    }

    filename.pop();
    link_lib(filename.to_str(), tmp);
}

/// Compile a shared library using the C++ compiler
pub fn compile_shared_library(
    compiler: Option<&str>,
    output: &str,
    args: &[&str],
) -> Result<bool, std::io::Error> {
    let cxx = std::env::var("CXX").unwrap_or("c++".to_owned());
    let mut cmd = Command::new(compiler.unwrap_or(&cxx));

    cmd.arg("-std=c++11");
    let res = cmd
        .arg("-shared")
        .arg("-o")
        .arg(output)
        .args(args)
        .status()?;
    Ok(res.success())
}

/// Build stores the required context for building a Halide kernel
#[derive(Debug)]
pub struct Build<'a> {
    /// Path to halide source
    pub halide_path: PathBuf,

    /// Input files
    pub src: Vec<PathBuf>,

    /// Output file
    pub output: PathBuf,

    /// C++ compiler
    pub cxx: Option<&'a str>,

    /// C++ compile time flags
    pub cxxflags: Option<&'a str>,

    /// C++ link time flags
    pub ldflags: Option<&'a str>,

    /// Extra arguments to build step
    pub build_args: Vec<&'a str>,

    /// Extra arguments to run step
    pub run_args: Vec<&'a str>,

    /// Keep executable when finished running
    pub keep: bool,

    /// Include Halide generator header
    pub generator: bool,
}

impl<'a> Build<'a> {
    /// Create a new build with the given halide path and output
    pub fn new<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
        halide_path: P,
        output: Q,
    ) -> Build<'a> {
        Build {
            halide_path: halide_path.as_ref().to_path_buf(),
            src: vec![],
            output: output.as_ref().to_path_buf(),
            cxx: None,
            cxxflags: None,
            ldflags: None,
            build_args: vec![],
            run_args: vec![],
            keep: false,
            generator: false,
        }
    }

    pub fn source_file(mut self, src: impl AsRef<std::path::Path>) -> Self {
        self.src.push(src.as_ref().to_owned());
        self
    }

    pub fn build_arg(mut self, src: &'a str) -> Self {
        self.build_args.push(src.as_ref());
        self
    }

    pub fn build_args(mut self, src: impl AsRef<[&'a str]>) -> Self {
        self.build_args.extend(src.as_ref());
        self
    }

    pub fn run_arg(mut self, src: &'a str) -> Self {
        self.run_args.push(src.as_ref());
        self
    }

    pub fn run_args(mut self, src: impl AsRef<[&'a str]>) -> Self {
        self.run_args.extend(src.as_ref());
        self
    }

    pub fn ldflags(mut self, flags: &'a str) -> Self {
        self.ldflags = Some(flags);
        self
    }

    pub fn cxxflags(mut self, flags: &'a str) -> Self {
        self.cxxflags = Some(flags);
        self
    }

    pub fn compiler(mut self, name: &'a str) -> Self {
        self.cxx = Some(name);
        self
    }

    pub fn keep(mut self, x: bool) -> Self {
        self.keep = x;
        self
    }

    pub fn generator(mut self, x: bool) -> Self {
        self.generator = x;
        self
    }

    /// Execute the build step
    pub fn build(&self) -> io::Result<bool> {
        let cxx_default = env::var("CXX").unwrap_or("c++".to_string());
        let mut cmd = Command::new(self.cxx.clone().unwrap_or(cxx_default.as_str()));

        cmd.arg("-std=c++11");
        cmd.args(&["-I", &self.halide_path.join("include").to_string_lossy()])
            .args(&["-I", &self.halide_path.join("tools").to_string_lossy()]);

        if let Some(flags) = &self.cxxflags {
            cmd.args(flags.split(" "));
        }

        if self.generator {
            cmd.arg(
                &self
                    .halide_path
                    .join("tools")
                    .join("GenGen.cpp")
                    .to_string_lossy()
                    .as_ref(),
            );
        }

        cmd.args(&self.build_args);

        let tinfo = std::env::var("TERMINFO").unwrap_or_else(|_| "-lncurses".to_string());

        cmd.args(&self.src)
            .args(&["-o", &self.output.to_string_lossy()])
            .args(&[
                "-L",
                &self.halide_path.join("lib").to_string_lossy(),
                "-lHalide",
                "-lpng",
                "-ljpeg",
                "-lpthread",
                &tinfo,
                "-ldl",
                "-lz",
            ]);

        if let Some(flags) = &self.ldflags {
            cmd.args(flags.split(" "));
        }

        cmd.status().map(|status| status.success())
    }

    /// Execute the run step
    pub fn run(&self) -> io::Result<bool> {
        if !self.output.exists() {
            return Ok(false);
        }

        let res = Command::new(&self.output)
            .args(&self.run_args)
            .env("LD_LIBRARY_PATH", self.halide_path.join("lib"))
            .status()
            .map(|status| status.success());

        if !self.keep {
            let _ = remove_file(&self.output);
        }

        res
    }
}

/// Source is used to maintain the Halide source directory
pub struct Source {
    pub halide_path: PathBuf,
    pub repo: String,
    pub branch: String,
    pub make: String,
    pub make_flags: Vec<String>,
}

impl Source {
    /// Download Halide source for the first time
    pub fn download(&self) -> io::Result<bool> {
        Command::new("git")
            .arg("clone")
            .args(&["-b", self.branch.as_str()])
            .arg(&self.repo)
            .arg(&self.halide_path)
            .status()
            .map(|status| status.success())
    }

    /// Update Halide source
    pub fn update(&self) -> io::Result<bool> {
        Command::new("git")
            .current_dir(&self.halide_path)
            .arg("pull")
            .arg("origin")
            .arg(&self.branch)
            .status()
            .map(|status| status.success())
    }

    /// Build Halide source
    pub fn build(&self) -> io::Result<bool> {
        Command::new(&self.make)
            .current_dir(&self.halide_path)
            .args(&self.make_flags)
            .status()
            .map(|status| status.success())
    }
}
