# Third-party notices

This project builds with third-party Rust crates. Each crate is distributed
under its own license. The full license text and copyright notices for every
crate are available in the corresponding crate archive from crates.io and in
the linked source repository. The versions used by this project are pinned in
`Cargo.lock`.

## Direct dependencies

| Crate | Version | License | Source |
| --- | --- | --- | --- |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | https://github.com/dtolnay/anyhow |
| crossterm | 0.29.0 | MIT | https://github.com/crossterm-rs/crossterm |
| dirs | 7.0.0 | MIT OR Apache-2.0 | https://codeberg.org/dirs/dirs-rs |
| mpris-server | 0.11.0 | MPL-2.0 | https://github.com/SeaDve/mpris-server |
| rand | 0.10.3 | MIT OR Apache-2.0 | https://github.com/rust-random/rand |
| ratatui | 0.30.2 | MIT | https://github.com/ratatui/ratatui |
| reqwest | 0.13.5 | MIT OR Apache-2.0 | https://github.com/seanmonstar/reqwest |
| rodio | 0.23.0 | MIT OR Apache-2.0 | https://github.com/RustAudio/rodio |
| serde | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| symphonia | 0.6.1 | MPL-2.0 | https://github.com/pdeljanov/Symphonia |
| tokio | 1.54.1 | MIT | https://github.com/tokio-rs/tokio |
| toml | 1.1.8+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |

## Mozilla Public License 2.0

Several direct and transitive dependencies are licensed under the Mozilla
Public License 2.0, including the `mpris-server`, `option-ext`, and `symphonia`
crate families. The full text of MPL-2.0 is included in
`LICENSES/MPL-2.0.txt`. The source code for these components is available at
their repositories and in their crates.io packages. The exact versions used by
this project are pinned in `Cargo.lock`.

Where a dependency offers a choice of licenses (for example, `MIT OR
Apache-2.0`), this project relies on the permissive option. License texts and
copyright notices for all transitive dependencies are distributed with the
corresponding crates.

## LGPL-3.0-only components

Parts of this project are based on
[yandex-music-api](https://github.com/MarshalX/yandex-music-api), which is
licensed under LGPL-3.0-only. The full license text is in `LICENSE`, and the
upstream copyright notice is reproduced in `NOTICE`.
