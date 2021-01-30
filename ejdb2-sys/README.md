# ejdb2-sys
Native bindings for [EJDB 2.0](https://github.com/Softmotions/ejdb)


## Usage

```toml
[dependencies]
ejdb2-sys={git="http://github.com/wowin/ejdb2-rs.git", path="ejdb2-sys"}
```

## Build

works on windows (requires msys2) & non-windows platform.

```
EJDB2_INSTALL_PATH=EJDB2_PREBUILD_FILES_BY_CMAKE_INSTALL
EJDB2_SOURCE=EJDB2_SOURCE_CODE
```

if ENV `EJDB2_INSTALL_PATH` was specified, will build with pre-build ejdb2 binaries.

if ENV `EJDB2_SOURCE` was specified, will build ejdb2 from source.

### Build for Non-Windows platform
To build the library, you need to have cmake installed along with gcc and clang.
And specify one of two ENV variables: `EJDB2_INSTALL_PATH`  and `EJDB2_SOURCE`.

###  Windows platform

steps:
- install msys2&mingw64
please refer to official document for how to get it work.

- install toolchain for msys2
```sh
pacman -S --needed base-devel git \
      mingw-w64-x86_64-toolchain \
      mingw-w64-x86_64-cmake
```

- configure ENV
set ENV variable `MSYS_HOME` to make it work.
```
MSYS_HOME=MSYS_INSTALL_FOLDER
```

And specify one of two ENV variables: `EJDB2_INSTALL_PATH`  and `EJDB2_SOURCE`.

If you choose to build from source, please configure ENV `EJDB2_SOURCE`. Then build your project. Note: the build script will patch and modify`cmake/modules/AddIOWOW.cmake`, change `-DBUILD_SHARED_LIBS=OFF` to `-DBUILD_SHARED_LIBS=${BUILD_SHARED_LIBS}`.

### Static build & dynamic build

The build script will infer static build or dynamic build in the following orders:
 - ENV `EJDB2_STATIC`
 - ENV `EJDB2_DYNAMIC`
 - rustc flags: +crt-static


## License

MIT
