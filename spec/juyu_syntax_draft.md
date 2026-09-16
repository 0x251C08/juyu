# The Complete Juyu Syntax Draft & Reference (Q1 – Q120)

This document synthesizes all 120 language design decisions into a cohesive, comprehensive syntax reference for Juyu, updated with the new streamlined syntax rules.

---

## 1. Modules, Imports & Visibility (Q7, Q10, Q13, Q14, Q52, Q115)

### 1.1 Project Structure
- `juyu.toml`: Declarative package metadata and Git dependency declarations.
- `build.ju`: Optional executable Juyu build script for custom compilation graphs and native C/C++ targets.
- **Top-Level Order Independence**: All top-level declarations (functions, structs, constants) within a package can be referenced in any order without forward declarations or headers.

### 1.2 Module Imports & Exports
```juyu
//! Top-level module documentation comment.

// 1. Module imports (brackets for multiple, parentheses for single):
import [std.io, std.math];
import (std.fs);

// 2. Embedded C/C++ header import:
let c := @cImport("stdio.h");
let raylib := @cImport("raylib.h");

// 3. Visibility: items are private by default; 'export' exports them:
export let DEFAULT_BUFFER_SIZE: usize = 4096;

export fn calculate(): f64 {
    => std.math.sin(std.math.PI / 2.0);
}

// Private helper function:
fn internal_helper() {
    // ...
}
```

---

## 2. Variables, Mutability & Types (Q21 – Q25, Q27 – Q30, Q35)

### 2.1 Bindings & Mutability
```juyu
// Immutable binding with explicit type:
let max_connections: u32 = 1000;

// Type inference requires explicit ':=':
let inferred_num := 42; // Inferred as i32
let language := "Juyu"; // Inferred as &[u8]

// Mutable binding:
var counter: i32 = 0;
var inferred_counter := 0; // Inferred as i32
counter += 1;

// Mandatory explicit initialization or '= undefined' with debug poisoning:
var buffer: [1024]u8 = undefined; // Poisoned with 0xAA in Debug/ReleaseSafe
```

### 2.2 Scalar Types & Custom Bit-Widths
```juyu
// Standard fixed-width types:
let a: i8 = -12;
let b: u8 = 255;
let c: i32 = -100_000;
let d: u64 = 1_000_000;
let e: usize = 64;
let f: isize = -64;
let g: f32 = 3.14159;
let h: f64 = 2.718281828459;
let i: f128 = 1.0;
let ok: bool = true;

// Arbitrary custom bit-widths (for neural-net quantization and hardware registers):
let flag: u1 = 1;        // Single bit
let ternary: u2 = 3;     // 2-bit weight
let nibble: u4 = 0xF;    // 4-bit quantized weight
let ascii_char: u7 = 65; // 7-bit ASCII
let audio_sample: i24 = 0x7FFFFF; // 24-bit PCM audio
```

### 2.3 Type Aliases, Distinct Newtypes & Casting
```juyu
// Transparent alias (pure synonym):
type Byte = u8;

// Distinct newtype (zero runtime cost, but prevents accidental substitution):
type Port = distinct u16;
type UserId = distinct u64;

// Type casting using Rust-style 'val as T':
let p: Port = 8080 as Port; // Explicit cast required
let raw_port: u16 = p as u16;
let big_num: i64 = 42 as i64;
```

### 2.4 Pointers, Slices & Nullability
```juyu
let val: i32 = 42;

// 1. Safe, non-null single-item pointer:
let ptr: *const i32 = &val;
var mut_val: i32 = 10;
let mut_ptr: *i32 = &mut_val;

// Explicit dereference via postfix '.*':
mut_ptr.* += 1;

// 2. Zero-cost nullable pointer (?*T):
var maybe_ptr: ?*i32 = null;

// Swift-style optional unwrapping via 'if (let x = opt)':
if (let active = maybe_ptr) {
    active.* = 99; // 'active' is guaranteed *i32
} else {
    std.io.println("Pointer was null");
}

// Null-coalescing '??' with default fallback:
let default_num := 0;
let resolved_ptr: *const i32 = maybe_ptr ?? &default_num;

// 3. Slices (Rust-style fat pointer '&[T]' / '&mut [T]': ptr + len, bounds-checked):
let array: [4]i32 = [1, 2, 3, 4];
let slice: &[i32] = &array[0..2];
let elem: i32 = slice[1]; // Bounds checked

// 4. Low-level Many-Item & C-FFI Pointers:
let raw_addr: usize = ptr as usize;
let c_ptr: [*c]i32 = ptr as [*c]i32;   // Permissive C pointer allowing arithmetic
let many_ptr: [*]i32 = ptr as [*]i32; // Allows pointer math: many_ptr + 1
```

