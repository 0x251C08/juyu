# The Complete Juyu Syntax Draft & Reference (Q1 – Q120)

This document synthesizes all 120 language design decisions into a cohesive, comprehensive syntax reference for Juyu.

---

## 1. Modules, Imports & Visibility (Q7, Q10, Q13, Q14, Q52, Q115)

### 1.1 Project Structure
- `juyu.toml`: Declarative package metadata and Git dependency declarations.
- `build.ju`: Optional executable Juyu build script for custom compilation graphs and native C/C++ targets.
- **Top-Level Order Independence**: All top-level declarations (functions, structs, constants) within a package can be referenced in any order without forward declarations or headers.

### 1.2 Module Imports & Exports
```juyu
//! Top-level module documentation comment.

// 1. Full namespace import (bound to an immutable identifier):
let math = import("std.math");
let io = import("std.io");

// 2. Selective item import (destructured directly into scope):
import { sin, cos, PI } from "std.math";

// 3. Embedded C/C++ header import:
let c = @cImport("stdio.h");
let raylib = @cImport("raylib.h");

// 4. Visibility: items are private by default; 'pub' exports them:
pub let DEFAULT_BUFFER_SIZE: usize = 4096;

pub fn calculate(): f64 {
    => sin(angle: PI / 2.0);
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
// Immutable binding (cannot be reassigned or mutated):
let max_connections: u32 = 1000;
let inferred_num = 42; // Inferred as i32

// Mutable binding:
var counter: i32 = 0;
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

### 2.3 Type Aliases & Distinct Newtypes
```juyu
// Transparent alias (pure synonym):
type Byte = u8;

// Distinct newtype (zero runtime cost, but prevents accidental substitution):
type Port = distinct u16;
type UserId = distinct u64;

let p: Port = @cast(Port, 8080); // Explicit cast required
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

// Unwrap via if-let:
if (let active = maybe_ptr) {
    active.* = 99; // 'active' is guaranteed *i32
} else {
    io.println(msg: "Pointer was null");
}

// Force unwrap (traps in safe modes if null):
// let force_val = maybe_ptr.?.*;

// Null-coalescing with default:
let default_num = 0;
let resolved_ptr: *const i32 = maybe_ptr ?? &default_num;

// 3. Slices (fat pointer: ptr + len, bounds-checked in safe modes):
let array: [4]i32 = [1, 2, 3, 4];
let slice: []const i32 = array[0..2];
let elem: i32 = slice[1]; // Bounds checked

// 4. Low-level Many-Item & C Pointers:
let raw_addr: usize = @ptrToInt(ptr);
let c_ptr: [*c]i32 = @ptrCast(ptr);   // Permissive C pointer allowing arithmetic
let many_ptr: [*]i32 = @ptrCast(ptr); // Allows pointer math: many_ptr + 1
```

### 2.5 Arrays, Sentinels, Strings & SIMD
```juyu
// Fixed-size array:
let fixed: [3]u32 = [10, 20, 30];

// Sentinel-terminated array & slice (zero-copy passing to C char*):
let c_string: [:0]const u8 = "hello\0";

// UTF-8 String slice:
let msg: []const u8 = "Hello, Juyu systems!";

// Multiline strings with automatic common indentation stripping:
let shader_code = """
    #version 450
    layout(location = 0) out vec4 outColor;
    void main() {
        outColor = vec4(1.0, 0.0, 0.0, 1.0);
    }
    """;

// SIMD Vector mapping directly to hardware SIMD registers:
let v1: @Vector(4, f32) = [1.0, 2.0, 3.0, 4.0];
let v2: @Vector(4, f32) = [5.0, 6.0, 7.0, 8.0];
let v3 = v1 + v2; // Single SIMD instruction
```

---

## 3. Functions, Arguments & Closures (Q48 – Q50, Q66 – Q75)

### 3.1 Function Declarations & Return Types
```juyu
// Uniform colon syntax for parameters and return types:
pub fn add(a: i32, b: i32): i32 {
    => a + b; // Arrow block return
}

// Void return type may be omitted for procedures:
pub fn log_message(msg: []const u8) {
    io.println(msg: msg);
}

// Concise single-expression function body shorthand:
pub fn square(x: f64): f64 => x * x;
pub fn is_even(n: i32): bool => n % 2 == 0;
```

### 3.2 Mandatory Callsite Argument Labels & Overloading
```juyu
// Swift-style function overloading distinguished by external argument labels:
pub fn move(to target: Point) {
    // ...
}

