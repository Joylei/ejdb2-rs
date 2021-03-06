# ejdb2-rs

[EJDB2](https://github.com/Softmotions/ejdb) rust binding

## Usage

Add ejdb2-rs as dependency in your `Cargo.toml`

```toml
[dependencies]
ejdb2={git=https://github.com/Joylei/ejdb2-rs.git}
```
Please see test cases for usage details.

## Build

Please refer to [README.md](./ejdb2-sys/README.md#build) for details.

## no-std

Turn off default features:
```toml
[dependencies]
ejdb2={git=https://github.com/Joylei/ejdb2-rs.git, default-features = false}
```

or with `alloc` feature:
```toml
[dependencies]
ejdb2={git=https://github.com/Joylei/ejdb2-rs.git, default-features = false, features=["alloc"]}
```

## License

MIT
