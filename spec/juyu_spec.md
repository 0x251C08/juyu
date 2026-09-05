# The Juyu Programming Language Specification (v0.1.0)

Juyu is a modern, statically-typed, high-performance systems programming language designed for low-level hardware control, zero-cost abstractions, predictable latency, and maximum developer velocity.

---

## 1. Design Philosophy & Vision

1. **Systems Performance & Zero-Cost Abstractions**: Direct hardware access, deterministic resource management, and peak machine performance via LLVM codegen.
2. **Tooling-First Velocity**: Ultra-fast compilation speed, unified tooling (`juyu build`, `juyu test`, `juyu fmt`, `juyu lsp`, `juyu doc`), and a lean, orthogonal language core.
3. **Seamless C and C++ Interoperability**: Direct `@cImport("header.h")` via embedded Clang AST parsing with zero-cost bidirectional FFI and automatic C ABI alignment.
4. **First-Class Compile-Time Execution (`comptime`)**: Replace textual macros and template metaprogramming with ordinary Juyu code running inside a sandboxed frontend bytecode VM.
5. **Deterministic Memory Without Cognitive Overhead**: Explicit allocator model (`std.mem.Allocator`), destructive move semantics, linear resource tracking, and block-scoped `defer`/`errdefer` cleanup without complex borrow checker annotations.

---

## 2. Compilation Architecture & Toolchain

### 2.1 Compiler Pipeline
- **Bootstrap Implementation**: Written in **Rust** using `llvm-sys`/`inkwell`.
- **Roadmap**: Self-hosting as a major milestone once compiler frontend, comptime VM, and LLVM codegen mature.
- **Frontend Architecture**:
  - Hand-written recursive descent lexer and Pratt expression parser with resilient error recovery.
  - Lossless AST preserving accurate source spans for compiler diagnostics and LSP.
  - Semantic analyzer lowering typed AST to a high-level Static Single Assignment Intermediate Representation (**JIR**).
  - Comptime bytecode VM interpreting JIR during semantic analysis.
- **Backend Architecture**:
  - Pure **LLVM Backend** lowering optimized JIR to LLVM IR.
  - Integrated **LLD** linker bundled by default with pluggable fallback (`-fuse-ld=mold` or system linker).

### 2.2 Compilation Graph & Modules
- **Order Independence**: Package-level top-level declarations (functions, structs, constants) are order-independent; the compiler automatically resolves symbol dependency DAGs without forward declarations or header files.
- **Module Imports**:
  - Full namespace import: `let math = import("math");`
  - Selective member import: `import { sin, cos } from "math";`
- **Visibility**:
  - Items are private to the declaring file by default.
  - `pub` prefix exports symbols publicly (`pub fn`, `pub struct`, `pub let`).

### 2.3 Build Configuration & Packaging
- **Declarative by Default**: `juyu.toml` defines package metadata, dependencies, targets, and compiler flags with no arbitrary code execution during dependency resolution.
- **Programmable Build Script**: Optional `build.ju` script written in Juyu for orchestrating native C/C++ compilation, custom code generation steps, or complex build graphs.
- **Dependency Resolution**: Decentralized Git repository URLs with cryptographic SHA-256 content-hash verification.

### 2.4 Build Profiles
1. `Debug`: Optimizations disabled (`-O0`), full runtime assertions, bounds checking, undefined memory poisoning (`0xAA`), and leak-detection enabled.
2. `ReleaseSafe`: Full LLVM optimizations (`-O3`), with runtime bounds checks, integer overflow traps, and spatial memory safety retained.
3. `ReleaseFast`: Maximum raw performance (`-O3`), bounds checks and safety assertions stripped, aggressive auto-vectorization enabled.
4. `ReleaseSmall`: Binary size optimization (`-Oz`), dead code elimination, and symbol stripping.

### 2.5 Cross-Compilation & Freestanding Execution
- **Out-of-the-Box Cross-Compilation**: Self-contained toolchain packaging cross-compilation target triples for Tier 1 platforms (`x86_64-linux`, `aarch64-linux`, `x86_64-windows`, `aarch64-macos`, `wasm32-freestanding`).
- **Freestanding Core**: Standard library interacts with the OS via direct system calls where feasible; zero-dependency static binaries produced by default. Linking `libc` is an explicit opt-in (`-lc`) or triggered automatically via C imports.

