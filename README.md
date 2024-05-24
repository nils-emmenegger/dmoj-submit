# dmoj-submit

A CLI tool for submitting to DMOJ.

![Using dmoj-submit](https://github.com/nils-emmenegger/dmoj-submit/blob/images/vhs-tapes/submit.gif?raw=true)

## Installation

You can download the latest official version on our [releases page](https://github.com/nils-emmenegger/dmoj-submit/releases).

You can also install directly from GitHub. First make sure you have the Rust toolchain installed. If not, you can do so through [rustup](https://rustup.rs/). Then, run the following command:

```sh
$ cargo install --git https://github.com/nils-emmenegger/dmoj-submit
```

## Usage

Before being able to use dmoj-submit you must have a DMOJ API token. You can get yours [on the profile edit page on DMOJ](https://dmoj.ca/edit/profile/).

Below are dmoj-submit's subcommands:

### submit

This is the main operation and raison d'être for dmoj-submit. This operation allows you to submit to DMOJ from your command line.

### set-config

This operation allows you to configure dmoj-submit to remember your API token and language preferences. Previously set defaults may be overwritten by running the command again.

### get-config

This prints the path to the TOML file that your configuration is stored at, as well as the configuration itself.

### list-languages

dmoj-submit lists languages supported by DMOJ in alphabetical order. This helps determine what language key corresponds to your desired language.

### help

To see all subcommands and options, you can use the `help` subcommand, e.g. `dmoj-submit help` or `dmoj-submit help set-config`.

### Examples

The following is an example of how you might use dmoj-submit. Lines prepended with `$` are user input. All other lines are program output. Additionally, MY_API_TOKEN should be replaced with your DMOJ API token.

```
$ dmoj-submit set-config --token MY_API_TOKEN
$ dmoj-submit set-config --language py:py3,java:java8 -vv
[2023-05-23T19:43:02Z INFO  dmoj_submit] Set extension py to language key py3
[2023-05-23T19:43:02Z INFO  dmoj_submit] Set extension java to language key java8
$ dmoj-submit submit helloworld.py
Case  #1: AC [0.027s, 10.01MB]
Result: AC
Resources: 0.027s, 10.01MB
Final score: 100/100
$ dmoj-submit submit --problem helloworld --language text --token MY_API_TOKEN my_file
Case  #1: AC [0.005s, 1.64MB]
Result: AC
Resources: 0.005s, 1.64MB
Final score: 100/100
```

## Additional Information

### How dmoj-submit determines problem and language when they are not explicitly specified

Problem code is determined by file stem (e.g. `helloworld` for `helloworld.py`).

Language can be determined by two methods, checked in this order:

1. Configuration is checked for a file extension -> language key mapping. This can be set using `dmoj-submit set-config --language ...`. For example, when `helloworld.py` is submitted in the example in the [Usage](#usage) section, there already exists a mapping for `py:py3`, so the file extension `py` is mapped to the language key `py3` (Python 3).
2. Hard-coded defaults defined by `EXT_KEY_DEFAULT_TUPLES` in `src/main.rs`. They are as follows:

| File extension | Language key |
| -------------- | ------------ |
| c              | c            |
| cpp            | cpp20        |
| java           | java         |
| kt             | kotlin       |
| py             | pypy3        |
| lua            | lua          |
| rs             | rust         |
| txt            | text         |
| go             | go           |
| hs             | hask         |
| js             | v8js         |
| nim            | nim          |
| ml             | ocaml        |
| zig            | zig          |

### Verbosity (e.g. `-vv`)

dmoj-submit uses [clap-verbosity-flag](https://github.com/clap-rs/clap-verbosity-flag) to add support for verbosity. You can add up to 4 `--verbose` or `-v` flags to get more logging output.
