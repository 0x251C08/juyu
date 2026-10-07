# Small executable bootstrap

This document describes the implementation added on top of `df329fe`, not the
full language specification. The root README provides a shorter getting-started
guide to this executable subset.

## Audit of df329fe (before this change)

| Area | Implementation actually present |
| --- | --- |
| Lexer | Handwritten lexer; nested comments, numeric/string tokens, keywords and operators. Numeric parse failures silently became zero. No `return` keyword. |
| AST | Untyped declarations, type syntax, expressions and statements. Includes many planned features. Has return/break/continue nodes that the parser could not produce. |
| Parser | Recursive descent and Pratt parser returning `Result<SourceFile, String>`. Partial syntax support, no semantic checking. |
| Spans/diagnostics | Byte start/end plus line/column on tokens and most AST nodes. String parser errors; no semantic diagnostic type. Some declaration spans pointed to their final token. |
| Compiler API | `parse_source`, plus re-exported `Lexer`, `Parser`, `SourceFile`. No sema, IR or backend modules. |
| Driver | `check` parsed only. `build` printed fictitious JIR/LLVM/linker success; `run` printed a message without executing. `test` claimed 12 passing suites without running anything; `fmt` printed a message without formatting. `cc`/`codebase` could fall back to `echo`. REPL only displayed ASTs. |
| Tests | Nine Rust tests: four library/AST, one lexer, four parser; all passed. No codegen, semantic, runtime or CLI execution tests. |
| Examples/runs | Eight files, of which only runs 01 and 06 parsed. None could execute. |
| `lib/std` | Twelve spec-facing Juyu files, not a working runtime. Dependencies include unsupported imports, `pub`, error sets, comptime and C imports. Several routines are placeholders (cached args, process spawning, constant monotonic time, networking/synchronization). |
| Spec/docs | Broad design proposals (LLVM, JIR, memory, errors, generics, modules, tooling), not evidence of implementation. Syntax also differs between documents. |
| README | Claims sema, a C backend/runtime prelude, 48 tests, working examples and LSP/editor support. Those implementation files and editor directory were absent. |

## Architecture now

```text
source -> lexer/parser -> AST -> sema -> validated typed Program
       -> C emitter -> system clang/gcc -> native executable
```

`compiler/src/sema` collects function signatures first, then resolves/checks each
body using a stack of lexical scopes. Function calls and variable references
carry resolved IDs. It tracks possible control-flow exits to reject non-void
functions with reachable fallthrough. Even unreachable source is checked.
Parameters share the function body's scope; nested blocks may shadow bindings.

`Program` can only be constructed inside the compiler crate. Its public accessor
allows read-only IR inspection. The C backend receives a `Program`, never raw
source or an unchecked AST. `sema::analyze` returns a structured `Diagnostic`
(code, original source span, message); `check_source` combines parsing and sema
into the existing string-error API style. Diagnostics currently stop at the
first error.

The backend emits prototypes for recursion/forward calls, stable numeric C
identifiers, explicit temporaries to preserve left-to-right evaluation, and
conditional evaluation of `&&`/`||`. String values are immutable byte slices
(pointer + length), supporting embedded NUL and UTF-8. All current string storage
comes from literals; string assignment/pass/return copies that static view.

## Accepted subset

- `void`, `bool`, `i8/i16/i32/i64`, `u8/u16/u32/u64`, `f32/f64`, `str`.
- Functions, positional or correctly named arguments, immutable parameters,
  local `let`/`var`, explicit annotations or inference (`=` or `:=`).
- Integer, decimal floating, boolean and string literals; identifiers;
  unary `-`, `!`, `~`; numeric `+ - * /`, integer `%`; comparisons;
  boolean operations; numeric `as` casts; ordinary function calls.
- Assignment and `+= -= *= /=` to mutable variables.
- Scoped statement blocks, `if/else` and `else if`, `while`, `loop`,
  unlabeled `break`/`continue`, explicit `return`.
- Expression-bodied functions and a final `=> expr;` in the function body.
- `main()` returns `void` (native exit 0) or `i32` (native exit status).
  Library files may pass `check` without main; executable generation requires it.