pub fn move(by delta: Point) {
    // ...
}

// Calling functions requires mandatory callsite argument labels:
move(to: Point { .x = 100.0, .y = 200.0 });
move(by: Point { .x = 10.0, .y = 5.0 });

let sum = add(a: 5, b: 10);
```

### 3.3 Tuples & Multiple Return Values
```juyu
// Functions returning labeled or unlabeled tuples:
pub fn get_dimensions(): (width: u32, height: u32) {
    => (width: 1920, height: 1080);
}

// Unpacking tuples at callsite:
let (w, h) = get_dimensions();
let dims = get_dimensions();
let width = dims.width; // Or dims.0
```

### 3.4 Unboxed Closures
```juyu
// Stack/struct unboxed closures (zero heap allocation, monomorphized):
let multiplier = 3;
let scale = |val: i32|: i32 => val * multiplier;

let result = scale(val: 10); // 30
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

extern fn puts(str: [*:0]const u8): i32 callconv(.C);

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
// Every statement terminates with a semicolon ';':
let computed = {
    let a = 10;
    let b = 20;
    => (a + b) * 2; // Yields value from block
};
```

### 4.2 Conditionals & Optional Chaining
```juyu
// If/else as an expression:
let status = if (score >= 50) "pass" else "fail";

// If/else with multiline blocks:
let grade = if (score >= 90) {
    io.println(msg: "Honors!");
    => 'A';
} else if (score >= 80) {
    => 'B';
} else {
    => 'F';
};

// Optional chaining '?.' and null coalescing '??':
let city = user?.profile?.address?.city ?? "Unknown";
```

### 4.3 Pattern Matching (`match`)
```juyu
// Exhaustive match expression with arrow arms:
let category = match (status_code) {
    200 => "OK",
    400...499 => "Client Error",
    500...599 => "Server Error",
    else => "Other",
};

// Tagged union variant unpacking:
let description = match (event) {
    .Click(let pos) => $"Clicked at ({pos.x}, {pos.y})",
    .KeyPress(let key) => $"Key pressed: {key}",
    .Quit => "Exiting",
};
```

### 4.4 Loops & Control Flow Labels
```juyu
// 1. Infinite loop:
loop {
    if (should_stop()) break;
}

// 2. While loop:
var i = 0;
while (i < 10) {
    i += 1;
}

// 3. Three-part C-style for loop:
for (var idx = 0; idx < items.len; idx += 1) {
    io.println_int(val: items[idx]);
}

// 4. Collection iteration:
for (item in items) {
    io.println(msg: item.name);
}

// 5. Direct identifier loop labels:
outer: for (var r = 0; r < 100; r += 1) {
    inner: for (var c = 0; c < 100; c += 1) {
        if (matrix[r][c] == target) {
            break outer;
        }
    }
}
```

### 4.5 Operators
- **Logical**: `&&`, `||`, `!`
- **Bitwise**: `&`, `|`, `^`, `~`, `<<`, `>>` (arithmetic on signed, logical on unsigned; traps if shift >= width)
- **Wrapping Arithmetic**: `+%`, `-%`, `*%`
- **Error Propagation**: `try expr`
- **Error Fallback**: `expr catch fallback`
- **Null-Coalescing**: `opt ?? fallback`

---

## 5. Memory Management & Resource Safety (Q36 – Q45)

### 5.1 Allocators & No Hidden Allocations
```juyu
let gpa = std.heap.GeneralPurposeAllocator.init();
defer gpa.deinit(); // Cleaned up when leaving block
let allocator = gpa.allocator();

// Allocating memory:
let slice = try allocator.alloc(u8, count: 1024);
defer allocator.free(slice: slice);

let node = try allocator.create(Node);
defer allocator.destroy(ptr: node);
```

### 5.2 Block-Scoped Cleanup (`defer` and `errdefer`)
```juyu
pub fn process_file(path: []const u8, alloc: Allocator): !void {
    let file = try std.fs.open(path: path);
    defer file.close(); // ALWAYS executed upon leaving scope (LIFO)

    let buffer = try alloc.alloc(u8, count: 4096);
    errdefer alloc.free(slice: buffer); // Executed ONLY if an error is returned below

    try file.read_all(dest: buffer);
    try parse_contents(buf: buffer);

    alloc.free(slice: buffer);
}
```

### 5.3 Destructive Move Semantics
```juyu
let file1 = try open_file(path: "log.txt");

