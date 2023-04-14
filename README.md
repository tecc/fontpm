# FontPM: the font package manager

FontPM is a package manager-like command line utility that allows you to install fonts swiftly, similar to how many package managers allow you to install packages.

> **NOTES**
> 
> - FontPM is quite rudimentary in its feature set. If you'd like to see features added, 
>   [open an issue](https://github.com/tecc/fontpm).
>
> - Right now, it only supports downloading **non-variable fonts** from [Google Fonts](https://fonts.google.com).
>   If you find any other sources that you think FontPM should have, [open an issue at the repository](https://github.com/tecc/fontpm)!


## Installation

### Step 1: Install FontPM

#### Cargo (the recommended way)

FontPM can be installed through cargo simply by running `cargo install fontpm`.
This is the recommended way of doing things.

#### Building and installing from source (the slightly more complex way)

FontPM can also be installed by building it from source. 
At the moment, there isn't a stable branch, so you'll just have to figure out which commit you want to build from. No guarantees are made about the stability of the dev branch.

The first step is to clone the repository using Git: `git clone https://github.com/tecc/fontpm.git`. 
After that's done, you can install FontPM by running `cargo install --path <path-to-fontpm>/cli` (where `<path-to-fontpm>` is whichever directory FontPM was closed into).

A full script for building and installing from source looks something like this: 
```bash
git clone https://github.com/tecc/fontpm.git
cd fontpm
cargo install --path ./cli 
```

### Step 2: Refresh the font index

The first thing you need to do before using FontPM is refreshing the local font index. 
You can do this using `fontpm refresh`. 


## Usage

### Refreshing the local font index

FontPM downloads indices of available fonts for each source, but it doesn't do this automatically for you. To refresh 

```bash
fontpm refresh
```

### Installing a font to your machine

> **WARNING**
> 
> This may not work on all platforms. It should work with any XDG-compatible Linux system, but beyond that it's untested.

To install a font to your machine, simply run the following command:

```bash
fontpm install <font-id...>
```

Substitute `<font-id>` with whatever font you want to install. This is usually in `kebab-case`, so if you want to install [Noto Sans](https://fonts.google.com/noto/specimen/Noto+Sans) you'd write `noto-sans`.

To install multiple fonts, simply list font IDs separated by spaces.

### Installing a font to a directory (e.g. for a project)

Installing fonts to a specific directory is done similarly to installing them globally.
Simply run the following command, substituting `<font-id>` for whatever fonts you want to use and `<directory>` for whatever directory you want to install the font

```bash
fontpm install -d <directory> <font-id...>
```

Additionally, you can specify _how_ you'd like FontPM to organise the files by using the `-f` (or `--format`) flag.
It only supports 2 modes as of now - `flat` and `flat-directory`.

```bash
fontpm install -d <directory> -f <format> <font-id...>
```

If you use `flat`, the files will look a bit like this:
```
<directory>
|_ font1-italic.ttf
|_ font1-regular.ttf
|_ font2-italic.ttf
|_ font2-regular.ttf
```

`flat-directory` separates them into directories based on the font ID, meaning it looks a bit like this:
```
<directory>
|_ font1
|  |_ italic.ttf
|  |_ regular.ttf
|_ font2
|  |_ italic.ttf
|  |_ regular.ttf 
```


## Configuration

### Location

FontPM's configuration (`config.toml`) resides in a platform-specific directory:
- On Linux it will be in `$XDG_CONFIG_HOME/fontpm` which normally is (and defaults to if `$XDG_CONFIG_HOME` is not set) `$HOME/.config/fontpm` (example: `/home/alice/.config/fontpm`)
- On Windows it will be in the Roaming AppData directory (example: `C:\Users\Alice\AppData\Roaming`).
- On Mac it will be `$HOME/Library/Application Support` (example: `/Users/Alice/Library/Application Support`).

### Explanation

```toml
[fontpm] # The main section of the configuration
# enabled_sources: array<string>
#   A list of source IDs.
#   All the sources included in this array, FontPM will use at runtime.
enabled_sources = ["google-fonts"]

# cache_dir: path
#   Path to the directory where FontPM should cache files.
#   This directory will contain the local index files and all downloaded font files.
#   If this is not provided, it will create a default at runtime.
cache_dir = "~/.cache/fontpm"

# font_install_dir: path
#   Path to the directory where installed fonts should reside.
#   If this is not provided, it will create a default at runtime.
font_install_dir = "~/.local/share/fonts/fontpm"
```

## Licence

FontPM is licensed under the [Apache 2.0 License](http://www.apache.org/licenses/LICENSE-2.0). You can find the text [in LICENCE](./LICENCE).


    Copyright (c) 2023 tecc
    
    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at
    
           http://www.apache.org/licenses/LICENSE-2.0
    
    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.