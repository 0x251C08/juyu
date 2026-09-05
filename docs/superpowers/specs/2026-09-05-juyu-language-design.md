# Juyu Language Architecture & Bootstrap Implementation Plan

**Date**: 2026-09-05  
**Topic**: Juyu Programming Language Architecture, Specification, and Compiler Bootstrap  
**Status**: Approved & Validated (via 120-question comprehensive design review)

---

## 1. Overview & Vision

Juyu is a modern, statically-typed, high-performance systems programming language designed for low-level hardware control, zero-cost abstractions, predictable latency, and maximum developer velocity.

Key differentiators:
1. **Tooling-First Velocity**: Ultra-fast compilation times with an orthogonal, small language surface and unified CLI tooling.
2. **Seamless C and C++ Interoperability**: Direct `@cImport("header.h")` via embedded Clang AST parsing with zero-cost bidirectional FFI and automatic C ABI alignment.
3. **First-Class Compile-Time Execution (`comptime`)**: Comptime execution via a sandboxed frontend bytecode VM, replacing textual macros with standard Juyu functions.
4. **Deterministic Explicit Memory Without Borrow-Checker Cognitive Load**: Explicit `std.mem.Allocator` passing, destructive move semantics for structs/arrays, linear resource tracking, and block-scoped `defer`/`errdefer`.

---

## 2. Comprehensive 120-Decision Summary Matrix

| Domain | Questions | Key Decisions |
|---|---|---|
| **1. Vision & Target Domain** | Q1–Q10 | Systems & high-performance applications; pure LLVM backend; Rust bootstrap compiler; ultimate self-hosting milestone; embedded Clang AST parser for `@cImport`; out-of-the-box cross-compilation for Tier 1 targets; strict declarative `juyu.toml` with optional executable `build.ju`. |
| **2. Compilation Architecture** | Q11–Q20 | Typed AST -> High-level SSA IR (JIR) -> LLVM IR; hand-written recursive descent + Pratt parser with resilient recovery; package-level top-level order independence; value-based + selective item imports; comptime functions returning types for generics; 4 build modes (`Debug`, `ReleaseSafe`, `ReleaseFast`, `ReleaseSmall`); integrated LLD linker; freestanding core with opt-in libc (`-lc`); `fn main(): !void`; built-in `test "name" { ... }`. |
| **3. Type System & Sizing** | Q21–Q35 | `let` (immutable) and `var` (mutable); standard fixed widths (`i8`..`i128`, `u8`..`u128`, `isize`, `usize`, `f32`, `f64`, `f128`, `bool`) plus custom arbitrary bit-widths (`iN`/`uN`, specifically `u1`, `u2`, `u4`, `u7`, `u24`); strict explicit casting (`@cast`); distinct pointers (`*T`, `*const T`, `[]T`, `[*]T`, `?*T`, `[*c]T`); zero-cost `?T` optionals; first-class error sets and error unions `!T`; `struct`, `extern struct`, and `packed struct`; `enum(T)` and `union(enum)`; fixed arrays `[N]T`, sentinel slices `[:0]T`, and SIMD `@Vector(N, T)`; UTF-8 slices `[]const u8` with null-terminated string literals `[:0]const u8`; explicit calling conventions; introspections (`@sizeOf`, `@bitSizeOf`, `@alignOf`, `@offsetOf`); `anytype` parameter deduction; transparent aliases and distinct newtypes. |
| **4. Memory & Resource Lifecycle** | Q36–Q45 | Polymorphic `std.mem.Allocator` (ptr + vtable); block-scoped `defer` and `errdefer`; comprehensive allocator suite (`PageAllocator`, `GPA` with leak detection, `ArenaAllocator`, `FixedBufferAllocator`, `c_allocator`); explicit init or `= undefined` with debug poisoning (`0xAA`); dual managed/unmanaged collections (`ArrayList` and `ArrayListUnmanaged`); spatial safety bounds checking in `Debug` and `ReleaseSafe`; temporal defense (stack escape analysis + GPA quarantine + `-fsanitize=address`); destructive move semantics for structs/arrays; scalar primitives copy automatically; explicit `.deinit()` with compile-time linear resource diagnostics. |
| **5. Syntax & Grammar** | Q46–Q55 | Mandatory semicolons `;`; arrow block returns `=> expr;`; colon return type `fn add(a: i32, b: i32): i32`; optional `: void`; single-expression function bodies `fn add(a: i32, b: i32): i32 => a + b;`; nestable `/* ... */`, `//`, Markdown doc-comments `///` and `//!`; `pub` prefix export modifier; triple-quote multiline strings `"""..."""` with indentation stripping; string interpolation `$"..."` desugaring to writer calls + printf format verification; `@""` escaped keyword identifiers. |
| **6. Control Flow & Expressions** | Q56–Q65 | `if`/`else` statements and expressions with optional parentheses and optional unwrap `if (let active = maybe)`; exhaustive `match` with arrow arms (`pattern => expr`); infinite `loop { ... }` and `while (cond) { ... }`; C-style three-part `for` and collection `for (item in slice)`; direct identifier loop labels (`outer: for (...) { break outer; }`); prefix `try` and infix `catch`; null-coalescing `??`; optional chaining `?.`; standard logical operators `&&`, `||`, `!`; standard bitwise operators `&`, `|`, `^`, `~`, `<<`, `>>` with shift safety. |
| **7. Functions & Closures** | Q66–Q75 | Swift-style overloading by named argument labels; mandatory callsite argument labels; unboxed closures with stack/struct captures (`\|a, b\| => a + b`); modifiers (`inline fn`, `noinline fn`, `export fn`, `extern fn`); LLVM compiler tail-call optimization; first-class tuples `(i32, f64)` with optional labels `(x: i32, y: i32)`; comptime tuple parameters (`args: anytype`) for variadics; `const fn` purity marker; `unreachable` keyword and `@likely`/`@unlikely` hints; `naked fn` for raw assembly routines. |
| **8. OOP, Structs & Traits** | Q76–Q85 | Struct methods + `extend Struct { ... }` blocks; explicit `static fn` keyword; structural `interface` satisfied implicitly (Go style); static dispatch by default, `*dyn Interface` for runtime dynamic dispatch; embedded anonymous struct fields with automatic member promotion; interface-based operator overloading (`add`, `sub`, `mul`, `index`); designated initializers `Type { .x = 10, .y = 20 }`; mandatory explicit type names on struct instantiation; auto-dereference for member access (`ptr.field`) with explicit `ptr.*` for values; `pub fn` vs module-private `fn`. |
| **9. Error Safety & Diagnostics** | Q86–Q93 | Lightweight error sets without hidden payloads; stack-tracing panics with `std.panic.setHook()`; division by zero traps in safe modes; compiler-enforced null safety; `@assert(cond)` in safe modes; stack probe guards; native LLVM UBSan and ASan integration. |
| **10. Concurrency & Parallelism** | Q94–Q102 | OS threads + work-stealing pool; C11/C++20 atomics; standard sync primitives (`Mutex`, `RwLock`, `CondVar`, `Semaphore`, `Once`); `threadlocal var`; library-based event loops (`epoll`/`kqueue`/`io_uring`/`IOCP`); typed channels; structured scopes (`std.Thread.scope`); fiber context primitives; direct SIMD `@Vector(N, T)`. |
| **11. Metaprogramming & Comptime** | Q103–Q108 | `@typeInfo(T)` compile-time reflection; `@Type(info)` reification; 100% metaprogramming via normal Juyu functions at compile time (no AST macros); `@embedFile("path")`; guaranteed loop unrolling (`inline for`); `@compileError` and `@compileLog`. |
| **12. Stdlib & Tooling Ecosystem** | Q109–Q120 | Broad, self-contained systems standard library; `Reader`/`Writer` buffered I/O; `std.fs`, `std.net`, `std.process.Child`, `std.time`; decentralized Git package management; canonical formatter `juyu fmt`; integrated `juyu lsp`; static documentation generator `juyu doc`; Clang header AST translator `@cImport`; scaffold Rust compiler repo + formal specification. |

