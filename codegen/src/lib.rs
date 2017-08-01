extern crate includedir_codegen;
extern crate walkdir;

use std::io::Write;
use std::path::Path;
use std::error::Error;
use std::fs::OpenOptions;

/// Do codegen to write a file (`codegen_fname`) which includes
/// the contents of all entries in `files_dir`.
#[cfg(feature = "bundle_files")]
fn create_codegen_file<P, Q>(files_dir: P, codegen_fname: Q) -> Result<(), std::io::Error>
    where P: AsRef<Path>,
          Q: AsRef<Path>
{
    // Collect list of files to include
    let entries = walkdir::WalkDir::new(files_dir.as_ref())
        .into_iter()
        .map(|entry| entry.expect("DirEntry error").path().into())
        .collect::<Vec<std::path::PathBuf>>();

    // Make sure we recompile if these files change
    println!("cargo:rerun-if-changed={}", files_dir.as_ref().display());
    for entry in entries.iter() {
        println!("cargo:rerun-if-changed={}", entry.display());
    }

    // Check that at least one of the needed files is there.
    let required: std::path::PathBuf = files_dir.as_ref().join("index.html");
    if !entries.contains(&required) {
        return Err(std::io::Error::new(std::io::ErrorKind::Other,
                                       format!("no {:?} file (hint: run make in elm_frontend)",
                                               required)));
    }

    let codegen_fname_str = format!("{}", codegen_fname.as_ref().display());
    // Write the contents of the files.
    includedir_codegen::start("PUBLIC")
        .dir(files_dir, includedir_codegen::Compression::Gzip)
        .build(&codegen_fname_str)?;
    Ok(())
}

/// Create an empty file (`codegen_fname`).
#[cfg(feature = "serve_files")]
fn create_codegen_file<P, Q>(_: P, codegen_fname: Q) -> Result<(), Box<Error>>
    where P: AsRef<Path>,
          Q: AsRef<Path>
{
    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = std::path::Path::new(&out_dir).join(codegen_fname);
    std::fs::File::create(dest_path)?;
    Ok(())
}

/// Update the codegen file (`codegen_fname`) to include
/// the `Config`.
fn include_config<P, Q>(files_dir: P, codegen_fname: Q) -> Result<(), Box<Error>>
    where P: AsRef<Path>,
          Q: AsRef<Path>
{
    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = std::path::Path::new(&out_dir).join(codegen_fname);

    let mut f = OpenOptions::new().append(true).open(dest_path)?;

    writeln!(f, "use bui_backend::lowlevel::Config;")?;
    writeln!(f, "fn get_default_config() -> Config {{")?;
    writeln!(f, "    Config {{")?;
    writeln!(f,
             "        serve_filepath: \"{}/\",",
             files_dir.as_ref().display())?;
    #[cfg(feature = "bundle_files")]
    {
        writeln!(f, "        bundled_files: &PUBLIC,")?;

    }
    writeln!(f, "        channel_size: 10,")?;
    writeln!(f, "        cookie_name: \"client\".into(),")?;
    writeln!(f, "    }}")?;
    writeln!(f, "}}")?;

    Ok(())
}

/// Write a file `generated_path` which would should include
/// with the `include!` directive into your rust source code.
/// This should be called from the `build.rs` script in your crate.
/// This will define the variable `BUI_BACKEND_CONFIG` which should
/// be passed to `bui_backend::BuiBackend::new()` to configure it
/// correctly. See the `demo_js` and `demo_elm` for example usage.
pub fn codegen<P, Q>(files_dir: P, generated_path: Q) -> Result<(), Box<Error>>
    where P: AsRef<Path>,
          Q: AsRef<Path>
{
    create_codegen_file(&files_dir, &generated_path)?;
    include_config(&files_dir, &generated_path)
}