### 2.5 Arrays, Sentinels, Strings & SIMD
```juyu
// Fixed-size array:
let fixed: [3]u32 = [10, 20, 30];

// Sentinel-terminated array & slice (zero-copy passing to C char*):
let c_string: [:0]u8 = "hello\0";

// UTF-8 String slice (&[u8]):
let language: &[u8] = "Juyu";
let version: f32 = 0.1;

// String interpolation with '$"..."':
let greeting := $"Hello, systems programming from {language} v{version}!";

// Multiline strings with '"""' (automatic common indentation stripping):
let shader_code := """
    #version 450
    layout(location = 0) out vec4 outColor;
    void main() {
        outColor = vec4(1.0, 0.0, 0.0, 1.0);
    }
    """;

// SIMD Vector mapping directly to hardware SIMD registers:
let v1: @Vector(4, f32) = [1.0, 2.0, 3.0, 4.0];
let v2: @Vector(4, f32) = [5.0, 6.0, 7.0, 8.0];
let v3 := v1 + v2; // Single SIMD instruction
```

---

## 3. Functions, Arguments & Closures (Q48 – Q50, Q66 – Q75)

### 3.1 Function Declarations, Return Types & Error Unions
```juyu
// Uniform colon syntax for parameters and return types:
export fn add(a: i32, b: i32): i32 {
    => a + b; // Arrow block return '=> val;'
}

// Error union return type '!T' (or '!void' for fallible procedures):
export fn open_config(path: &[u8]): !File {
    let file := std.fs.open(path)!; // Postfix '!' error propagation
    => file;
}

// Void return type may be omitted for procedures:
export fn log_message(msg: &[u8]) {
    std.io.println(msg);
}

// Concise single-expression function body shorthand with '=>':
export fn square(x: f64): f64 => x * x;
export fn is_even(n: i32): bool => n % 2 == 0;
```

### 3.2 Parameter Labels & Callsite Ergonomics
```juyu
// Parameter labels are optional, but allowed for clarity or overloading:
export fn move(to target: Point) {
    // ...
}

export fn move(by delta: Point) {
    // ...
}

// Calling functions: argument labels are optional:
move(to: Point { x: 100.0, y: 200.0 });
move(Point { x: 10.0, y: 5.0 });

let sum1 := add(10, 20);
let sum2 := add(a: 10, b: 20);
```

### 3.3 Tuples & Multiple Return Values
```juyu
// Functions returning labeled or unlabeled tuples:
export fn get_dimensions(): (width: u32, height: u32) {
    => (width: 1920, height: 1080);
}

// Unpacking tuples at callsite:
let (w, h) := get_dimensions();
let dims := get_dimensions();
let width := dims.width; // Or dims.0
```

### 3.4 Unboxed Closures & Trailing Closures
```juyu
// Stack/struct unboxed closures (zero heap allocation, monomorphized):
let multiplier := 3;
let scale := |val: i32|: i32 => val * multiplier;

let result := scale(10); // 30

// Trailing closures (Swift/Kotlin style syntax for higher-order functions):
let numbers := [1, 2, 3, 4, 5];
numbers.each() |item| {
    std.io.printlnInt(item as i64);
};

let doubled := numbers.map() |x| => x * 2;
```

### 3.5 Function Modifiers
```juyu
// Inlining control:
inline fn fast_add(a: i32, b: i32): i32 => a + b;
noinline fn cold_error_path(code: u32) { ... }

// Linkage & FFI:
export fn juyu_api_entry(code: i32): i32 callconv(.C) {
    => code * 2;
}

extern fn puts(str: [*c]const u8): i32 callconv(.C);

// Pure compile-time / runtime function:
const fn compute_lookup_table(size: usize): usize => size * 4;

// Naked assembly functions (no prologue/epilogue):
naked fn reset_handler() {
    // raw assembly instructions only
}
```

---

## 4. Control Flow & Expressions (Q46, Q47, Q56 – Q65)

