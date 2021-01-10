# 🦉 yair

[![Actions Status](https://github.com/sheredom/yair/workflows/Rust/badge.svg)](https://github.com/sheredom/yair/actions)

**Y**et **A**nother **I**ntermediate **R**epresentation (pronounced Ya! IR) is a compiler intermediate representation written entirely in Rust. Key design decisions make the representation unique:

- No Φ (phi) nodes, basic blocks take arguments instead. (Similar in some way to the Swift Intermediate Language approach[1](References-1)).
- Target agnostic representation for de-coupling of components.
- Strong seperation between library components (you don't need to build, link, or use components you don't need).



## References

### References 1  [https://llvm.org/devmtg/2015-10/#talk7](Swift's High-Level IR: A Case Study of Complementing LLVM IR with Language-Specific Optimization).