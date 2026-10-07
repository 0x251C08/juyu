//! Checked scalar operations avoid undefined behavior and silent narrowing in C.
use super::{c_type, operator, Type, B};

pub(super) fn integer(n: i128) -> String {
    if n == i64::MIN as i128 {
        "(-INT64_C(9223372036854775807) - INT64_C(1))".into()
    } else if n < 0 {
        format!("(-INT64_C({}))", -n)
    } else {
        format!("UINT64_C({n})")
    }
}

pub(super) fn arithmetic(ty: Type, op: B) -> (String, String) {
    let operation = match op {
        B::Add => "add",
        B::Sub => "sub",
        B::Mul => "mul",
        B::Div => "div",
        B::Mod => "mod",
        _ => unreachable!(),
    };
    let name = format!("juyu_{operation}_{ty}");
    let t = c_type(ty);
    let mut body = format!("static {t} {name}({t} a, {t} b) {{\n");
    if matches!(op, B::Add | B::Sub | B::Mul) {
        body.push_str(&format!("    {t} result;\n    if (__builtin_{operation}_overflow(a, b, &result)) abort();\n    return result;\n"));
    } else {
        body.push_str("    if (b == 0) abort();\n");
        if ty.is_signed() {
            body.push_str(&format!(
                "    if (a == {} && b == -1) abort();\n",
                integer(ty.bounds().unwrap().0)
            ));
        }
        body.push_str(&format!("    return ({t})(a {} b);\n", operator(op)));
    }
    body.push_str("}\n");
    (name, body)
}

pub(super) fn cast(from: Type, to: Type) -> (String, String) {
    let name = format!("juyu_cast_{from}_{to}");
    let source = c_type(from);
    let target = c_type(to);
    let mut body = format!("static {target} {name}({source} value) {{\n");
    if let Some((min, max)) = to.bounds() {
        if from.is_integer() {
            // All supported integers, including u64, fit in signed __int128.
            body.push_str(&format!("    __int128 wide = (__int128)value;\n    if (wide < (__int128){} || wide > (__int128){}) abort();\n", integer(min), integer(max)));
        } else {
            let bits = (max as u128 + 1).trailing_zeros();
            let lower = if min == 0 {
                "0.0L".into()
            } else {
                format!("-0x1p{bits}L")
            };
            // Exclusive power-of-two upper bounds stay exact even when long
            // double cannot represent the largest integer below that bound.
            body.push_str(&format!("    long double wide = (long double)value;\n    if (!(wide >= {lower} && wide < 0x1p{bits}L)) abort();\n"));
        }
    } else if from == Type::F64 && to == Type::F32 {
        body.push_str(
            "    if (isfinite(value) && (value > FLT_MAX || value < -FLT_MAX)) abort();\n",
        );
    }
    body.push_str(&format!("    return ({target})value;\n}}\n"));
    (name, body)
}
