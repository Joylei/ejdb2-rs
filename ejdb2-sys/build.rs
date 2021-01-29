extern crate anyhow;
extern crate bindgen;
extern crate cmake;
extern crate pkg_config;

use anyhow::{anyhow, Context, Result};
use cmake::Config;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use std::{fmt::write, process};
use std::{ops::Index, str};

fn main() -> Result<()> {
    let is_static = false; // flag for dev

    let vars = [
        "MSYS_HOME",
        "EJDB2_SOURCE",
        "EJDB2_DYNAMIC",
        "EJDB2_STATIC",
        "EJDB2_INSTALL_PATH",
    ];
    {
        let is_static = is_static || check_static();
        config_and_build(is_static)
    }
    .map(|_| {
        for x in vars.iter() {
            println!("cargo:rerun-if-env-changed={}", x);
        }
    })
    .map_err(|e| {
        eprintln!("========ENV VARS======");
        for x in vars.iter() {
            eprintln!("{}={}", x, env::var(x).ok().unwrap_or_default());
        }
        e
    })
}

fn config_and_build(is_static: bool) -> Result<()> {
    eprintln!("is static: {}", is_static);
    #[cfg(windows)]
    {
        msys::check_msys(is_static)?;
    }
    let install_dir = if let Ok(install_dir) = env::var("EJDB2_INSTALL_PATH") {
        eprintln!("use pre-build ejdb2: {}", install_dir);
        PathBuf::from(install_dir)
    } else {
        eprintln!("build ejdb2 from source");
        let source_dir = env::var("EJDB2_SOURCE").unwrap();
        build_source(source_dir, is_static)?
    };
    link_libs(&install_dir, is_static)?;
    gen_binding(&install_dir)?;
    Ok(())
}

fn link_libs(dst: &PathBuf, is_static: bool) -> Result<()> {
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("bin").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("lib").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("lib64").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("build").join("lib").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("build").join("lib64").display()
    );

    if is_static {
        println!("cargo:rustc-link-lib=static=ejdb2-2");
        println!("cargo:rustc-link-lib=static=iowow-1");
        if cfg!(windows) {
            println!("cargo:rustc-link-lib=static=iberty"); //xstrndup
            println!("cargo:rustc-link-lib=static=winpthread");
            println!("cargo:rustc-link-lib=static=mingwex");
            println!("cargo:rustc-link-lib=static=mingw32");
            println!("cargo:rustc-link-lib=static=msvcrt");
            println!("cargo:rustc-link-lib=static=gcc");
        } else {
            //println!("cargo:rustc-link-lib=static=pthread");
            pkg_config::Config::new().probe("zlib")?;
            pkg_config::Config::new().probe("pthread")?;
        }
    } else {
        println!("cargo:rustc-link-lib=libejdb2");
        println!("cargo:rustc-link-lib=libiowow");

        #[cfg(windows)]
        install_dlls(&dst).with_context(|| "failed to copy dlls")?;
    }
    Ok(())
}

fn install_dlls(out_dir: &PathBuf) -> Result<()> {
    let target_dir = get_profile_folder(out_dir)?.to_path_buf();
    //copy dll to profile folder
    fs::copy(
        out_dir.join("bin\\libejdb2.dll"),
        target_dir.join("libejdb2.dll"),
    )?;
    fs::copy(
        out_dir.join("build\\bin\\libiowow.dll"),
        target_dir.join("libiowow.dll"),
    )?;
    Ok(())
}

fn get_profile_folder(out_dir: &PathBuf) -> Result<&Path> {
    let level = 3;
    let mut p = out_dir.as_path();
    for _ in 0..level {
        if let Some(v) = p.parent() {
            p = v;
        } else {
            return Err(anyhow!("failed to get cargo profile folder"));
        }
    }
    Ok(p)
}

