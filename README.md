# typstlab

English | [日本語](README_JA.md)

`typstlab` is a toolchain manager for Typst projects.

## Why I Built This

Typst itself and its surrounding tools are excellent.
However, there were not many tools that could handle Typst version compatibility while also managing docs, builds, and MCP as one coherent toolchain.

It is possible to solve parts of this with skills or Makefiles.
Even then, the result can be slow, difficult to manage in git, or hard to call truly reproducible.

In modern programming language ecosystems, formatter, LSP, tests, builds, and docs are expected to work together naturally inside a project.
For Typst and its surrounding tools, it can be difficult to keep checking which tool version is compatible with which Typst version.
That makes it harder to fully benefit from the excellent tools that already exist.

`typstlab` is being built to solve that problem.
The most direct reason is simple: I wanted a stable Typst toolchain for writing school reports.

The goal is to manage the Typst version, Typst documentation, and the project-local paper/template/dist structure together, while exposing the same operations from both the CLI and MCP.

This project is still under development.
At the moment, Typst command execution, documentation download and conversion, and MCP-based operations are available.

## Relationship With External Projects

`typstlab` uses artifacts from Typst and Typst docs projects.
It does not reimplement those projects.

The main external artifacts currently used are:

- Typst binary: release binaries from [`typst/typst`](https://github.com/typst/typst)
- Typst docs: docs release artifacts from [`typst-community/dev-builds`](https://github.com/typst-community/dev-builds)

`typstlab` does not modify those artifacts directly.
It handles version resolution, download, caching, project-local syncing, and MCP access around them.

## Current Status

- [x] Typst version resolution
- [x] Typst binary download and cache
- [x] Typst command execution
- [x] PDF / PNG / SVG / HTML builds
- [x] Typst docs download
- [x] Typst docs Markdown conversion
- [x] MCP server
- [ ] fmt
- [ ] LSP
- [ ] test

## Installation

### Development Version

`typstlab` is not published to crates.io yet.
Until then, clone it from GitHub.

```bash
git clone https://github.com/SotaTne/typstlab.git
cd typstlab
cargo install --path crates/typstlab-cli
```

Using SSH:

```bash
git clone git@github.com:SotaTne/typstlab.git
cd typstlab
cargo install --path crates/typstlab-cli
```

After installation, the `typstlab` command is available.

```bash
typstlab --help
```

To try it without installing, use `cargo run --` from this repository.

```bash
cargo run -- --help
```

## First Steps

Create a new Typst project.

```bash
typstlab new my-project
cd my-project
```

Check the project status.

```bash
typstlab status
```

Build papers.

```bash
typstlab build
```

You can also specify output formats.

```bash
typstlab build --pdf
typstlab build --png
typstlab build --svg
typstlab build --html
```

If no format is specified, `typstlab build` outputs PDF.

## typstlab.toml

A `typstlab` project contains `typstlab.toml`.

```toml
[project]
name = "my-project"
init_date = "2026-05-04"

[toolchain]
typst = "0.14.2"
typst_docs = "auto"
typstyle = "none"

[structure]
papers_dir = "papers"
dist_dir = "dist"
templates_dir = "templates"
```

## Toolchain Configuration

`typst` must be an explicit version.

```toml
[toolchain]
typst = "0.14.2"
```

`typst_docs` and future companion tools can be configured with one of three choices.

```toml
typst_docs = "auto"
typst_docs = "none"
typst_docs = "0.14.2"
```

- `auto`: use the newest compatible version
- `none`: do not use that tool
- `"0.14.2"`: use the specified version

If an incompatible version is specified, `typstlab` returns an error.

## status

`status` shows which toolchain the current project is actually using.

```bash
typstlab status
```

It shows:

- project name
- project root
- Typst version
- Typst binary path
- docs version
- docs path
- docs cache
- papers
- templates
- dist

Example:

```text
typstlab status

Project
  name     my-project
  root     /path/to/my-project

Toolchain
  typst
    version  0.14.2
    binary   /path/to/cache/typst/0.14.2/typst
  docs
    version  0.14.2
    path     /path/to/my-project/.typstlab/typst_docs
    cache    /path/to/cache/docs
```

## build

`build` compiles papers under the `papers` directory with Typst.

```bash
typstlab build
```

You can build a specific paper.

```bash
typstlab build paper-id
```

Multiple papers can be specified.

```bash
typstlab build paper-a paper-b
```

Output formats:

```bash
typstlab build --pdf
typstlab build --png
typstlab build --svg
typstlab build --html
```

## Papers and Templates

Create a paper.

```bash
typstlab gen paper intro
```

Create a template.

```bash
typstlab gen template report
```

Create a paper from a template.

```bash
typstlab gen paper intro --template report
```

If no local template is found, `typstlab` falls back to Typst template initialization.

## Typst Docs

When `typst_docs = "auto"` is configured, `typstlab` resolves docs compatible with the configured Typst version.

Docs are stored in the global cache first, then synced into the project as `.typstlab/typst_docs`.

```text
Global cache
└── typstlab/
    └── docs/
        └── 0.14.2/
            └── ...
                 ↓ sync
Project
└── .typstlab/
    └── typst_docs/
        └── ...
```

- First run: download docs and store them in the cache
- Later runs: sync from cache into the project
- Other projects: reuse the same docs cache

This lets each project refer to resolved local docs while still sharing the cache across projects.

## MCP

The MCP server can run over stdio.

```bash
typstlab mcp stdio .
```

Without installing:

```bash
cargo run -- mcp stdio .
```

Current MCP capabilities:

- get project status
- build a paper and return PNG images

This allows editors and agents to inspect `typstlab` project status and trigger builds.

Pass the project root as an argument from your MCP client.

```json
{
  "mcpServers": {
    "typstlab": {
      "command": "typstlab",
      "args": ["mcp", "stdio", "/path/to/project"]
    }
  }
}
```

## Troubleshooting

### I want to see which cache is being used

`status` shows both the Typst binary path and docs cache path.

```bash
typstlab status
```

If the docs look stale or broken, first check the `cache` path shown by `status`.

### I want to resync docs

Docs are synced from the cache into `.typstlab/typst_docs` inside the project.
To rebuild only the project-local synced docs, remove `.typstlab/typst_docs` and run `status` again.

```bash
typstlab status
```

## What This Tool Aims To Become

Typst is powerful on its own, but real projects still need answers to questions like:

- which Typst version should build this project
- how docs versions should be aligned
- how papers and templates should be managed
- how agents and editors should build safely
- how fmt, LSP, and tests should eventually be integrated

`typstlab` is the layer that treats these as one project toolchain.

It is not complete yet.
The first focus is stabilizing Typst command execution, docs, and MCP.
After that, fmt, LSP, and test support will be added.

