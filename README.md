# cargo-testjs
Cargo extension to run tests by nodejs

## Install

```shell
$ cargo install cargo-testjs
```

## Run

```shell
$ cargo testjs
```

## Config

You can write configs in Cargo.toml.

```toml:Cargo.toml
[package.metadata.testjs]
node = "nodejs"
target = "asmjs-unknown-emscripten"
prelude = "tests/test.js"
```

### node

An absolute path to the nodejs. The default value is `node`

### target

The JS target to be built. The default value is `asmjs-unknown-emscripten`

### prelude (Optional)

A JS file to load before the test file.