fn build_source(dir: impl AsRef<str>, is_static: bool) -> Result<PathBuf> {
    eprintln!("is_static: {}", is_static);
    eprintln!("build EJDB2 from source: {}", dir.as_ref());

    #[cfg(windows)]
    {
        //need to patch source code before build
        let target_file = PathBuf::from(dir.as_ref()).join("cmake/modules/AddIOWOW.cmake");
        let code = fs::read_to_string(&target_file)?;
        if code.contains("-DBUILD_SHARED_LIBS=OFF") {
            eprintln!("apply patch for {}", target_file.display());
            let code_patched = code.replace(
                "-DBUILD_SHARED_LIBS=OFF",
                "-DBUILD_SHARED_LIBS=${BUILD_SHARED_LIBS}",
            );
            fs::write(target_file, code_patched)?;
        }
    }

    //debug or release
    let is_debug = get_env_bool("DEBUG").unwrap_or_default();

    //cmake build
    let mut conf = Config::new(dir.as_ref());
    conf.cflag("-w")
        .profile(if is_debug {
            "RelWithDebInfo"
        } else {
            "Release"
        })
        .define("BUILD_SHARED_LIBS", if is_static { "OFF" } else { "ON" })
        .define("BUILD_EXAMPLES", "OFF")
        .define("PACKAGE_TGZ", "OFF")
        .define("PACKAGE_ZIP", "OFF");

    if cfg!(windows) {
        // hack target here, cmake crate need this to work correctly,
        // please see https://docs.rs/cmake/0.1.45/src/cmake/lib.rs.html#469
        let target = env::var("TARGET")
            .unwrap()
            .replace("windows-msvc", "windows-gnu");
        //use custom toolchain
        let tc_file = env::current_dir()?.join("win64-tc.cmake");
        conf.generator("MSYS Makefiles")
            .target(&target)
            .define("ENABLE_HTTP", "OFF")
            .define("CMAKE_TOOLCHAIN_FILE", tc_file);
    }
    //conf.very_verbose(true);
    let dst = conf.build();

    #[cfg(windows)]
    {
        //fix header file, need this until iowow code gets modified
        patch_headers(&dst)?;
    }
    Ok(dst)
}

/// patch headers for windows
fn patch_headers(out_dir: &PathBuf) -> Result<()> {
    let iwp_file = out_dir.join("include\\ejdb2\\iowow\\iwp.h");
    eprintln!("apply patch for {}", iwp_file.display());
    let code = fs::read_to_string(&iwp_file)?;
    if !code.contains("<sys/types.h>") {
        let code_patched = code.replace(
            "#include <sys/time.h>",
            r#"#ifndef _WIN32
#include <sys/time.h>
#else
#include <sys/types.h>
#endif"#,
        );
        fs::write(iwp_file, code_patched)?;
    }

    Ok(())
}

fn gen_binding(dst: &PathBuf) -> Result<()> {
    let header_file = dst
        .join("include/ejdb2/ejdb2.h")
        .as_path()
        .to_str()
        .unwrap()
        .to_owned();

    println!("cargo:rerun-if-changed={}", header_file);

    let bindings = bindgen::Builder::default()
        .header(header_file)
        .clang_arg("-I".to_owned() + dst.join("include").as_path().to_str().unwrap())
        //.clang_arg("-IE:/msys64/usr/include")
        .enable_function_attribute_detection()
        .derive_default(true)
        .rustified_enum(".*")
        .whitelist_type("(EJDB|JBL|JBR|ejdb|jbl|jbp|jbr|re|iwkv)(_.*?)?")
        .whitelist_function("(ejdb|jbl|jbp|jbn|jql|jbr|lwre|iwxstr|iwlog)_.*")
        .opaque_type("_JBL_iterator")
        .rustfmt_bindings(true)
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .with_context(|| "Couldn't write bindings!")
}