// Destructive move: ownership is transferred to file2
let file2 = file1; 

// file1 is now invalid and inaccessible; compiler errors if referenced:
// file1.read(); // COMPILE ERROR: use of moved resource 'file1'
```

---

## 6. Structs, Enums, Interfaces & OOP (Q27, Q28, Q76 – Q85)

### 6.1 Struct Layouts & Designated Initializers
```juyu
// 1. Standard struct (compiler optimizes field ordering to eliminate padding):
pub struct Point {
    x: f32,
    y: f32,

    // Explicit 'static fn' for associated functions/constructors:
    pub static fn init(x: f32, y: f32): Point {
        => Point { .x = x, .y = y }; // Mandatory dot-prefixed designated initializers
    }

    // Instance method taking explicit receiver:
    pub fn distance_to(self: *const Point, other: Point): f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        => math.sqrt(val: dx * dx + dy * dy);
    }
}

// 2. Strict C ABI struct layout:
pub extern struct CHeader {
    magic: u32,
    version: u16,
}

// 3. Bit-packed struct for hardware registers:
pub packed struct StatusRegister {
    enabled: u1,
    mode: u3,
    reserved: u4,
}
```

### 6.2 External Struct Extensions
```juyu
// External modules can attach methods via 'extend':
extend Point {
    pub fn taxicab_norm(self: *const Point): f32 {
        => math.abs(val: self.x) + math.abs(val: self.y);
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

let p = Player {
    .Entity = Entity { .id = 1, .x = 0.0, .y = 0.0 },
    .inventory_count = 10,
};

// Automatic promotion allows direct access:
let player_id = p.id; // Accesses p.Entity.id directly
```

### 6.4 Enums & Tagged Unions
```juyu
// Enum with explicit backing integer type:
pub enum(u8) FileMode {
    ReadOnly = 0x01,
    WriteOnly = 0x02,
    ReadWrite = 0x03,
}

// Safe tagged union (sum type):
pub union(enum) Payload {
    Text: []const u8,
    Number: i64,
    Coord: Point,
}
```

### 6.5 Structural Interfaces & Dynamic Dispatch
```juyu
// Structural interface (satisfied automatically without 'impl'):
pub interface Writer {
    fn write(mut self, bytes: []const u8): !usize;
}

// Static polymorphism (monomorphized with zero runtime cost):
pub fn print_data(w: Writer, data: []const u8): !void {
    try w.write(bytes: data);
}

// Dynamic dispatch via explicit fat pointer:
pub fn log_to_output(writer: *dyn Writer, msg: []const u8): !void {
    try writer.write(bytes: msg);
}
```

---

## 7. Metaprogramming & Compile-Time Directives (Q8, Q15, Q103 – Q108)

### 7.1 Comptime Type Generics
```juyu
// Generics are regular functions taking 'comptime T: type' and returning 'type':
pub fn List(comptime T: type): type {
    => struct {
        items: []T,
        capacity: usize,

        pub fn get(self: *const Self, index: usize): T {
            => self.items[index];
        }
    };
}

let IntList = List(comptime T: i32);
```

### 7.2 Introspection & Builtins
```juyu
// Compile-time assertions:
@compileError("Unsupported target architecture");

// Embed raw binary files into executable data section:
let font_bytes: []const u8 = @embedFile("assets/font.ttf");

// Type inspection:
let size = @sizeOf(Point);
let align = @alignOf(Point);
let info = @typeInfo(Point);

// Guaranteed loop unrolling at compile-time:
inline for (fields in @typeInfo(Point).Struct.fields) {
    // Emits unrolled code per field
}
```

---

## 8. Built-in Testing & Program Entry (Q19, Q20)

```juyu
// Standard entry point:
pub fn main(): !void {
    let args = std.process.args();
    io.println(msg: "Welcome to Juyu!");
}

// Built-in test block (stripped in production builds, executed via 'juyu test'):
test "verify point distance calculation" {
    let p1 = Point { .x = 0.0, .y = 0.0 };
    let p2 = Point { .x = 3.0, .y = 4.0 };
    @assert(p1.distance_to(other: p2) == 5.0);
}
```