---

## 3. Type System & Sizing

### 3.1 Primitive Types
- **Boolean**: `bool` (`true`, `false`).
- **Signed Integers**: `i8`, `i16`, `i32`, `i64`, `i128`, `isize`.
- **Unsigned Integers**: `u8`, `u16`, `u32`, `u64`, `u128`, `usize`.
- **Arbitrary Bit-Width Integers**: `iN` and `uN` (e.g. `u1`, `u2`, `u4`, `u7`, `u24`) for packed hardware registers and neural-network quantization.
- **Floating Point**: `f32`, `f64`, `f128`.
- **Void Type**: `void`.
- **Never / Bottom Type**: `unreachable`.

### 3.2 Conversions & Arithmetic Safety
- **No Implicit Numeric Coercion**: All numeric widening and narrowing conversions must be explicit via `@cast(TargetType, value)`.
- **Overflow Semantics**:
  - Standard operators (`+`, `-`, `*`) trap/panic on overflow in `Debug` and `ReleaseSafe`.
  - Wrapping operators (`+%`, `-%`, `*%`) provide modular arithmetic.
  - Shift operators (`<<`, `>>`) trap if shift amount >= bit-width; `>>` performs arithmetic shift on signed and logical shift on unsigned.

### 3.3 Pointer & Memory Address Types
1. **Single-Item Pointer (`*T`, `*const T`)**:
   - Strictly non-null. Guaranteed by the compiler.
   - Dereferenced via postfix `.*` (e.g. `ptr.* += 1;`).
   - Member access automatically dereferences: `ptr.field`.
   - Pointer arithmetic is forbidden on `*T`.
2. **Optional Pointer (`?*T`, `?*const T`)**:
   - Zero-cost nullable pointer: address `0x0` represents `null`.
   - `@sizeOf(?*T) == @sizeOf(*T)`.
3. **Slice (`[]T`, `[]const T`)**:
   - Fat pointer containing `ptr: [*]T` and `len: usize`.
   - Bounds-checked indexing `slice[i]` and sub-slicing `slice[a..b]`.
4. **Many-Item Pointer (`[*]T`)**:
   - Pointer to an unknown number of items. Supports pointer arithmetic (`ptr + 1`). Used for C buffers.
5. **C-Compatible Pointer (`[*c]T`)**:
   - Permissive C-style pointer for FFI compatibility, allowing arithmetic and nullability.
6. **Sentinel-Terminated Arrays and Pointers**:
   - `[N:0]T`, `[:0]T`, `[*:0]T` guarantee null-terminated buffers for zero-copy C API passing.

### 3.4 Strings & Text
- Strings in Juyu are UTF-8 byte slices: `[]const u8`.
- String literals are null-terminated byte slices: `[:0]const u8`.
- Multiline strings use triple-quote syntax `"""..."""` with automatic common indentation stripping.
- String interpolation is written `$"Hello, {name:04x}!"`, desugaring to zero-allocation writer calls or explicit allocator buffers.
- Formatted printing is also available via `std.fmt.printf(...)` with compile-time format-string verification.

### 3.5 Type Aliasing & Newtypes
- **Transparent Alias**: `type Byte = u8;` (type synonym).
- **Distinct Newtype**: `type Port = distinct u16;` (zero-cost distinct type preventing accidental parameter swapping without explicit `@cast`).

---

## 4. Memory Model & Resource Lifecycle

### 4.1 Explicit Allocators
Juyu enforces an explicit memory allocation philosophy. No function or container allocates heap memory invisibly.
- **Allocator Interface**: `std.mem.Allocator` is a polymorphic fat pointer (`ptr: *anyopaque, vtable: *const VTable`).
- **Standard Suite**:
  - `PageAllocator`: Direct OS virtual memory syscalls (`mmap`, `VirtualAlloc`).
  - `GeneralPurposeAllocator`: Leak detection, double-free detection, and quarantine safety padding.
  - `ArenaAllocator`: Rapid bulk allocations with O(1) batch teardown via `.deinit()`.
  - `FixedBufferAllocator`: Stack or pre-allocated static buffer management with zero syscalls.
  - `c_allocator`: Direct delegation to system `malloc`/`free`.