### 4.1 Semicolons & Arrow Block Yields
```juyu
// Every statement terminates with a mandatory semicolon ';':
let computed := {
    let a := 10;
    let b := 20;
    => (a + b) * 2; // Yields value from block via '=>'
};
```

### 4.2 Conditionals, Optional Unwrapping & Coalescing
```juyu
// If/else as an expression:
let status := if (score >= 50) "pass" else "fail";

// If/else with multiline blocks and arrow yields:
let grade := if (score >= 90) {
    std.io.println("Honors!");
    => 'A';
} else if (score >= 80) {
    => 'B';
} else {
    => 'F';
};

// Swift-style optional binding:
if (let active = maybe_ptr) {
    std.io.printlnInt(active.* as i64);
}

// Optional chaining '?.' and null coalescing '??':
let city := user?.profile?.address?.city ?? "Unknown";
```

### 4.3 Pattern Matching (`match`)
```juyu
// Exhaustive match expression with arrow arms:
let category := match (status_code) {
    200 => "OK",
    400..499 => "Client Error",
    500..599 => "Server Error",
    _ => "Other",
};

// Tagged union variant unpacking:
let description := match (event) {
    .Click(let pos) => $"Clicked at ({pos.x}, {pos.y})",
    .KeyPress(let key) => $"Key pressed: {key}",
    .Quit => "Exiting",
};
```

### 4.4 Loops & Loop Labels (`@outer`)
```juyu
// 1. Infinite loop:
loop {
    if (should_stop()) break;
}

// 2. While loop:
var i := 0;
while (i < 10) {
    i += 1;
}

// 3. Three-part C-style for loop:
for (var idx := 0; idx < items.len; idx += 1) {
    std.io.printlnInt(items[idx] as i64);
}

// 4. Collection iteration:
for (item in items) {
    std.io.println(item.name);
}

// 5. Loop labels with '@label' syntax:
@outer: for (var r := 0; r < 100; r += 1) {
    @inner: for (var c := 0; c < 100; c += 1) {
        if (matrix[r][c] == target) {
            break @outer;
        }
    }
}
```

### 4.5 Operators
- **Logical**: `&&`, `||`, `!`
- **Bitwise**: `bitand`, `bitor`, `bitxor`, `~`, `<<`, `>>`
  ```juyu
  let flags := a bitand b;
  let mask := c bitor d;
  let toggled := e bitxor f;
  ```
- **Wrapping Arithmetic**: `+%`, `-%`, `*%`
  ```juyu
  let wrapped := max_val +% 1;
  ```
- **Type Casting**: `val as T` (e.g., `sum as i64`)
- **Error Propagation**: Postfix `expr!` (e.g., `let file = std.fs.open(path)!;`)
- **Error Unions**: `!T` (e.g., `!void`, `!File`)
- **Null-Coalescing**: `opt ?? fallback`

---

## 5. Memory Management & Resource Safety (Q36 – Q45)

### 5.1 Allocators & No Hidden Allocations
```juyu
// Memory management is explicit via allocator parameters:
var gpa = std.heap.GeneralPurposeAllocator.init();
defer {
    if (!gpa.deinit()) {
        std.io.println("Memory leak detected!");
    }
};
let allocator := gpa.allocator();

// Allocating memory using explicit allocator:
let slice: &[u8] = allocator.alloc(u8, 1024)!;
defer allocator.free(slice);

let node: *Node = allocator.create(Node)!;
defer allocator.destroy(node);
```

### 5.2 Block-Scoped Cleanup (`defer` and `errdefer`)
```juyu
export fn process_file(path: &[u8], alloc: std.mem.Allocator): !void {
    let file := std.fs.open(path)!;
    defer file.close(); // ALWAYS executed upon leaving scope (LIFO)

    let buffer := alloc.alloc(u8, 4096)!;
    errdefer alloc.free(buffer); // Executed ONLY if an error is returned below

    file.read_all(buffer)!;
    parse_contents(buffer)!;

    alloc.free(buffer);
}
```

### 5.3 Destructive Move Semantics
```juyu
let file1 := open_file("log.txt")!;

// Destructive move: ownership is transferred to file2
let file2 := file1;

// file1 is now invalid and inaccessible; compiler errors if referenced:
// file1.read(); // COMPILE ERROR: use of moved resource 'file1'
```

---

## 6. Structs, Enums, Interfaces & OOP (Q27, Q28, Q76 – Q85)

