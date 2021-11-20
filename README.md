# 🦜 yair

[![Actions Status](https://github.com/sheredom/yair/workflows/Rust/badge.svg)](https://github.com/sheredom/yair/actions)
[![Crates.io](https://img.shields.io/crates/v/yair.svg)](https://crates.io/crates/yair)
[![API Docs](https://docs.rs/mio/badge.svg)](https://docs.rs/yair)

**Y**et **A**nother **I**ntermediate **R**epresentation (pronounced Yarrrr! (like a pirate!)) is a compiler intermediate representation written entirely in Rust. Key design decisions make the representation unique:

- Single Static Assignment representation [\[1\]](#References-1).
- No Φ (phi) nodes, basic blocks take arguments instead [\[2\]](#References-2).
- Target agnostic representation for de-coupling of components.
- Strong seperation between library components (you don't need to build, link, or use components you don't need).

## TODOs

- Core:
  - Add per-domain functions and function multi-versioning.
- Verifier:
  - When we have per-domain functions (CPU-only for instance) check for:
    - Recursion.
    - Calling a function in a conflicting domain (call GPU from CPU).
  - Maybe restrict variables to non-any non-gpu?
    - At the least we should have some form of thread_local (shared) variables, and cpu globals too. But any else doesn't really make sense I think?
  - Check for casts to the same type as the value.
  - Check for pointers in invalid domains being inside pointers of other domains (like stack pointer being stored into CPU memory).
  - Check that blocks have correct terminating instructions (ret/br/etc).
- Add a cranelift code generation library.
- Add an optimizer!
- Explain the syntax of the IR:
  - Globals and structs both use `%name` to differentiate them from other symbols.
- Verify that all statements in a block are reachable from the tree of blocks above.
- Verify that a block doesn't have any instructions after a terminator.

## Features

The following features are present in the yair crate.

### io

The 'io' feature is a **default** feature of the yair crate. It lets you consume and produce binary or textual intermediate representation files from yair. This allows for inspection, testing, and serialization to the intermediate representation.

When this feature is enabled, two additional binaries are produced alongside the library crate - `yair-as` and `yair-dis`, allowing for assembling and disassembling of the intermediate representation between the human readable textual form, and the binary form.

Additionally, there is a `yair::io` module that lets users read/write the textual or binary representation into a yair `Library` that they can work with.

#### .yail Files

The human readable representation of yair are .yail files. An example file is:

```
mod "😀" {
  fn foo(a : i64, b : i64) : i64 {
    bar(a : i64, b : i64):
      r = or a, b
      ret r
  }
}
```

Constants in .yail files are slightly strange - constants as used in the `Library` object are unique per the value and type combination for that given constant. But in the intermediate representation, constants are treated like any other value within the body of a basic block:

```
mod "😀" {
  fn foo(a : i64) : i64 {
    bar(a : i64):
      b = const i64 4
      r = or a, b
      ret r
  }
}
```

This means that constants behave like regular SSA notes for the purposes of the intermediate representation.

## References

### References 1

[Static single assignment form](https://en.wikipedia.org/wiki/Static_single_assignment_form).

### References 2

This approach is similar in some ways to the Swift Intermediate Language approach - [Swift's High-Level IR: A Case Study of Complementing LLVM IR with Language-Specific Optimization.](https://llvm.org/devmtg/2015-10/#talk7)