/// check if static build in the order of:
/// EJDB2_STATIC, EJDB2_DYNAMIC, rustflags: +crt-static
fn check_static() -> bool {
    if let Some(v) = get_env_bool("EJDB2_STATIC") {
        return v;
    }
    if let Some(v) = get_env_bool("EJDB2_DYNAMIC") {
        return !v;
    }
    cfg!(target_feature = "crt-static")
}

fn get_env_bool(key: &str) -> Option<bool> {
    env::var(key)
        .ok()
        .map(|v| v == "1" || v.to_lowercase() == "true")
}

mod msys {
    use super::*;

    struct MingwEnv {
        pub msys_home: String,
        pub mingw64_home: String,
    }

    fn probe_mingw64() -> Result<MingwEnv> {
        let msys_home = env::var("MSYS_HOME")?;
        let mingw64_home = format!("{}\\mingw64", msys_home);
        if test_make().is_err() {
            let path = env::var("PATH").unwrap();
            env::set_var(
                "PATH",
                format!("{}\\bin;{}\\usr\\bin;{}", mingw64_home, msys_home, path),
            );
            test_make()?;
        }
        eprintln!("make found, MINGW64 is ready to use");
        Ok(MingwEnv {
            msys_home,
            mingw64_home,
        })
    }

    fn test_make() -> Result<bool> {
        eprintln!("test make...");
        let out = std::process::Command::new("make")
            .arg("--version")
            .output()?;
        if !out.status.success() {
            return Err(anyhow!(
                "Command failed: [make --version], status: {}",
                out.status
            ));
        }
        let msg: &str = str::from_utf8(&out.stdout)?;
        if msg.contains("GNU Make") {
            return Ok(true);
        }
        return Err(anyhow!("Command failed: [make --version]\r\n{}", msg));
    }

    pub(crate) fn check_msys(is_static: bool) -> Result<()> {
        let probe_res = probe_mingw64()?;
        let mingw64_path = PathBuf::from(&probe_res.mingw64_home);
        eprintln!("build ejdb2-sys for windows");
        let arch = "x86_64-w64-mingw32";

        let gcc_dir = find_gcc(&mingw64_path, arch)?;

        println!(
            "cargo:rustc-link-search=native={}",
            mingw64_path.join("lib").display()
        );

        //for iberty
        println!(
            "cargo:rustc-link-search=native={}",
            mingw64_path.join("lib").join("binutils").display()
        );

        println!(
            "cargo:rustc-link-search=native={}",
            mingw64_path.join(arch).join("lib").display()
        );
        println!("cargo:rustc-link-search=native={}", gcc_dir.display());

        Ok(())
    }
    /// find lib gcc path
    fn find_gcc(mingw64_path: &PathBuf, arch: &str) -> Result<PathBuf> {
        // ${mingw64_path}\\lib\\gcc\\x86_64-w64-mingw32
        let root = mingw64_path.join("lib").join("gcc").join(arch);
        let mut paths: Vec<(PathBuf, Version)> = fs::read_dir(root)?
            .filter_map(|x| {
                x.ok()
                    .map(|d| {
                        let p = d.path();
                        if p.is_dir() {
                            Some(p)
                        } else {
                            None
                        }
                    })
                    .flatten()
            })
            .filter_map(|p| {
                let name = p.file_name().map(|x| x.to_str()).flatten();
                name.map(|name| parse_version(name))
                    .flatten()
                    .map(|v| (p, v))
            })
            .collect();
        paths.sort_by_cached_key(|x| x.1);
        match paths.last() {
            Some(v) => Ok(v.0.to_owned()),
            None => Err(anyhow!("Failed to find gcc for mingw64")),
        }
    }

    fn parse_version(ver: &str) -> Option<Version> {
        let mut parts = ver.split(".");
        let major = parts.next().map(|x| x.parse().ok()).flatten()?;
        let minor = parts.next().map(|x| x.parse().ok()).flatten()?;
        let patch = parts.next().map(|x| x.parse().ok()).flatten()?;
        Some((major, minor, patch))
    }

    type Version = (usize, usize, usize);
}
