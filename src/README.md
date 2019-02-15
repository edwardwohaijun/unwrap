# Introduction
A simple wrapper of common decompressors for zip, rar, xz, tar, tgz, bzip, base64(more are coming). 

# Build & Install
If you haven't installed `Rust` yet: 
```bash
curl https://sh.rustup.rs -sSf | sh
```

For Windows, go to [https://www.rust-lang.org/install.html](https://www.rust-lang.org/install.html)

Check for a successful install
```bashe
rustc --version 
```
And then:
```bash
git clone https://github.com/edwardwohaijun/unwrap
cd unwrap
cargo build
```
# Usage
```bash
./target/debug/unwrap file1.zip file2.rar file3.tar file4.tgz file5.bz2 file6.rar QXdlc29tZSBSdXN0
```
Each file will be decompressed into a directory with the same name with extension stripped.
