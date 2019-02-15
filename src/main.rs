#[macro_use]
extern crate lazy_static;

extern crate flate2;
extern crate tar;
extern crate lzma;
extern crate bzip2;
extern crate unrar;
extern crate base64;
extern crate regex;

use flate2::read::GzDecoder;
use tar::Archive;
use lzma::LzmaReader;
use bzip2::read::{BzDecoder};

use base64::{decode};
use regex::Regex;

use std::str;
use std::env;
use std::path::Path;
use std::collections::HashMap;
use std::io;
use std::fs;
use std::process;
use std::borrow::Cow;

lazy_static! {
    static ref path_digi_suffix: Regex = Regex::new(r"_\((\d+)\)$").unwrap(); // to match a directory name: this_is_a_directory_(123)
}

fn main() {
    let mut unwrapper: HashMap<&str, Box<dyn Fn(&Path, &Path) -> io::Result<()>>> = HashMap::new(); // todo: use with_capacity(16);
    unwrapper.insert("application/zip", Box::new(unwrap_zip));
    //unwrapper.insert("application/x-7z-compressed", Box::new(unwrap_7z));
    unwrapper.insert("application/x-xz", Box::new(unwrap_xz));
    unwrapper.insert("application/x-tar", Box::new(unwrap_tar));
    unwrapper.insert("application/gzip", Box::new(unwrap_tar));
    unwrapper.insert("application/x-bzip", Box::new(unwrap_bzip));
    unwrapper.insert("application/vnd.rar", Box::new(unwrap_rar));
    unwrapper.insert("application/x-rar-compressed", Box::new(unwrap_rar));

    let args = env::args();
    if args.len() == 1 { // todo: showing a help msg
        println!("missing argument");
        process::exit(1);
    }

    for file in args.skip(1) { // todo: check filename encoding?
        let file_path = Path::new(&file); // todo: check whether it's a regular file, not directory.
        if !file_path.exists() { // assume to be base64 string, it's better to check content are all valid b64 characters.
            if let Err(_e) = unwrap_base64(&file) {
                continue;
            }
        }

        let wrapped_type: &str = &tree_magic::from_filepath(file_path);
        if let Some(unwrap) = unwrapper.get(wrapped_type) {
            let unwrap_to = &file_path.file_stem().unwrap().to_string_lossy(); // todo: check whether it's empty string
            match create_dir(unwrap_to) { // running create_dir("abc") might return Ok("abc_(1)"), because "abc/" already exists.
                Ok(unwrap_to) => {
                    if let Err(e) = unwrap(file_path, std::path::Path::new(&unwrap_to)) {
                        println!("err unwrapping {:?}: {:?}", file_path, e);
                        continue;
                    }
                },
                Err(e) => {
                    println!("failed to create a directory before unwrapping into: {:?}", e);
                    continue;
                }
            }
        } else { // check whether the passed is a regular file, not directory, symLink
            println!("not supported type: {}", wrapped_type);
            continue;
        }
    }
}

fn unwrap_zip(file_path: &Path, unwrap_to: &Path) -> io::Result<()> {
    println!("unwrapping zip");
    let file = fs::File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(file).unwrap(); // todo: check return value. If error, .....

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = unwrap_to.join(file.sanitized_name());

        if (&*file.name()).ends_with('/') {
            println!("File {} extracted to \"{}\"", i, outpath.as_path().display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            println!("File {} extracted to \"{}\" ({} bytes)", i, outpath.as_path().display(), file.size());
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }

        // Get and Set permissions
        #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
                }
            }
    }

    Ok(())
}

// not implemented yet
/*
fn unwrap_7z(_file_path: &Path, _unwrap_to: &Path) -> io::Result<()> {
    println!("unwrapping 7z: {:?}", _file_path);
    Ok(())
}
*/

fn unwrap_xz(file_path: &Path, unwrap_to: &Path) -> io::Result<()> {
    println!("unwrapping xz: {:?}", file_path);
    let file = fs::File::open(file_path)?;
    let mut f = LzmaReader::new_decompressor(file).unwrap();

    let output_file_name = file_path.file_stem().unwrap();
    let output_file_path = std::path::Path::new(unwrap_to).join(output_file_name);
    let mut output_file = fs::File::create(output_file_path)?;
    io::copy(&mut f, &mut output_file)?;

    // todo: check whether the unwrapped is of type tar.

    Ok(())
}

fn unwrap_bzip(file_path: &Path, unwrap_to: &Path) -> io::Result<()> {
    println!("unwrapping bzip");
    let file = fs::File::open(file_path)?;
    let mut f = BzDecoder::new(file);

    println!("path stem: {:?}", file_path.file_stem().unwrap());
    let output_file_name = file_path.file_stem().unwrap(); // todo: check output validity
    let output_file_path = std::path::Path::new(unwrap_to).join(&output_file_name);
    let mut output_file = fs::File::create(output_file_path)?;
    io::copy(&mut f, &mut output_file)?;

    // todo: check unzipped format, if tar, do untar

    Ok(())
}

fn unwrap_tar(file_path: &Path, unwrap_to: &Path) -> io::Result<()> {
    println!("unwrapping tar");
    let file = fs::File::open(file_path)?;
    let gz;
    // let mut archive;
    if &tree_magic::from_filepath(file_path) == "application/gzip" {
        gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);
        archive.unpack(unwrap_to)
    } else {
        let mut archive = Archive::new(file);
        archive.unpack(unwrap_to)
    }
}

fn unwrap_rar(file_path: &Path, unwrap_to: &Path) -> io::Result<()> {
    unrar::Archive::new(file_path.to_str().unwrap().into())
        .extract_to(unwrap_to.to_str().unwrap().into())
        .unwrap()
        .process()
        .unwrap();

    Ok(())
}

fn unwrap_base64(input: &str) -> io::Result<()> {
    match decode(input) {
        Ok(decoded) => println!("decoded base64: {:?}", str::from_utf8(&decoded).unwrap()),
        Err(e) => println!("err decoding base64: {:?}", e), // todo, now what????? how to convert this error into Result<()>
    }
    Ok(())
}

// todo: check whether path is an empty string(it's possible after stripping the extension part)
// use 'while' instead of recursive call.
fn create_dir(path: &str) -> io::Result<String> { // after moving this fn into a module, make this one private
    let mut new_path = Cow::Borrowed(path); // replace() of Regex capture return a Cow<'a, str>
    let res = fs::create_dir(path);
    match res {
        Ok(_) => Ok(path.to_string()),
        Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if let Some(digi_suffix) = path_digi_suffix.captures(path) {
                let suffix = digi_suffix.get(1).unwrap().as_str().parse::<i32>().unwrap() + 1; // todo: cautious of overflow(consider using wrap-around)
                let new_suffix = format!("_({})", suffix);
                new_path = path_digi_suffix.replace_all(&path, &new_suffix[..]);
            } else {
                new_path += "_(1)";
            }
            create_dir(&new_path)
        },
        Err(e) => Err(e)
    }
}