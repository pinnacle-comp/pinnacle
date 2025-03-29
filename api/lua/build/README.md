# Lua gRPC Codegen Tool

This crate generates a Lua file containing methods and definitions for
interfacing with a gRPC API.

## Requirements
Requires the following Lua libraries:
- `lua-protobuf`
- `lua-cqueues`
- `lua-http`

Optionally requires:
- `lua-compat53`, for compatibility with Lua 5.2 and below.

## Usage

Run `cargo run -- <path-to-protobufs-directory>` to print the generated
code to stdout. Pipe it to a file to save it.
