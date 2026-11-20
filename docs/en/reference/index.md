# Reference

This section is for the reference of the commands, the JSON contracts and the error codes of
Wayloop. There is no release yet; what comes next is in the [roadmap](../project/roadmap.md).

## Commands of a build from source

The `wayloop` binary built from `crates/wayloop-cli` answers two commands. Both print one line
of JSON on standard output and exit with code 0. The version is the one of the workspace
manifest.

| Command | Output |
| --- | --- |
| `wayloop version` (also `wayloop --version`) | The name and the version of the engine |
| `wayloop steps` | The identifiers of the steps of the issue cycle, in order |

```console
$ cargo run -q -p wayloop-cli -- version
{"name":"wayloop","version":"0.0.0"}
$ cargo run -q -p wayloop-cli -- steps
{"issue_steps":["claim","reanalysis","counter_review","checklist","build","verify","review","pull_request","merge"]}
```

Any other command, or no command, prints a usage line on standard error and exits with code 2:

```console
$ cargo run -q -p wayloop-cli
wayloop 0.0.0: pre-alpha. Commands: version, steps.
```

When standard output cannot be written, for example because the reading end of a pipe is
closed, the command prints the cause on standard error and exits with code 3, an environment
error. The exit codes follow
[ADR 0009](../project/adr/0009-contract-guidelines.md) (C7).

The step identifiers are `snake_case` values of the contract, as every enum value of the JSON
contracts ([ADR 0009](../project/adr/0009-contract-guidelines.md), C4), and have the same
spelling in every channel. Other spellings, such as the labels of the issue tracker, are
produced by the adapters from these values. Adding a step is an additive change; renaming or
removing a step is a breaking change of the contract. The steps themselves are described in
[The loop](../concepts/the-loop.md#the-issue-cycle).
