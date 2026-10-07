# Juyu

A systems language taking its first executable steps.

Juyu's Rust bootstrap compiler parses and typechecks a small language subset,
generates C, and uses a system C compiler to produce native executables. Functions,
scalar types, lexical scopes, control flow, and a tiny printing runtime work today.

**Status: early bootstrap.** The [language specification](spec/juyu_spec.md)
describes a much larger design. Features in the specification are not necessarily
implemented. This README covers the executable subset.

```ju
fn square(n: i64): i64 => n * n;

export fn main() {
    std.io.println("Hello from Juyu!");
    std.io.printlnInt(square(7));
}
```

```text
Hello from Juyu!
49
```

## Build and run

You need a recent stable Rust toolchain (`cargo`) and a system **Clang or GCC**.
The C backend uses C11 plus Clang/GCC checked-overflow builtins and `__int128`.
The native test suite also requires the compiler's UBSan runtime.

```sh
git clone https://github.com/0x251C08/juyu.git
cd juyu
cargo build --release --workspace

./target/release/juyu check examples/bootstrap/hello.ju
./target/release/juyu build examples/bootstrap/hello.ju
./examples/bootstrap/hello

# Build and execute in one step:
./target/release/juyu run examples/bootstrap/math.ju
```

A build leaves both the generated `.c` file and executable beside the `.ju`
source, so you can inspect the output directly:

```sh
cat examples/bootstrap/hello.c
file examples/bootstrap/hello
```

Juyu searches for `clang`, then `gcc`. Set `CC` to select a compiler executable:

```sh
CC=gcc ./target/release/juyu run examples/bootstrap/control.ju
```

`CC` is one executable name or path, not a shell command with flags. Juyu does not
download or bundle a C compiler. Builds currently target the host machine.

## What works today

| Area | Executable support |
| --- | --- |
| Types | `void`, `bool`, `i8/i16/i32/i64`, `u8/u16/u32/u64`, `f32/f64`, `str` |
| Functions | Typed parameters and returns, forward calls, recursion, expression bodies |
| Bindings | Local `let` and `var`, type inference, lexical scopes, nested shadowing |
| Expressions | Literals, arithmetic, comparisons, boolean operations, numeric casts, calls |
| Assignment | `=`, `+=`, `-=`, `*=`, `/=` on mutable variables |
| Control flow | Statement blocks, `if/else`, `while`, `loop`, `break`, `continue`, `return` |
| Strings | Immutable byte slices from literals; UTF-8, embedded NUL, value equality |
| Runtime | `std.io.println(msg: str)` and `std.io.printlnInt(val: i64)` |
| Diagnostics | Source-located errors for names, types, calls, mutability, returns and loop control |

Unsupported constructs are rejected by semantic analysis rather than silently
lowered to guessed C. Some additional syntax can be parsed without being accepted
by the executable compiler.

## A short language tour

### Values and types

`let` creates an immutable binding; `var` allows reassignment. Parameters are
immutable. Annotations are optional when the type can be inferred.

```ju
fn example() {
    let limit: i64 = 10;
    var count: i64 = 0;
    count += 1;

    let enabled = true;
    let message: str = "Ready";
    let ratio: f32 = 1.25;
    let wider: f64 = ratio as f64;
}
```

Typed numeric values require explicit conversion with `as`. Unsuffixed integer
literals use a contextual integer type or default to `i32`; floating literals use
`f32`/`f64` context or default to `f64`. Integer literals are range-checked.

