#[macro_use]
extern crate lazy_static;

extern crate flate2;
extern crate tar;
extern crate lzma;
extern crate bzip2;
extern crate unrar;
extern crate base64;
extern crate regex;
extern crate rand;

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

use std::iter;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

lazy_static! {
    static ref path_digi_suffix: Regex = Regex::new(r"_\((\d+)\)$").unwrap(); // to match a directory name: this_is_a_directory_(123)
}

#[derive(Debug)]
enum WrappedTypes {
    Zip, Xz, Tar, Gzip, Bzip, Rar, Base64
}

fn main() {
    use WrappedTypes::*;
    fn unwrap<P: AsRef<Path>>(wrapped: &WrappedTypes, file_path: P, unwrap_to: P) -> io::Result<()> {
        match wrapped {
            Zip => unwrap_zip(file_path, unwrap_to),
            Xz => unwrap_xz(file_path, unwrap_to),
            Tar => unwrap_tar(file_path, unwrap_to),
            Gzip => unwrap_gzip(file_path, unwrap_to),
            Bzip => unwrap_bzip(file_path, unwrap_to),
            Rar => unwrap_rar(file_path, unwrap_to),
            Base64 => unwrap_base64(file_path.as_ref().to_str().unwrap()),
        }
    }

    let mut unwrapper = HashMap::new();
    unwrapper.insert("application/zip", Zip);
    unwrapper.insert("application/x-xz", Xz);
    unwrapper.insert("application/x-tar", Tar);
    unwrapper.insert("application/gzip", Gzip);
    unwrapper.insert("application/x-bzip", Bzip);
    unwrapper.insert("application/vnd.rar", Rar);
    unwrapper.insert("application/x-rar-compressed", Rar);

    let args = env::args();
    if args.len() == 1 { // todo: showing a help msg
        println!("missing argument");
        process::exit(1);
    }

    for file in args.skip(1) { // todo: check filename encoding?
        let file_path = Path::new(&file); // todo: check whether it's a regular file, not a directory.
        if !file_path.exists() { // assume to be base64 string, it's better to check content are all valid b64 characters.
            // unwrap_base64(&file).unwrap();
            unwrap(&Base64, file_path, file_path).unwrap();
            continue
        }

        // let ref wrapped_type = tree_magic::from_filepath(file_path);
        if let Some(wrapped_type) = unwrapper.get(&tree_magic::from_filepath(file_path) as &str) {
            let unwrap_to = &file_path.file_stem().unwrap().to_string_lossy(); // todo: check whether it's empty string
            match create_dir3(unwrap_to) { // running create_dir("abc") might return Ok("abc_(1)"), because "abc/" already exists.
                Ok(unwrap_to) => {
                    let ff = unwrap_to.as_ref();
                    let pth = Path::new(ff);
                if let Err(e) = unwrap(wrapped_type,file_path, pth) {
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
            println!("not supported type: {}", 123);
            continue;
        }
    }
}

// try to parse entry name in the correct encoding.
fn unwrap_zip<P: AsRef<Path>, Q: AsRef<Path>>(file_path: P, unwrap_to: Q) -> io::Result<()> {
    println!("unwrapping zip");
    let file = fs::File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(file).unwrap(); // todo: check return value. If error, .....

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = unwrap_to.as_ref().join(file.sanitized_name());

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
fn unwrap_7z<P>(file_path: P, unwrap_to: P) -> io::Result<()>
    where P: AsRef<Path>
{
    println!("unwrapping 7z: {:?}", _file_path);
    Ok(())
}
*/

// called inside unwrap_xz/bzip/gzip() if decompressed file is a tar
fn untar<P: AsRef<Path>, Q: AsRef<Path>>(file_path: P, unwrap_to: Q) -> io::Result<()> {
    if let "application/x-tar" = tree_magic::from_filepath(file_path.as_ref()).as_ref() {
        println!("tar found");
        let mut rng = thread_rng();
        let random_filename: String = iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(8)
            .collect();
        let new_file_path = file_path.as_ref().parent().unwrap().to_path_buf().join(random_filename);
        fs::rename(&file_path, &new_file_path)?;
        println!("renaming {:?} to {:?}", file_path.as_ref(), &new_file_path);
        unwrap_tar(&new_file_path, unwrap_to)?;
        fs::remove_file(&new_file_path)?;
    }
    Ok(())
}

fn unwrap_xz<P: AsRef<Path>, Q: AsRef<Path>>(file_path: P, unwrap_to: Q) -> io::Result<()> {
    println!("unwrapping xz: {:?}", file_path.as_ref());
    let file = fs::File::open(file_path.as_ref())?;
    let mut f = LzmaReader::new_decompressor(file).unwrap();

    let output_file_name = file_path.as_ref().file_stem().unwrap();
    let output_file_path = unwrap_to.as_ref().join(output_file_name);
    let mut output_file = fs::File::create(&output_file_path)?;
    io::copy(&mut f, &mut output_file)?;

    if let "application/x-tar" = tree_magic::from_filepath(&output_file_path).as_ref() {
        println!("tar found");
        return untar(output_file_path, unwrap_to)
    }
    Ok(())
}

fn unwrap_bzip<P: AsRef<Path>, Q: AsRef<Path>>(file_path: P, unwrap_to: Q) -> io::Result<()> {
    println!("unwrapping bzip");
    let file = fs::File::open(file_path.as_ref())?;

    println!("path stem: {:?}", file_path.as_ref().file_stem().unwrap());
    let output_file_name = file_path.as_ref().file_stem().unwrap(); // todo: check output validity
    let output_file_path = unwrap_to.as_ref().join(&output_file_name);
    let mut output_file = fs::File::create(&output_file_path)?;

    let mut f = BzDecoder::new(file);
    io::copy(&mut f, &mut output_file)?;

    if let "application/x-tar" = tree_magic::from_filepath(&output_file_path).as_ref() {
        println!("tar found");
        return untar(output_file_path, unwrap_to)
    }
    Ok(())
}

fn unwrap_gzip<P: AsRef<Path>, Q: AsRef<Path>>(file_path: P, unwrap_to: Q) -> io::Result<()> {
    println!("unwrapping bzip");
    let file = fs::File::open(file_path.as_ref())?;

    println!("path stem: {:?}", file_path.as_ref().file_stem().unwrap());
    let output_file_name = file_path.as_ref().file_stem().unwrap(); // todo: check output validity
    let output_file_path = unwrap_to.as_ref().join(&output_file_name);
    let mut output_file = fs::File::create(&output_file_path)?;

    let mut f = GzDecoder::new(file);
    io::copy(&mut f, &mut output_file)?;

    if let "application/x-tar" = tree_magic::from_filepath(&output_file_path).as_ref() {
        println!("tar found");
        return untar(output_file_path, unwrap_to)
    }
    Ok(())
}

fn unwrap_tar<P: AsRef<Path>, Q: AsRef<Path>>(file_path: P, unwrap_to: Q) -> io::Result<()> {
    println!("unwrapping tar: {:?} file into: {:?}", file_path.as_ref(), unwrap_to.as_ref());
    let file = fs::File::open(file_path.as_ref())?;
    let mut archive = Archive::new(file);
    archive.unpack(unwrap_to)
}

fn unwrap_rar<P: AsRef<Path>, Q: AsRef<Path>>(file_path: P, unwrap_to: Q) -> io::Result<()> {
    unrar::Archive::new(file_path.as_ref().to_str().unwrap().to_owned())
        .extract_to(unwrap_to.as_ref().to_str().unwrap().to_owned())
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

/*
// failed attempt
fn create_dir2<'a>(path: &'a str) -> io::Result<Cow<'a, str>> {
    if let Ok(_) = fs::create_dir(path) {
        return Ok(path.into())
    }
    // let mut new_path = path.to_string() + "_(1)";
    let mut new_path: Cow<str>= Cow::Owned(path.to_string() + "_(1)");
    while let Err(e) = fs::create_dir(&new_path.as_ref::<>()) {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            let digit_suffix = path_digi_suffix.captures(&new_path).unwrap();
            let suffix = digit_suffix.get(1).unwrap().as_str().parse::<i32>().unwrap() + 1;
            let new_suffix = format!("_({})", suffix);
            new_path = path_digi_suffix.replace_all(&new_path, &new_suffix[..]).; // old Cow get dropped, new Cow get allocated, still causing heap allocation :-(
            continue
        } else {
            return Err(e)
        }
    }
    Ok(new_path.into())
}
*/

fn create_dir3(path: &str) -> io::Result<Cow<str>> {
    if let Ok(_) = fs::create_dir(path) {
        return Ok(path.into())
    }
    let mut new_path = path.to_string() + "_(1)";
    // let mut new_path: Cow<str>= Cow::Owned(path.to_string() + "_(1)");
    while let Err(e) = fs::create_dir(&new_path) {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            let tmp;
            {
                let digit_suffix = path_digi_suffix.captures(&new_path).unwrap(); // this capture always match, new_path definitely ends with "_(\d+)".
                let suffix = digit_suffix.get(1).unwrap().as_str().parse::<i32>().unwrap() + 1; // todo: cautious of overflow(consider using wrap-around)
                let new_suffix = format!("_({})", suffix);
                tmp = path_digi_suffix.replace_all(&new_path, &new_suffix[..]).into_owned();
            }
            new_path = tmp;
            continue
        } else {
            return Err(e)
        }
    }
    Ok(new_path.into()) // same as Ok(Cow::Owned(new_path))
}