---

## 3. Bootstrap Compiler (v0) Repository Architecture

The Rust bootstrap compiler repository is structured as a modular Cargo workspace:

```
juyulang/
├── Cargo.toml                  # Workspace manifest
├── spec/
│   └── juyu_spec.md            # Formal language specification
├── compiler/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── syntax/             # Lexer, Tokens, Pratt Parser, CST/AST
│       │   ├── mod.rs
│       │   ├── token.rs        # Token kinds, Keywords, Spans
│       │   ├── lexer.rs        # Hand-written fast lexer
│       │   ├── ast.rs          # Typed and Untyped AST nodes
│       │   └── parser.rs       # Recursive descent + Pratt parser
│       ├── sema/               # Semantic analysis & type checking
│       │   ├── mod.rs
│       │   ├── types.rs        # Type representations
│       │   └── typecheck.rs    # Type checker & symbol resolver
│       ├── ir/                 # High-Level SSA IR (JIR)
│       │   ├── mod.rs
│       │   ├── instructions.rs # SSA instructions
│       │   └── lower.rs        # AST -> JIR lowering
│       ├── comptime/           # Compile-time bytecode VM
│       │   ├── mod.rs
│       │   └── vm.rs           # Deterministic sandboxed interpreter
│       └── codegen/            # LLVM backend
│           ├── mod.rs
│           └── llvm.rs         # JIR -> LLVM IR generation
├── driver/                     # CLI binary (`juyu`)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs             # CLI dispatcher (`build`, `test`, `fmt`, `lsp`, `run`)
└── examples/
    ├── hello_world.ju          # Minimal example
    └── pointers_and_memory.ju  # Memory & optional pointers showcase
```

---

## 4. Verification & Testing Strategy

1. **Lexer & Parser Unit Tests**: Tokenization accuracy, multiline string indentation stripping, error recovery on incomplete input, and operator precedence verification in Pratt parser.
2. **Semantic & Typecheck Tests**: Checking explicit conversions, pointer mutability, optional unwrapping rules, and mandatory argument labels.
3. **Comptime VM Tests**: Validating pure constant evaluation, loops, and `@sizeOf`/`@alignOf` calculations.
4. **End-to-End Integration Tests**: Compiling `.ju` programs to executables and asserting on return codes and standard output.
