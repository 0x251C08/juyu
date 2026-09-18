# Juyu — a systems programming language (bootstrap v0)

Juyu is a modern, statically-typed, high-performance **systems programming
language**: Zig-flavoured syntax, explicit memory management, first-class
errors and optionals, and a tooling-first workflow (`build`, `check`,
`test`, `fmt`, `lsp` — one binary, no setup).

> **Stage: v0 bootstrap.** The compiler is written in Rust, typechecks
> Juyu source, and lowers it to C, which Clang/GCC turns into native
> binaries. The full language vision lives in [`spec/juyu_spec.md`](spec/juyu_spec.md)
> and [`docs/`](docs/); this README documents **what the bootstrap
> actually does today**. Anything marked *(roadmap)* is planned, not done.

```ju
//! Hello World in Juyu

export fn main(): !void {
    let language: &[u8] = "Juyu";
    let version: f32 = 0.1;

    std.io.println($"Hello, systems programming from {language} v{version}!");

    let sum = add(10, 20);
    std.io.printlnInt(sum as i64);
}

export fn add(a: i32, b: i32): i32 => a + b;

test "verify addition" {
    @assert(add(2, 3) == 5);
}
```

---

## Table of contents

1. [Status snapshot](#1-status-snapshot)
2. [Quickstart](#2-quickstart)
3. [Toolchain tour](#3-toolchain-tour)
4. [Language tour](#4-language-tour)
5. [Types and odd bit-widths](#5-types-and-odd-bit-widths)
6. [Errors, optionals, and null safety](#6-errors-optionals-and-null-safety)
7. [Memory model](#7-memory-model)
8. [Standard library](#8-standard-library)
9. [Compiler architecture](#9-compiler-architecture)
10. [Diagnostics reference](#10-diagnostics-reference)
11. [Editor support (LSP)](#11-editor-support-lsp)
12. [Examples and runs catalog](#12-examples-and-runs-catalog)
13. [Project layout](#13-project-layout)
14. [Roadmap](#14-roadmap)

---

## 1. Status snapshot

| Area | State in v0 |
|---|---|
| Lexer + recursive-descent/Pratt parser | ✅ Complete, tested |
| Name resolution + typechecker (`sema`) | ✅ Complete, tested |
| C backend + Clang/GCC driver | ✅ All 8 sample programs build and run |
| Structured diagnostics (`E1xxx`/`E2xxx`) | ✅ Complete |
| `return` / `break` / `continue` statements | ✅ Parse, check, and compile |
| Odd-width integers/floats (`i24`, `u15`, `f128`, …) | ✅ Parse, check (incl. range errors), compile |
| `juyu check/build/run/test/fmt/repl/cc/codebase` | ✅ Working |
| `juyu lsp` (diagnostics, hover, goto-def, symbols, completion) | ✅ Working, tested (`editors/tests/lsp_smoke.py`) |
| Wrapping (`+%`), bitwise (`&`, `bitand`), `match`, `defer` | ✅ Working |
| Generics (`ArrayList<i32>`), interfaces, `extend`, embedding | ✅ Parse + check; codegen covers the prelude-known set |
| Module/file imports on disk | ⚠️ Parsed, not resolved (single-file programs for now) |
| `lib/std/*.ju` full standard library | ⚠️ Written against the full spec; bootstrap executes a runtime subset (see §8) |
| LLVM backend, monomorphization, comptime VM | 🔲 Roadmap (§14) |
| Build modes/cache, `juyu doc`, full formatter, test harness | 🔲 Roadmap (§14) |

---

## 2. Installation

### Option A — Build from source (recommended for contributors)

Prerequisites: Rust toolchain (`cargo`), plus Clang ≥ 19 or GCC ≥ 14
(the driver falls back to a bundled compiler otherwise).

```sh
# 1. Clone the repo
git clone https://github.com/anomalyco/juyulang.git
cd juyulang

# 2. Move the folder to /opt/
sudo mv juyulang /opt/juyulang
cd /opt/juyulang

# 3. Build the driver (single-threaded flag is kind to weak hardware)
cargo build --release --jobs 1

# 4. Symlink the binary so `juyu` is on PATH
sudo ln -sf /opt/juyulang/target/release/juyu /usr/local/bin/juyu

# 5. Verify
juyu version
juyu check examples/hello_world.ju
```

To update later: `cd /opt/juyulang && git pull && cargo build --release --jobs 1`.

To uninstall: `sudo rm /usr/local/bin/juyu && sudo rm -rf /opt/juyulang`.

### Option B — Prebuilt binary from GitHub Releases

No Rust toolchain needed. Grab the latest `juyu-linux-x86_64` (or
`juyu-linux-aarch64`) asset from
[GitHub Releases](https://github.com/anomalyco/juyulang/releases).

```sh
# 1. Download the binary (replace v0.1.0 with the latest tag)
curl -LO https://github.com/anomalyco/juyulang/releases/download/v0.1.0/juyu-linux-x86_64
chmod +x juyu-linux-x86_64

# 2. Move it into /opt/
sudo mkdir -p /opt/juyulang
sudo mv juyu-linux-x86_64 /opt/juyulang/juyu

# 3. Symlink the binary so `juyu` is on PATH
sudo ln -sf /opt/juyulang/juyu /usr/local/bin/juyu

# 4. Verify
juyu version
juyu check /opt/juyulang/examples/hello_world.ju
```

To uninstall: `sudo rm /usr/local/bin/juyu && sudo rm -rf /opt/juyulang`.

### Quickstart (after either install)

```sh
# Check, build, and run the hello-world example
juyu check examples/hello_world.ju
juyu build examples/hello_world.ju
./examples/hello_world
# Hello, systems programming from Juyu v0.1!

# Or compile-and-run in one step
juyu run examples/hello_world.ju
```

A good next read after this file: `runs/01_basic_math_and_logic.ju`
through `runs/06_io_and_networking.ju` — six small programs that tour the
language, all verified to build and run.

---

## 3. Toolchain tour

One binary, eleven commands:

| Command | What it does |
|---|---|
| `juyu build <file>` | Parse → typecheck → lower to C → compile to a native executable next to the source |
| `juyu run <file>` | `build`, then execute immediately |
| `juyu check <file>` | Parse + semantic analysis only; prints `✓ Syntax OK` / `✓ Sema OK` or diagnostics |
| `juyu test [file]` | Discover `test "name" { … }` blocks and report them (execution harness is roadmap) |
| `juyu fmt <path>` | Bootstrap formatter: validates syntax, trims trailing whitespace, one trailing newline |
| `juyu repl` | Interactive loop: parses each line and prints the AST (evaluation is roadmap) |
| `juyu lsp` | JSON-RPC 2.0 language server over stdio (see §11) |
| `juyu cc [args]` | Invoke system Clang/GCC or the bundled compiler |
| `juyu codebase [args]` | Search the codebase with system/bundled ripgrep |
| `juyu version` | Print the compiler version |

`check` is the fast feedback loop — it runs the full typechecker without
invoking the C backend:

```sh
$ juyu check runs/02_structs_and_interfaces.ju
Checking syntax for: runs/02_structs_and_interfaces.ju
✓ Syntax OK: Parsed 7 top-level declarations successfully.
✓ Sema OK: No semantic errors.
```

---

## 4. Language tour

### 4.1 Functions

Return type after a colon, `void` when omitted, arrow bodies for single
expressions, `=> expr;` yields for block bodies:

```ju
export fn add(a: i32, b: i32): i32 => a + b;

export fn classify(n: i32): []u8 {
    if (n < 0) {
        => "negative";
    } else {
        => "non-negative";
    }
}
```

### 4.2 Bindings: `let` vs `var`

`let` is immutable, `var` is mutable — and the typechecker enforces it
(assigning to a `let` is `E2007`). Types are optional; without one the
compiler infers (`5` → `i32`, `1.5` → `f64`):

```ju
let pi: f64 = 3.14159;
var counter = 0;
counter += 1;
```

### 4.3 Structs, methods, and composition

Methods live inside the struct, `static fn` builds values, `&self` /
`&mut self` are receivers, and `extend` adds methods from outside —
including interface implementations:

```ju
export struct Player {
    Entity, // anonymous embedding: promotes Entity's fields
    name: &[const u8],
    health: i32,

    export static fn init(id: u64, name: &[const u8]): Player {
        => Player {
            Entity: Entity { id: id, x: 0.0, y: 0.0 },
            name: name,
            health: 100,
        };
    }

    export fn takeDamage(&mut self, amount: i32) {
        self.health -= amount;
        if (self.health < 0) {
            self.health = 0;
        }
    }
}

export interface Describable {
    fn describe(&self): &[const u8];
}

extend Player {
    export fn describe(&self): &[const u8] {
        => self.name;
    }
}
```

Both struct-literal styles work: `Point { x: 1, y: 2 }` and
`Point { .x = 1, .y = 2 }`. Missing or unknown fields are type errors,
and `p.id` above resolves through the embedded `Entity` (promoted
members typecheck too).

### 4.4 Enums, tagged unions, and `match`

```ju
export enum(u8) HttpMethod {
    Get = 1,
    Post = 2,
    Put = 3,
    Delete = 4,
}

export union(enum) Event {
    Click: Point,
    KeyPress: u8,
    Quit,
}

export fn handle(ev: Event): i32 {
    => match (ev) {
        Event::Click(pt) => pt.x + pt.y,
        Event::KeyPress(code) => code as i32,
        Event::Quit => -1,
    };
}

export fn kind(n: i32): []u8 {
    => match (n) {
        0 => "zero",
        1..9 => "single digit",
        _ => "big",
    };
}
```

Enum backing types must be integers and discriminants are range-checked
(`A = 300` in an `enum(u8)` fails). Match arms unify to one result type.

### 4.5 Control flow

`if` works as statement and expression, `if let` unwraps optionals,
all three loop forms exist, and `break` / `continue` / `return`
are real statements:

```ju
export fn firstPositive(xs: []i32): i32 {
    for (x in xs) {
        if (x > 0) {
            return x;
        }
    }
    return -1;
}

export fn countdown(n: i32): void {
    var i = n;
    while (i > 0) {
        std.io.printlnInt(i as i64);
        i -= 1;
    }
}

export fn spin(): void {
    loop {
        break;
    }
}

export fn withIndex(xs: []i32): void {
    for (var i = 0; i < xs.len(); i += 1) {
        std.io.printlnInt(xs[i] as i64);
    }
}
```

### 4.6 `defer` and `errdefer`

Deferred expressions run LIFO at scope end — the backbone of the
explicit-resource style (§7):

```ju
var gpa = std.heap.GeneralPurposeAllocator.init();
defer {
    if (!gpa.deinit()) {
        std.io.println("Memory leak detected!");
    }
};
```

### 4.7 Strings

Double-quoted literals, triple-quoted multiline strings with indentation
stripping, and `$"…"` interpolation (bare identifiers are resolved by
the typechecker):

```ju
let lang = "Juyu";
std.io.println($"Hello from {lang}!");

let poem = """
    Roses are red,
    pointers are fun.
    """;
```

### 4.8 Operators, casts, and builtins

Arithmetic, comparison, logic, bit ops (`& | ^ ~ << >>`, plus the
`bitand`/`bitor`/`bitxor` word forms), wrapping arithmetic (`+%`, `-%`,
`*%` — wrapping is the only correct overflow story and typechecks as
integer-only), C-style `for` sugar aside:

```ju
let wrapped: u8 = a +% b;   // wraps instead of trapping
let masked = flags & 0x0Fu8;
let ratio = x as f64;       // explicit cast, always written out
@assert(count >= 0);        // aborts with a message when false
let n = @sizeOf(Point);     // compile-time size
```

`@cast(T, x)` also takes a type as its first argument, and `@sizeOf`
accepts type names — the typechecker understands both.

### 4.9 Tests and imports

Test blocks sit next to the code they verify:

```ju
test "clamp keeps values in range" {
    @assert(clamp(5, 0, 10) == 5);
    @assert(clamp(-3, 0, 10) == 0);
}
```

Selective imports parse today (`import { read } from "fs.ju";`) and are
recorded by name resolution; loading modules from disk is roadmap —
v0 programs are single files with `std.*` paths resolving to the
built-in prelude.

---

## 5. Types and odd bit-widths

Juyu has the usual fixed widths **plus odd widths the hardware people
keep asking for**. Every one below parses, typechecks, and compiles:

| Family | Types |
|---|---|
| Signed | `i8`, `i15`, `i16`, `i24`, `i32`, `i33`, `i36`, `i64`, `i128`, `i256`, `isize` |
| Unsigned | `u1`, `u4`, `u8`, `u15`, `u16`, `u24`, `u32`, `u33`, `u36`, `u64`, `u128`, `u256`, `usize` |
| Float | `f15`, `f24`, `f32`, `f33`, `f36`, `f64`, `f128`, `f256` |
| Other | `bool`, `char`, `str`, `void` |

Small widths lower bit-exactly where C allows (`u1`/`u4` pack into bytes,
`i24`/`u24` into 32-bit carriers); 128-bit uses `__int128`/`_Float128`.
Integer literals take suffixes (`5u24`, `100i15`) and — this is the part
people notice — **comptime-known values are range-checked against sized
targets**:

```ju
let ok: u8 = 255;     // fine
let bad: u8 = 300;    // E2003: integer literal 300 is out of range for 'u8'
let edge: i24 = 8388607;   // 2^23 - 1, fine
let over: i24 = 8388608;   // E2003
```

Unsuffixed literals coerce to any numeric type; suffixed literals whose
value fits the target are accepted too (`let x: i32 = 5u8;` works).

Compound types:

```ju
let a: [4]u8 = ...;        // fixed array
let s: []u8 = "hi";        // slice — same layout as str literals
let z: [:0]u8 = ...;       // sentinel-terminated slice
let p: *const Point = ...; // pointers, *T and *const T
let m: [*c]u8 = ...;       // C pointer
let o: ?i32 = null;        // optional (see §6)
let e: !i32 = ...;         // error union (see §6)
let t: (i32, f64) = ...;   // tuples, labels allowed
```

---

## 6. Errors, optionals, and null safety

Null doesn't exist as a value you can slip into an `i32`. Absence is
`?T`, failure is `!T`, and both are unwrapped explicitly:

```ju
export fn find(id: u64): ?Player {
    if (id == 0) {
        => null;
    }
    => players[id];
}

export fn load(id: u64): !Player {
    let maybe = find(id);
    // `??` needs an optional left side and a matching fallback:
    => maybe ?? Player { .name = "anon", .health = 0 };
}

export fn mustLoad(id: u64): !Player {
    // `try` unwraps error unions; `!` postfix does the same:
    let p = try load(id);
    let q = load(id)!;
    => p;
}

export fn withFallback(id: u64): Player {
    // `catch` handles the error inline:
    => load(id) catch Player { .name = "fallback", .health = 1 };
}
```

Misusing them is a type error, not a runtime surprise: `try` on a
non-error-union, `??` on a non-optional, and `?.` on a non-optional are
all `E2007`. Optional chaining collapses to null and feeds `??`:

```ju
let city = user?.profile?.city ?? "unknown";
```

---

## 7. Memory model

There is no garbage collector and no borrow checker. Memory is explicit:

* Every allocation names its allocator (`std.mem.Allocator` passed as a
  parameter, Zig-style).
* `defer` / `errdefer` release resources at scope end (§4.6).
* `let` vs `var` plus the typechecker give you const-correctness without
  lifetime annotations.
* Debug poisoning, bounds checking, and leak detection are designed in
  (GPA leak check ships in the runtime prelude; sanitizer integration is
  roadmap).

The bootstrap offers a pragmatic escape hatch while the full allocator
suite lands: `PageAllocator.alloc(T, n)` lowers to `malloc` with a
length-carrying buffer, and `defer` frees it:

```ju
var buf = std.heap.PageAllocator.alloc(u8, 64)!;
defer std.heap.PageAllocator.free(buf);
```

---

## 8. Standard library

`lib/std/` is a twelve-module standard library — `mem`, `heap`, `io`,
`fs`, `process`, `net`, `time`, `sync`, `math`, `fmt`, `debug` behind a
`std.ju` root with top-level re-exports (`Allocator`, `Reader`,
`Writer`, `ArrayList`, `Instant`, …). One honest caveat: **it is written
against the full language spec** (error sets, `callconv(.C)`,
`comptime` params, `*anyopaque`) and does not yet parse under the
bootstrap (try `juyu check lib/std/io.ju` and watch `E1002`). The
bootstrap instead ships a runtime **prelude** baked into generated C
that covers what the sample programs need:

| Prelude item | Provides |
|---|---|
| `ArrayList_i32` + `init/append/get/len/deinit` | Growable `i32` list |
| `GeneralPurposeAllocator` + `allocator/deinit` | Leak-counting allocator |
| `ArenaAllocator` | Bump allocator over a 64 KiB block |
| `Instant::now/elapsed`, `Duration::toMillis` | Monotonic timers |
| `std.fs.cwd()`, `juyu_buf` | Filesystem root + length-carrying buffers |
| `std.net.Address::parseIp4` | IPv4 address parsing |
| `juyu_str`, `juyu_println`, `JUYU_TO_CSTR` | String slices, printing, interpolation |

`std.io.println`, `printlnInt`, `alloc`/`free`, `cwd`, `now`,
`parseIp4` in sample programs resolve to these. Migrating `lib/std`
onto the implemented language is tracked work, not done work.

---

## 9. Compiler architecture

```
.ju source
  → Lexer (hand-written, nestable block comments, suffix literals)
  → Parser (recursive descent + Pratt expressions, both struct-literal styles)
  → sema (name resolution + type checking, E2xxx on failure)
  → C backend (GNU C: statement-expressions, _Generic printing)
  → clang / gcc -O2 → native binary
```

Crates and files:

* `compiler/` — `juyu_compiler`: `syntax/{lexer,parser,ast,token,diag}`,
  `sema.rs` (~2700 lines: scopes, unification, range checks, method
  resolution, exhaustiveness helpers), `codegen.rs` (C emitter + `cc`
  invocation), 48 unit/integration tests.
* `driver/` — the `juyu` binary: `main.rs` (11 subcommands) plus
  `lsp.rs` + `lsp/analysis.rs` (dependency-free JSON-RPC server) and
  `toolchain.rs` (bundled compiler/ripgrep bootstrap).
* `lib/std/` — spec-facing standard library (see §8 caveat).
* `examples/`, `runs/` — the 8 programs that must always build + run.
* `editors/` — micro/kate/VS Code integration + LSP smoke test.
* `spec/`, `docs/` — language spec and design records.

`juyu build` runs the whole pipeline and keeps the intermediate `.c`
next to the binary on failure for inspection.

---

## 10. Diagnostics reference

Errors are structured — code, span, message, hint — and greppable:

```text
error[E2003] at line 4, col 27: initializer for 'language': expected '[]u8', got 'str'
```

| Code | Meaning | Example trigger |
|---|---|---|
| E1001 | Expected token | missing `;`, `)` |
| E1002 | Unexpected top-level item | stray expression outside a declaration |
| E1003 | Unexpected expression token | `break` where no statement starts (pre-`return`-support files) |
| E1004 | Bad method receiver | `&foo` instead of `&self` |
| E1005 | Bad import declaration | `import x from` without braces |
| E1006 | Bad test block header | `test 42 { … }` |
| E1007 | Bad array length | `[x]u8` with non-integer length |
| E2001 | Undefined variable/function | typo'd name, missing `let` |
| E2002 | Undefined type | unknown annotation, `Self` outside a method |
| E2003 | Type mismatch / out-of-range literal | `let x: u8 = 300;` |
| E2004 | Duplicate definition | two `fn f`, double `let x` |
| E2005 | Wrong argument count | `add(1)` for a 2-param function |
| E2006 | Unknown member | missing field, bad method, bad union variant |
| E2007 | Invalid operation | non-bool `if`, `try` on non-union, assigning to `let` |
| E2008 | Invalid literal | unknown suffix like `5u7` |
| E2009 | Return mismatch | `=> true;` in an `i32` function |
| E2010 | Bad control flow | `break` outside a loop |

---

## 11. Editor support (LSP)

`juyu lsp` speaks JSON-RPC 2.0 over stdio with zero dependencies: full-doc
sync, `publishDiagnostics` from the real parser+sema on every keystroke,
plus hover signatures, go-to-definition (exact name spans), document
symbols, and completion (in-scope locals, globals, keywords, word
fallback for broken files). Verified by a scripted session in
`editors/tests/lsp_smoke.py` (19 checks: initialize → diagnostics →
hover → definition → symbols → completion → change → shutdown).

v0 editor wiring lives in `editors/`: a minimal VS Code extension
(`package.json`, client, TextMate grammar), Kate server JSON + syntax
XML, and micro `lsp.server` settings + syntax YAML. Server side is
tested; editor-side setup needs a human with each editor installed.

---

## 12. Examples and runs catalog

| File | Shows |
|---|---|
| `examples/hello_world.ju` | Minimal program, printing, interpolation, test block |
| `examples/pointers_and_memory.ju` | GPA + leak check, `ArrayList`, pointers, optionals |
| `runs/01_basic_math_and_logic.ju` | Scalars, `u4`/`u1`/`i24`, wrapping, conditionals |
| `runs/02_structs_and_interfaces.ju` | Embedding + promotion, methods, `extend`, interfaces |
| `runs/03_memory_and_allocators.ju` | GPA, arenas, collections, `defer` cleanup |
| `runs/04_pattern_matching_and_enums.ju` | Backed enums, tagged unions, literal/range/variant match |
| `runs/05_pointers_and_optionals.ju` | `?T`, `?.`, `??`, `!`, casts |
| `runs/06_io_and_networking.ju` | Timers, filesystem, IPv4 parsing |

---

## 13. Project layout

```text
juyulang/
├── compiler/            # juyu_compiler crate
│   ├── src/syntax/      # lexer, parser, AST, tokens, diagnostics
│   ├── src/sema.rs      # name resolution + typechecker
│   ├── src/codegen.rs   # C backend + cc invocation
│   ├── src/lib.rs       # parse_source / check_source API
│   └── tests/           # lexer, parser, sema suites (48 tests)
├── driver/              # juyu binary: CLI, LSP server, toolchain bootstrap
├── lib/std/             # spec-facing standard library (12 modules)
├── examples/ runs/      # the 8 must-build programs
├── editors/             # micro / kate / vscode integration + LSP smoke test
├── spec/ docs/          # language spec + design records
└── README.md            # you are here
```

---

## 14. Roadmap

Roughly in planned order. Nothing here is claimed as done:

1. **Migrate `lib/std` onto the implemented language** so the spec
   library actually compiles, module by module.
2. **Real module system** — resolve `import` from disk (unity builds),
   then a build cache and parallel per-file codegen.
3. **Build modes as flags** (`-O Debug/ReleaseSafe/ReleaseFast/ReleaseSmall`),
   with bounds checking and wrapping builtins wired to the mode.
4. **Error-union ABI + `errdefer` tracking**, per-scope `defer` in loops.
5. **Monomorphization** for user generics (today only prelude-known
   instantiations like `ArrayList_i32` exist), then `any Trait` vtables.
6. **LLVM-IR backend** (begun as the long-term codegen target; the C
   backend stays as the portable fallback and MSVC story).
7. **Comptime bytecode VM**, closures, tuples-as-values polish, bit-exact
   small-int lowering end to end.
8. **Full canonical formatter**, real test harness (compile-and-run test
   blocks), `juyu doc`, structured build cache.
9. **Self-hosting**: Juyu compiler written in Juyu.

Experimental bootstrap, built in the open. If a sample stops building,
that is a release-blocking bug — file it with the `.ju` attached.
