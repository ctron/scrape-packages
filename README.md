Create a list of crates:

```bash
cargo tree --manifest-path=../trunk/Cargo.toml --no-default-features --features native-tls --target x86_64-unknown-linux-gnu --prefix none | awk '{ print $1" "$2 }' | sort -u > input
```

Then run it through this app:

```bash
cargo run -- < input
```