Typed values must match exactly: numeric widening/narrowing requires `as`.
Unsuffixed integer literals take a contextual integer type or default to `i32`;
unsuffixed floating literals take `f32`/`f64` context or default to `f64`.
Integer literals are range-checked, including signed minima and `u64::MAX`.
Arithmetic overflow/underflow and invalid integer division abort at runtime.
Casts to integer types check representable range before converting; float-to-int
truncates toward zero only after that range check. NaN/infinity cannot convert to
integers. Finite `f64` values outside `f32` range also abort on conversion.
Floating arithmetic otherwise uses the host C floating behavior.

The C backend targets system Clang/GCC with C11 plus their checked-overflow
builtins and `__int128` extension (used in checked integer casts). It is a native
host bootstrap, not a portable or cross-compilation promise.

## Runtime

The bootstrap function table in `compiler/src/runtime.rs` declares:

```text
std.io.println(msg: str): void
std.io.printlnInt(val: i64): void
```

These are registered in the normal function namespace. Sema validates their
argument count, labels and types. Resolved runtime call targets select the small
C implementations under `compiler/src/runtime/`; only used functions are emitted.
They print a newline after a byte slice or signed integer. Immediate stdio errors
abort. There is no allocator, filesystem API, libc FFI or general stdlib loading.

## Parser changes

The existing return/break/continue AST variants are now reachable via statement
parsing; only a `Return` token/keyword was added. Function, parameter and binding
spans now start at their names. Bare conditions no longer mistake the following
body brace for a struct initializer; `else if` has a statement-shaped branch.
Malformed/overflowing numeric literals now produce lexical errors instead of
silently becoming zero. Float suffixes are rejected rather than silently lost;
use an annotation such as `let value: f32 = 1.25;`.

All original parser/lexer tests are retained. Rust files also received the
workspace's standard `cargo fmt` formatting.

## CLI and reproduction

```sh
cargo build --workspace
target/debug/juyu check examples/bootstrap/hello.ju
target/debug/juyu build examples/bootstrap/hello.ju
file examples/bootstrap/hello
./examples/bootstrap/hello
target/debug/juyu run examples/bootstrap/hello.ju
```

Repeat with `math.ju` and `control.ju`. Outputs are respectively:

```text
Hello from Juyu!
```

```text
square(7):
49
factorial(6):
720
```

```text
sum(1, 2, 4, 5, 6) * 2:
36
```

`build` requires a `.ju` source, keeps generated `.c` for inspection, and uses
`CC` as a single compiler executable path, or searches for `clang` then `gcc`.
No shell evaluates paths or arguments. Compiler stdout/stderr are inherited.
Compilation occurs into a unique staging directory; success requires a nonempty
regular executable before publishing it next to the source. Existing successful
executables survive failed rebuilds, but `run` never executes them after failure.
Non-generated sibling `.c` files and non-regular output paths are protected.
`run` inherits program stdout/stderr and propagates normal exit codes (or
128 + signal on Unix). It prints no build chatter into program stdout.

`test`/`fmt` now fail explicitly as unimplemented. `cc` and `codebase` execute
real system tools or fail; there is no pretend download or echo fallback.
The REPL is explicitly AST inspection, not execution.

## Tests

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Rust semantic tests cover positive cases, each requested diagnostic, lexical
scoping, numeric boundaries, entrypoint checks and unsupported-feature rejection.
Native codegen tests compile with Clang/GCC and UBSan, then assert actual exit
codes/output: recursion, arithmetic, all widths, casts, shadowing, loop transfers,
short-circuiting, nested comparisons, safe C names, string bytes and I/O.
CLI tests cover all three example flows, every invalid fixture, exit codes,
stale binaries, missing tools/files/main, syntax failures, C compiler failure,
false compiler success, and preservation of user-owned C files. No new dependency
is required; native tests require a system compiler with UBSan support.

Negative programs live in `compiler/tests/fixtures/invalid` and must fail `check`,
`build`, and `run`.

## Deliberately unsupported

Globals, structures/unions/enums, user generics, interfaces, pointers, arrays,
optionals/error unions, defer/errdefer, modules/imports, match, for loops, loop
labels/values, value-producing nested blocks/if expressions, function values,
advanced function modifiers, custom integer widths, wrapping and binary bitwise
operators, interpolation, numeric exponent/float-suffix syntax, builtins/FFI,
comptime, LLVM, LSP, packages, and the full spec-facing standard library.
Unsupported AST constructs fail sema explicitly instead of emitting guessed C.
The old spec-oriented examples remain unchanged and are not claimed to work.
