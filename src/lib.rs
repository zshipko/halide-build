use std::env;
use std::fs::remove_file;
use std::io;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub struct Build<'a> {
    pub halide_path: PathBuf,
    pub src: Vec<PathBuf>,
    pub output: PathBuf,
    pub cxx: Option<&'a str>,
    pub cxxflags: Option<&'a str>,
    pub ldflags: Option<&'a str>,
    pub build_args: Vec<&'a str>,
    pub run_args: Vec<&'a str>,
    pub keep: bool,
    pub generator: bool,
}

impl<'a> Build<'a> {
    pub fn new<P: AsRef<std::path::Path>, Q: AsRef<std::path::PathBuf>>(
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

    pub fn build(&self) -> io::Result<bool> {
        let cxx_default = env::var("CXX").unwrap_or("c++".to_string());
        let mut cmd = Command::new(self.cxx.clone().unwrap_or(cxx_default.as_str()));

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
            cmd.args(&self.build_args);
        }

        cmd.args(&self.src)
            .args(&["-o", &self.output.to_string_lossy()])
            .args(&[
                "-L",
                &self.halide_path.join("lib").to_string_lossy(),
                "-lHalide",
                "-lpng",
                "-ljpeg",
                "-lpthread",
                "-ltinfo",
                "-ldl",
                "-lz",
            ]);

        if let Some(flags) = &self.ldflags {
            cmd.args(flags.split(" "));
        }

        cmd.status().map(|status| status.success())
    }

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

pub fn run<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    halide_path: P,
    path: Q,
    run_args: Vec<&str>,
    generator: bool,
) -> io::Result<bool> {
    let build = Build {
        halide_path: halide_path.as_ref().to_path_buf(),
        src: vec![path.as_ref().to_path_buf()],
        output: path.as_ref().to_path_buf().with_extension("exe"),
        cxx: None,
        cxxflags: None,
        ldflags: None,
        keep: false,
        generator,
        run_args,
        build_args: vec![],
    };

    build.build()?;
    build.run()
}

pub struct Source {
    pub halide_path: PathBuf,
    pub repo: String,
    pub branch: String,
    pub make: String,
    pub make_flags: Vec<String>,
}

impl Source {
    pub fn download(&self) -> io::Result<bool> {
        Command::new("git")
            .arg("clone")
            .args(&["-b", self.branch.as_str()])
            .arg(&self.repo)
            .arg(&self.halide_path)
            .status()
            .map(|status| status.success())
    }

    pub fn update(&self) -> io::Result<bool> {
        Command::new("git")
            .current_dir(&self.halide_path)
            .arg("pull")
            .arg("origin")
            .arg(&self.branch)
            .status()
            .map(|status| status.success())
    }

    pub fn build(&self) -> io::Result<bool> {
        Command::new(&self.make)
            .current_dir(&self.halide_path)
            .args(&self.make_flags)
            .status()
            .map(|status| status.success())
    }
}