Integer overflow, underflow, invalid division, and out-of-range numeric casts
abort at runtime. Boolean `&&` and `||` short-circuit. Call arguments evaluate
left-to-right. See [bootstrap semantics](docs/bootstrap.md#accepted-subset) for
conversion details.

### Functions and control flow

Functions can use a block with `return`, a final `=> expr;` in the function body,
or an expression body:

```ju
fn square(n: i64): i64 => n * n;

fn sumUntil(limit: i64): i64 {
    var n: i64 = 0;
    var total: i64 = 0;
    while (n < limit) {
        n += 1;
        if (n == 3) {
            continue;
        }
        if (n == 7) {
            break;
        }
        total += n;
    }
    return total;
}

export fn main() {
    std.io.printlnInt(sumUntil(8));
}
```

Arguments may be positional or labeled with the parameter's name, such as
`square(7)` or `square(n: 7)`. Value-producing nested blocks and `if` expressions
are not implemented yet; use statement branches and explicit returns.

The executable entrypoint is `main()` with no parameters, returning either
`void` or `i32`. A `void` main exits successfully; an `i32` main supplies the
process exit status. **Error unions, including `!void`, are not supported yet.**

## Commands

| Command | Behavior |
| --- | --- |
| `juyu check <file.ju>` | Parse and typecheck without generating an executable |
| `juyu build <file.ju>` | Typecheck, generate C, invoke the C compiler, verify and publish the executable |
| `juyu run <file.ju>` | Build, execute with inherited stdout/stderr, and propagate exit status |
| `juyu cc [args]` | Invoke the selected system C compiler |
| `juyu codebase [args]` | Invoke system `rg` (requires ripgrep) |
| `juyu repl` | Inspect parsed ASTs; it does not evaluate code |
| `juyu version` | Show the bootstrap version |
| `juyu --help` | Show command usage |

`juyu test` and `juyu fmt` explicitly report that they are not implemented. Use
Cargo to run the compiler's Rust tests and format its Rust sources.

Failed compilation returns a nonzero status. Generated C is kept for inspection;
`run` never executes an older binary after a failed build. A successful build
requires an actual nonempty executable, not just a successful compiler exit code.

## Examples

Start with the programs in [`examples/bootstrap/`](examples/bootstrap/):

| Example | Demonstrates | Output |
| --- | --- | --- |
| [`hello.ju`](examples/bootstrap/hello.ju) | String output and entrypoint | `Hello from Juyu!` |
| [`math.ju`](examples/bootstrap/math.ju) | Function calls and recursion | Square: `49`; factorial: `720` |
| [`control.ju`](examples/bootstrap/control.ju) | Mutable state, branches, loops, break/continue | `36` |

The older files directly under `examples/` and in `runs/` demonstrate proposed
language features. They are retained as design references and are **not** a
working bootstrap example suite.

Likewise, [`lib/std/`](lib/std/) contains spec-facing library source, not the
runtime currently executed by this compiler. The implemented runtime lives in
[`compiler/src/runtime.rs`](compiler/src/runtime.rs) and its adjacent C helpers.

## Compiler architecture

```text
Juyu source
  -> lexer / parser
  -> AST
  -> semantic analysis
  -> validated typed Program
  -> C backend
  -> system Clang / GCC
  -> native executable
```

Sema collects function signatures before checking bodies, maintains lexical
scopes, resolves bindings to symbol IDs, and checks return paths. The C backend
consumes validated IR, not source-text patterns. Bootstrap runtime functions use
the same name lookup and argument checking as user functions.

| Path | Responsibility |
| --- | --- |
| [`compiler/src/syntax/`](compiler/src/syntax/) | Lexer, parser, AST and source spans |
| [`compiler/src/sema/`](compiler/src/sema/) | Scopes, types, function resolution and validated IR |
| [`compiler/src/diagnostic.rs`](compiler/src/diagnostic.rs) | Structured semantic diagnostics |
| [`compiler/src/codegen/`](compiler/src/codegen/) | C emission and checked numeric operations |
| [`compiler/src/runtime.rs`](compiler/src/runtime.rs) | Bootstrap runtime function declarations |
| [`driver/src/`](driver/src/) | CLI, build artifacts, compiler invocation and execution |
| [`compiler/tests/`](compiler/tests/) and [`driver/tests/`](driver/tests/) | Semantic, codegen and CLI regression tests |
| [`spec/`](spec/) | Broader language design |

For the implementation audit and detailed boundaries, read
[`docs/bootstrap.md`](docs/bootstrap.md).

## Contributing and validation

Keep changes small, add tests for behavior, and distinguish executable support
from language design. Before submitting a change, run:

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The suite covers parser behavior, positive and negative semantic cases, native C
execution under UBSan, runtime output, and CLI failures. Invalid fixtures live in
[`compiler/tests/fixtures/invalid/`](compiler/tests/fixtures/invalid/).

For an end-to-end check, build an example, inspect it with `file`, execute the
binary directly, then execute it through `juyu run`.

## Not implemented yet

The bootstrap intentionally stops short of the full specification. It does not
implement structs/enums/unions, globals, pointers/arrays, optionals/error unions,
`defer`, modules/imports, generics/interfaces, `for`/`match`, string interpolation,
FFI, comptime, LLVM, LSP, a package manager, or the full standard library.

Diagnostics currently stop at the first error, and return-path analysis is
conservative. Cross-compilation and freestanding builds are not implemented.
These boundaries are explicit so each next feature can be added and tested
without implying that the rest of the specification already works.
