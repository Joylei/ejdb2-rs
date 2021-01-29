# ejdb2-sys
ejdb2 rust binding


## how to use
```toml
[dependencies]
ejdb2-sys={git="http://github.com/wowin/ejdb2-rs.git", path="ejdb2-rs-sys"}
```

## how to build

works on windows (requires msys2) & non-windows platform.

```
EJDB2_INSTALL_PATH=EJDB2_PREBUILD_FILES_BY_CMAKE_INSTALL
EJDB2_SOURCE=EJDB2_SOURCE_CODE
```

if ENV `EJDB2_INSTALL_PATH` was specified, will build with pre-build ejdb2 binaries.
if ENV `EJDB2_SOURCE` was specified, will build ejdb2 from source.

### Non-Windows

 Nothing to special, just specify one of two ENV variables: `EJDB2_INSTALL_PATH`  and `EJDB2_SOURCE`.

###  Windows

steps:
- install msys2&mingw64
please refer to official document for how to get it work.

- install toolchain for msys2
```sh
pacman -S --needed base-devel git \
      mingw-w64-x86_64-toolchain \
      mingw-w64-x86_64-cmake
```

- config ENV
need ENV variable `MSYS_HOME` to get it work.
```
MSYS_HOME=MSYS_INSTALL_FOLDER
```

---

if you choose to use pre-build files , please config ENV variable `EJDB2_INSTALL_PATH`.  Nothing else to do, just go ahead to build your project.

If you choose to build from source, please config ENV `EJDB2_SOURCE`. Then build your project. Note: the build script will patch and modify`cmake/modules/AddIOWOW.cmake`, change `-DBUILD_SHARED_LIBS=OFF` to `-DBUILD_SHARED_LIBS=${BUILD_SHARED_LIBS}`.

### static build or dynamic build
The build script will infer static build or dynamic build in the following orders:
 - ENV `EJDB2_STATIC`
 - ENV `EJDB2_DYNAMIC`
 - rustc flags: +crt-static


## License
MIT
