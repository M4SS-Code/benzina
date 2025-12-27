use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};

use crate::{U15, U31, U63};

macro_rules! impl_schemars_numbers {
    ($($type:ident => $format:literal),*) => {
        $(
            impl JsonSchema for $type {
                fn schema_name() -> Cow<'static, str> {
                    stringify!($type).into()
                }

                fn json_schema(_: &mut SchemaGenerator) -> Schema {
                    json_schema!({
                        "type": "integer",
                        "format": $format,
                        "minimum": $type::MIN.get(),
                        "maximum": $type::MAX.get()
                    })
                }
            }
        )*
    }
}

impl_schemars_numbers! {
    U15 => "int16",
    U31 => "int32",
    U63 => "int64"
}
