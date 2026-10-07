//! Bootstrap stdlib declarations and their C implementations live together.
//! Sema registers these signatures in its ordinary function namespace; the
//! backend sees resolved call targets, not special source-text patterns.
use crate::sema::Type;

pub(crate) struct RuntimeFunction {
    pub path: &'static str,
    pub parameters: &'static [(&'static str, Type)],
    pub result: Type,
    pub c_name: &'static str,
    pub c_source: &'static str,
}
pub(crate) const FUNCTIONS: &[RuntimeFunction] = &[
    RuntimeFunction {
        path: "std.io.println",
        parameters: &[("msg", Type::Str)],
        result: Type::Void,
        c_name: "juyu_io_println",
        c_source: include_str!("runtime/println.c"),
    },
    RuntimeFunction {
        path: "std.io.printlnInt",
        parameters: &[("val", Type::I64)],
        result: Type::Void,
        c_name: "juyu_io_println_int",
        c_source: include_str!("runtime/println_int.c"),
    },
];
