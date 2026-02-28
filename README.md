# FontPM: the font package manager

FontPM is a package manager-like command line utility that allows you to install fonts swiftly, similar to how most package managers allow you to install packages.

> [!WARNING]
> This software only supports Linux systems at the moment.
> Support for other platforms may be added in the future, pending me being able
> to test FontPM on said platforms.

## Installation

### Build from source

Building from source requires you to have a Rust installation available.

```sh
# Step 1: Clone the repository
git clone -b next https://github.com/tecc/fontpm
#         (and navigate to the newly cloned repository)
cd fontpm
# Step 3: Install FontPM
cargo install --locked --path . --bin fontpm
# and done!
```

### Through Cargo

> [!WARNING]
> The version of FontPM available on crates.io is not up to date.
> Only use if you want to suffer a horror in the form of executable code.

```sh
cargo install --locked fontpm
```

### Post-install notes

#### Refresh indices

When you've just installed FontPM, chances are you have not let FontPM download its internal indices.

The indices are necessary for FontPM to know what fonts are available for downloading (i.e. the main thing FontPM is able to do).

It is recommended that you run the following command after installing.

```sh
fontpm refresh
```

It is also recommended to run the above command now and then to ensure you can check all fonts.

## Usage

Please run `fontpm help` or `fontpm -h` for information on how to use the command-line interface.

## Licence

FontPM is licensed under the [Apache 2.0 License](http://www.apache.org/licenses/LICENSE-2.0). You can find the full licence text [in LICENCE](./LICENCE).

    Copyright (c) 2023-2026 tecc

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