- **Collections**: Standard dynamic containers exist in dual variants:
  - Managed: `ArrayList(T)` stores the allocator for ergonomic `.append(x)`.
  - Unmanaged: `ArrayListUnmanaged(T)` stores only `(ptr, len, cap)`, taking the allocator at method calls.

### 4.2 Destructive Move Semantics
- **Scalar Primitives Only Copy**: Only scalar numbers, booleans, and raw pointers are implicitly copyable on assignment.
- **Structs and Arrays Move Destructively**: Assignment (`let b = a;`) or passing by value transfers ownership; the source variable becomes inaccessible at compile-time.
- **Deep Duplication**: Explicit `.clone(allocator)` method required for cloning heap-allocated resources.

### 4.3 Resource Cleanup (`defer` and `errdefer`)
- **Block-Scoped Execution**: `defer` statements run in LIFO order upon exiting the immediate enclosing lexical block.
- **Error Cleanup**: `errdefer` statements execute only if the enclosing function returns an error union.
- **Linear Type Safety**: The compiler issues compile-time errors/warnings if an owned resource struct is dropped without being consumed or deinitialized.

### 4.4 Uninitialized Memory
- Variables must be initialized or explicitly assigned `= undefined`.
- In `Debug` and `ReleaseSafe` modes, `= undefined` memory is poisoned with `0xAA` to deterministically trap uninitialized memory reads.

---

## 5. Syntax & Grammar

### 5.1 Statements & Bindings
- Statements end with mandatory semicolons `;`.
- **Immutable Bindings**: `let x: i32 = 10;`
- **Mutable Bindings**: `var y: i32 = 20;`
- **Type Inference**: `let x = 10;` infers `i32`.

### 5.2 Functions & Procedures
- **Declaration Syntax**: Colons are used uniformly for parameter and return types:
  ```juyu
  pub fn add(a: i32, b: i32): i32 {
      => a + b;
  }
  ```
- **Void Return**: `: void` may be omitted for procedures:
  ```juyu
  pub fn log(msg: []const u8) {
      std.io.println(msg: msg);
  }
  ```
- **Single-Expression Shorthand**:
  ```juyu
  pub fn multiply(a: i32, b: i32): i32 => a * b;
  ```
- **Mandatory Callsite Argument Labels**:
  All function arguments must be labeled at callsite:
  ```juyu
  let sum = add(a: 5, b: 10);
  ```
- **Swift-Style Overloading**: Functions may share a name if their external argument labels differ:
  ```juyu
  pub fn move(to target: Point) { ... }
  pub fn move(by delta: Point) { ... }
  ```
- **Compile-Time Functions**: Pure functions marked `const fn` can be executed at both compile-time and runtime.
- **Linkage Modifiers**: `inline fn`, `noinline fn`, `export fn`, `extern fn`.
- **Raw Assembly**: `naked fn` for assembly routines with zero compiler prologue/epilogue.

### 5.3 Blocks & Control Flow
- **Arrow Block Yields**: Blocks evaluate to a value using `=> expr;`:
  ```juyu
  let result = {
      let x = compute();
      => x * 2;
  };
  ```
- **Conditionals**: `if (cond)` or `if cond`, usable as statement or expression:
  ```juyu
  let status = if (score >= 50) "pass" else "fail";
  ```
- **Optional Unwrapping**:
  ```juyu
  if (let active = maybe_ptr) {
      active.* += 1;
  } else {
      // null case
  }
  ```
- **Pattern Matching (`match`)**: Exhaustive pattern matching with arrow arms:
  ```juyu
  let description = match (state) {
      .Ready => "System ready",
      .Running(let pid) => $"Running PID {pid}",
      else => "Unknown",
  };
  ```
- **Loops**:
  - Infinite: `loop { if (done) break; }`
  - While: `while (cond) { ... }`
  - Three-part C-style: `for (var i = 0; i < n; i += 1) { ... }`
  - Collection iteration: `for (item in slice) { ... }`
  - Labeled loops: `outer: for (x in list) { break outer; }`

