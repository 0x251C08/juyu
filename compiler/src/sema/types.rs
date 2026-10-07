use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Void,
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Str,
}

impl Type {
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "void" => Self::Void,
            "bool" => Self::Bool,
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "str" => Self::Str,
            _ => return None,
        })
    }
    pub fn bounds(self) -> Option<(i128, i128)> {
        Some(match self {
            Self::I8 => (i8::MIN as i128, i8::MAX as i128),
            Self::I16 => (i16::MIN as i128, i16::MAX as i128),
            Self::I32 => (i32::MIN as i128, i32::MAX as i128),
            Self::I64 => (i64::MIN as i128, i64::MAX as i128),
            Self::U8 => (0, u8::MAX as i128),
            Self::U16 => (0, u16::MAX as i128),
            Self::U32 => (0, u32::MAX as i128),
            Self::U64 => (0, u64::MAX as i128),
            _ => return None,
        })
    }
    pub fn is_integer(self) -> bool {
        self.bounds().is_some()
    }
    pub fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }
    pub fn is_numeric(self) -> bool {
        self.is_integer() || self.is_float()
    }
    pub fn is_signed(self) -> bool {
        matches!(self, Self::I8 | Self::I16 | Self::I32 | Self::I64)
    }
}
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}
