use std::borrow::Cow;

use utoipa::{
    PartialSchema, ToSchema,
    openapi::{KnownFormat, ObjectBuilder, RefOr, Schema, SchemaFormat, Type, schema::SchemaType},
};

use crate::{U15, U31, U63};

macro_rules! impl_utoipa_numbers {
    ($($type:ident => $known_format:ident),*) => {
        $(
            impl PartialSchema for $type {
                fn schema() -> RefOr<Schema> {
                    RefOr::T(Schema::Object(
                        ObjectBuilder::new()
                            .schema_type(SchemaType::new(Type::Integer))
                            .minimum(Some($type::MIN.get()))
                            .maximum(Some($type::MAX.get()))
                            .format(Some(SchemaFormat::KnownFormat(KnownFormat::$known_format)))
                            .build(),
                    ))
                }
            }

            impl ToSchema for $type {
                fn name() -> Cow<'static, str> {
                    Cow::Borrowed(stringify!($type))
                }
            }
        )*
    }
}

impl_utoipa_numbers! {
    U15 => Int32,
    U31 => Int32,
    U63 => Int64
}
