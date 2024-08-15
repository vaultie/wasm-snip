use anyhow::Context;
use std::fs;
use std::io::{self, Write};

fn main() -> Result<(), anyhow::Error> {
    let matches = parse_args();

    let mut opts = wasm_snip::Options::default();

    opts.functions = matches
        .values_of("function")
        .map(|fs| fs.map(|f| f.to_string()).collect())
        .unwrap_or(vec![]);

    opts.patterns = matches
        .values_of("pattern")
        .map(|ps| ps.map(|p| p.to_string()).collect())
        .unwrap_or(vec![]);

    if matches.is_present("snip_rust_fmt_code") {
        opts.snip_rust_fmt_code = true;
    }

    if matches.is_present("snip_rust_panicking_code") {
        opts.snip_rust_panicking_code = true;
    }

    if matches.is_present("skip_producers_section") {
        opts.skip_producers_section = true;
    }

    let config = walrus_config_from_options(&opts);
    let path = matches.value_of("input").unwrap();
    let buf = fs::read(&path).with_context(|| format!("failed to read file {}", path))?;
    let mut module = config.parse(&buf)?;

    wasm_snip::snip(&mut module, opts).context("failed to snip functions from wasm module")?;

    if let Some(output) = matches.value_of("output") {
        module
            .emit_wasm_file(output)
            .with_context(|| format!("failed to emit snipped wasm to {}", output))?;
    } else {
        let wasm = module.emit_wasm();
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        stdout
            .write_all(&wasm)
            .context("failed to write wasm to stdout")?;
    }

    Ok(())
}

fn walrus_config_from_options(options: &wasm_snip::Options) -> walrus::ModuleConfig {
    let mut config = walrus::ModuleConfig::new();
    config.generate_producers_section(!options.skip_producers_section);
    config
}

fn parse_args() -> clap::ArgMatches<'static> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .long_about(
            "
`wasm-snip` replaces a WebAssembly function's body with an `unreachable`.

Maybe you know that some function will never be called at runtime, but the
compiler can't prove that at compile time? Snip it! Then run `wasm-gc`[0] again
and all the functions it transitively called (which could also never be called
at runtime) will get removed too.

Very helpful when shrinking the size of WebAssembly binaries!

[0]: https://github.com/alexcrichton/wasm-gc
",
        )
        .arg(
            clap::Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .help("The path to write the output wasm file to. Defaults to stdout."),
        )
        .arg(
            clap::Arg::with_name("input")
                .required(true)
                .help("The input wasm file containing the function(s) to snip."),
        )
        .arg(clap::Arg::with_name("function").multiple(true).help(
            "The specific function(s) to snip. These must match \
             exactly. Use the -p flag for fuzzy matching.",
        ))
        .arg(
            clap::Arg::with_name("pattern")
                .required(false)
                .multiple(true)
                .short("p")
                .long("pattern")
                .takes_value(true)
                .help("Snip any function that matches the given regular expression."),
        )
        .arg(
            clap::Arg::with_name("snip_rust_fmt_code")
                .required(false)
                .long("snip-rust-fmt-code")
                .help("Snip Rust's `std::fmt` and `core::fmt` code."),
        )
        .arg(
            clap::Arg::with_name("snip_rust_panicking_code")
                .required(false)
                .long("snip-rust-panicking-code")
                .help("Snip Rust's `std::panicking` and `core::panicking` code."),
        )
        .arg(
            clap::Arg::with_name("skip_producers_section")
                .required(false)
                .long("skip-producers-section")
                .help("Do not emit the 'producers' custom section."),
        )
        .get_matches()
}