### 6.1 Struct Layouts, Instantiation & Method Receivers
```juyu
// 1. Standard struct:
export struct Point {
    x: f32,
    y: f32,

    // Associated function / constructor:
    export static fn init(x: f32, y: f32): Point {
        => Point { x: x, y: y }; // Struct instantiation: Rust/JS-style without dot prefix
    }

    // Method receiver with '&self' (immutable borrow):
    export fn distance_to(&self, other: Point): f32 {
        let dx := self.x - other.x;
        let dy := self.y - other.y;
        => std.math.sqrt(dx * dx + dy * dy);
    }

    // Method receiver with '&mut self' (mutable borrow):
    export fn translate(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
    }
}

// 2. Strict C ABI struct layout:
export extern struct CHeader {
    magic: u32,
    version: u16,
}

// 3. Bit-packed struct for hardware registers:
export packed struct StatusRegister {
    enabled: u1,
    mode: u3,
    reserved: u4,
}
```

### 6.2 External Struct Extensions
```juyu
// External modules can attach methods via 'extend':
extend Point {
    export fn taxicab_norm(&self): f32 {
        => std.math.abs(self.x) + std.math.abs(self.y);
    }
}
```

### 6.3 Struct Composition & Field Promotion
```juyu
struct Entity {
    id: u64,
    x: f32,
    y: f32,
}

struct Player {
    Entity, // Anonymous embedding: promotes all fields and methods
    inventory_count: u32,
}

let p := Player {
    Entity: Entity { id: 1, x: 0.0, y: 0.0 },
    inventory_count: 10,
};

// Automatic promotion allows direct access:
let player_id := p.id; // Accesses p.Entity.id directly
```

### 6.4 Enums & Tagged Unions
```juyu
// Enum with explicit backing integer type:
export enum(u8) FileMode {
    ReadOnly = 0x01,
    WriteOnly = 0x02,
    ReadWrite = 0x03,
}

// Safe tagged union (sum type):
export union(enum) Payload {
    Text: &[u8],
    Number: i64,
    Coord: Point,
}

// Inferred enum variants:
let mode: FileMode = .ReadOnly;
```

### 6.5 Structural Interfaces & Dynamic Dispatch (`any Trait`)
```juyu
// Structural interface (satisfied automatically without 'impl'):
export interface Writer {
    fn write(&mut self, bytes: &[u8]): !usize;
}

// Static polymorphism (monomorphized with zero runtime cost):
export fn print_data(w: any Writer, data: &[u8]): !void {
    w.write(data)!;
}

// Dynamic dispatch using Swift-style 'any Trait' existential:
export fn log_to_output(writer: &mut any Writer, msg: &[u8]): !void {
    writer.write(msg)!;
}
```

---

## 7. Metaprogramming & Compile-Time Directives (Q8, Q15, Q103 – Q108)

### 7.1 Generic Types (`List<T>`)
```juyu
// Generics use angle bracket syntax '<T>':
export struct List<T> {
    items: &[T],
    capacity: usize,

    export fn get(&self, index: usize): T {
        => self.items[index];
    }
}

export fn identity<T>(val: T): T => val;

let int_list := List<i32> { items: &[], capacity: 0 };
let str := identity<String>("hello");
```

### 7.2 Introspection & Builtins (`@sizeOf`, `@assert`, etc.)
```juyu
// Compile-time assertions and errors:
@compileError("Unsupported target architecture");

// Builtin type inspection:
let size := @sizeOf(Point);
let align := @alignOf(Point);
let info := @typeInfo(Point);

// Embed raw binary files into executable data section:
let font_bytes: &[u8] = @embedFile("assets/font.ttf");

// Guaranteed loop unrolling at compile-time:
inline for (fields in @typeInfo(Point).Struct.fields) {
    // Emits unrolled code per field
}
```

---

## 8. Built-in Testing & Program Entry (Q19, Q20)

```juyu
// Standard entry point:
export fn main(allocator: std.mem.Allocator): !void {
    let args := std.process.args();
    std.io.println("Welcome to Juyu!");
}

// Built-in test block (stripped in production builds, executed via 'juyu test'):
test "verify point distance calculation" {
    let p1 := Point { x: 0.0, y: 0.0 };
    let p2 := Point { x: 3.0, y: 4.0 };
    @assert(p1.distance_to(p2) == 5.0);
}
```
