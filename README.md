# halide-build

A build tool for [Halide](https://github.com/halide/halide) filters.

It can be used from within Rust code or from the command-line.

## CLI

To build the command-line interface the `bin` feature must be activated:

```shell
$ cargo build --features=bin
```

## Build

To build a kernel from Rust `build.rs`:

```rust
// Create the build context
let build = Build::new(halide_path, output_path);

// Add your source files
build.src.push("mykernel.cpp");

// Build
if build.build()? {
  // Run
  assert!(build.run()?);

  // Link the resulting library
  link_library("./libmykernel.a")
}

```