### 5.4 Error Handling & Optional Operators
- **Error Sets**: `const FileError = error { NotFound, AccessDenied };`
- **Error Unions**: `!T` or `FileError!T`.
- **Error Propagation**: `let f = try open_file(path: "test.txt");`
- **Error Fallback**: `let port = parse_port(s: raw) catch 8080;`
- **Error Handling**: `let data = fetch() catch (err) { => fallback; };`
- **Null-Coalescing**: `let val = opt ?? default_val;`
- **Optional Chaining**: `let city = user?.profile?.address?.city;`
- **Force Unwrap**: `opt.?` (panics in safe modes if null).

---

## 6. Compound Types, Interfaces & OOP

### 6.1 Struct Layouts
1. `struct Name { ... }`: Compiler-optimized field order to eliminate alignment holes.
2. `extern struct Name { ... }`: Strict C ABI field layout for FFI.
3. `packed struct Name { ... }`: Bit-precise field packing (supporting arbitrary bit widths `u1`, `u7`, `u24`).

### 6.2 Struct Instantiation & Extension
- **Designated Initializers**: Explicit dot prefix with explicit type name:
  ```juyu
  let p = Point { .x = 10.0, .y = 20.0 };
  ```
- **Associated / Static Functions**: Declared with `pub static fn`:
  ```juyu
  pub struct Point {
      x: f32,
      y: f32,

      pub static fn init(x: f32, y: f32): Point => Point { .x = x, .y = y };
      pub fn length(self: *const Point): f32 { ... }
  }
  ```
- **External Method Extensions**:
  ```juyu
  extend Point {
      pub fn taxicab_norm(self: *const Point): f32 => self.x + self.y;
  }
  ```
- **Composition / Field Promotion**: Anonymous embedded structs promote members automatically:
  ```juyu
  struct Entity { id: u64 }
  struct Player {
      Entity, // anonymous embedding
      score: u32,
  }
  // player.id accesses player.Entity.id
  ```

### 6.3 Interfaces & Polymorphism
- **Structural Interfaces**: Satisfied automatically without explicit `impl` keywords:
  ```juyu
  pub interface Reader {
      fn read(mut self, buffer: []u8): !usize;
  }
  ```
- **Static vs Dynamic Dispatch**:
  - Generic functions statically monomorphize interface parameters with zero runtime overhead.
  - Dynamic runtime dispatch uses fat pointers: `*dyn Reader` (data pointer + vtable pointer).
- **Operator Overloading**: Value types implement standard interface operations (`add`, `sub`, `mul`, `index`) to support `a + b` and `vec[i]`.

---

## 7. Metaprogramming & Compile-Time Execution

- **No Textual Macros**: All metaprogramming is accomplished with ordinary Juyu code running inside the compile-time VM.
- **Type Introspection**: `@typeInfo(T)` inspects structs, fields, functions, and alignments at compile-time.
- **Reified Types**: `@Type(info)` constructs new types programmatically at compile-time.
- **Guaranteed Loop Unrolling**: `inline for` and `inline while`.
- **Compile-Time Assets**: `@embedFile("assets/font.ttf")` embeds binary files directly into read-only binary segments.
- **Compile Diagnostics**: `@compileError("message")` and `@compileLog(...)`.

---

## 8. Concurrency & Synchronization

- **Execution Model**: Native OS threads with lightweight work-stealing thread pools (`std.Thread` and `std.Thread.Pool`).
- **Atomics**: C11 / C++20 memory orderings (`@atomicLoad`, `@atomicStore`, `@atomicRmw`, `@fence`).
- **Primitives**: `std.sync.Mutex`, `RwLock`, `ConditionVariable`, `Semaphore`, and `Once`.
- **Channels**: Strongly-typed message-passing channels (`std.sync.Channel(T)`).
- **Structured Scopes**: `std.Thread.scope` guarantees thread join before scope exit.
- **Async I/O**: High-performance, non-colored OS event loops (`epoll`, `kqueue`, `io_uring`, `IOCP`) in `std.event`.

---

## 9. Built-in Tooling & Ecosystem

- `juyu build [options]`: Compiles project according to `juyu.toml` and optional `build.ju`.
- `juyu test`: Discovers and executes first-class `test "name" { ... }` blocks across the codebase.
- `juyu fmt`: Opinionated canonical source code formatter with zero configuration flags.
- `juyu lsp`: Built-in Language Server Protocol implementation for IDE integration.
- `juyu doc`: Generates static, zero-JS HTML documentation from `///` and `//!` markdown comments.
