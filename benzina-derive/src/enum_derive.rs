use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::{
    Data, DeriveInput, Fields, Ident, LitByteStr, LitStr, Path, Token, Type, spanned::Spanned,
};

use crate::rename_rule::RenameRule;

macro_rules! fail {
    ($t:expr, $m:expr) => {
        return Err(syn::Error::new_spanned($t, $m))
    };
}

macro_rules! try_set {
    ($i:ident, $v:expr, $t:expr) => {
        match $i {
            Some(_) => fail!($t, "duplicate attribute"),
            None => $i = Some($v),
        }
    };
}

pub(crate) struct Enum {
    ident: Ident,
    sql_type: Type,
    rename_all: RenameRule,
    variants: Vec<EnumVariant>,

    crate_name: Option<Path>,
}

struct EnumVariant {
    original_name: String,
    original_name_span: Span,
    rename: Option<String>,
    #[cfg(all(feature = "postgres", feature = "json"))]
    payload: Option<EnumVariantPayload>,

    crate_name: Option<Path>,
}

#[cfg(all(feature = "postgres", feature = "json"))]
struct EnumVariantPayload {
    type_: Type,
    span: Span,
}

impl Enum {
    #[expect(clippy::too_many_lines)]
    pub(crate) fn parse(input: DeriveInput) -> Result<Self, syn::Error> {
        let Data::Enum(e) = input.data else {
            fail!(input, "`benzina::Enum` macro available only for enums");
        };

        let (rename_all, sql_type, crate_name) = {
            let mut first_attr = None;
            let mut sql_type = None;
            let mut rename_all = None;
            let mut crate_name = None;

            for attr in input
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("benzina"))
            {
                first_attr.get_or_insert(attr);

                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("sql_type") {
                        meta.input.parse::<Token![=]>()?;
                        let val: Type = meta.input.parse()?;
                        try_set!(sql_type, val, val);
                    } else if meta.path.is_ident("rename_all") {
                        meta.input.parse::<Token![=]>()?;
                        let val: LitStr = meta.input.parse()?;
                        try_set!(
                            rename_all,
                            val.value()
                                .parse()
                                .map_err(|err| syn::Error::new_spanned(val, err))?,
                            val
                        );
                    } else if meta.path.is_ident("crate") {
                        meta.input.parse::<Token![=]>()?;
                        let val: Path = meta.input.parse()?;
                        try_set!(crate_name, val, val);
                    }

                    Ok(())
                })?;
            }

            let Some(first_attr) = first_attr else {
                fail!(e.enum_token, "expected #[benzina(...)] attribute");
            };

            let Some(sql_type) = sql_type else {
                fail!(first_attr, "expected `sql_type`");
            };

            (rename_all.unwrap_or(RenameRule::None), sql_type, crate_name)
        };

        let variants = e
            .variants
            .into_iter()
            .map(|variant| {
                let payload = match &variant.fields {
                    Fields::Unit => None,
                    #[cfg(all(feature = "postgres", feature = "json"))]
                    Fields::Unnamed(fields) => {
                        let mut fields = fields.unnamed.iter();
                        let (Some(field), None) = (fields.next(), fields.next()) else {
                            fail!(variant, "only single-item variants are supported");
                        };

                        let span = field.span();
                        Some(EnumVariantPayload {
                            type_: field.ty.clone(),
                            span,
                        })
                    }
                    #[cfg(not(all(feature = "postgres", feature = "json")))]
                    Fields::Unnamed(_fields) => {
                        fail!(variant, "fields require both the `postgres` and the `json` feature to be enabled");
                    }
                    Fields::Named(_fields) => {
                        fail!(variant, "only unit an unnamed variants are supported");
                    }
                };

                let name = variant.ident.to_string();
                let mut rename = None;

                for attr in variant
                    .attrs
                    .iter()
                    .filter(|attr| attr.path().is_ident("benzina"))
                {
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("rename") {
                            meta.input.parse::<Token![=]>()?;
                            let val: LitStr = meta.input.parse()?;
                            try_set!(rename, val.value(), val);
                        }

                        Ok(())
                    })?;
                }

                // Suppress build breakage when building without the
                // PostgreSQL JSON feature.
                #[cfg(not(all(feature = "postgres", feature = "json")))]
                let _: Option<()> = payload;

                let original_name_span = variant.span();
                Ok(EnumVariant {
                    original_name: name,
                    original_name_span,
                    rename,
                    #[cfg(all(feature = "postgres", feature = "json"))]
                    payload,

                    crate_name: crate_name.clone(),
                })
            })
            .collect::<Result<Vec<_>, syn::Error>>()?;
        Ok(Self {
            ident: input.ident,
            sql_type,
            rename_all,
            variants,

            crate_name,
        })
    }

    #[cfg(all(feature = "postgres", feature = "json"))]
    fn has_json_fields(&self) -> bool {
        self.variants
            .iter()
            .any(|variant| variant.payload.is_some())
    }

    #[cfg(not(all(feature = "postgres", feature = "json")))]
    #[expect(
        clippy::unused_self,
        reason = "kept for compatibility with the above implementation"
    )]
    fn has_json_fields(&self) -> bool {
        false
    }
}

impl ToTokens for Enum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            ident,
            sql_type,
            rename_all,
            variants,

            crate_name,
        } = &self;
        let crate_name = crate::crate_name(crate_name);

        let from_bytes_arms = variants
            .iter()
            .map(|variant| variant.gen_from_bytes(self.has_json_fields(), *rename_all));
        let to_str_arms = variants
            .iter()
            .map(|variant| variant.gen_to_str(self.has_json_fields(), *rename_all));

        if !self.has_json_fields() {
            tokens.append_all(quote! {
                impl #ident {
                    #[doc(hidden)]
                    fn __benzina04_from_bytes(val: &[u8]) -> #crate_name::__private::std::option::Option<Self> {
                        match val {
                            #(#from_bytes_arms)*
                            _ => #crate_name::__private::std::option::Option::None,
                        }
                    }

                    #[doc(hidden)]
                    fn __benzina04_as_str(&self) -> &'static str {
                        match self {
                            #(#to_str_arms)*
                        }
                    }
                }
            });
        }

        #[cfg(feature = "postgres")]
        if self.has_json_fields() {
            #[cfg(all(feature = "postgres", feature = "json"))]
            {
                tokens.append_all(quote! {
                    #[automatically_derived]
                    impl #crate_name::__private::diesel::deserialize::FromSql<(#sql_type, #crate_name::__private::diesel::pg::sql_types::Jsonb), #crate_name::__private::diesel::pg::Pg> for #ident {
                        fn from_sql(bytes: #crate_name::__private::diesel::pg::PgValue<'_>) -> #crate_name::__private::diesel::deserialize::Result<Self> {
                            match Self::__benzina04_from_bytes(bytes.as_bytes()) {
                                #crate_name::__private::std::option::Option::Some(this) => {
                                    #crate_name::__private::std::result::Result::Ok(this)
                                },
                                #crate_name::__private::std::option::Option::None => {
                                    #crate_name::__private::std::result::Result::Err(
                                        #crate_name::__private::std::convert::Into::into(
                                            "Unrecognized enum variant"
                                        )
                                    )
                                },
                            }
                        }
                    }

                    #[automatically_derived]
                    impl #crate_name::__private::diesel::serialize::ToSql<(#sql_type, #crate_name::__private::diesel::pg::sql_types::Jsonb), #crate_name::__private::diesel::pg::Pg> for #ident {
                        fn to_sql<'b>(&'b self, out: &mut #crate_name::__private::diesel::serialize::Output<'b, '_, #crate_name::__private::diesel::pg::Pg>) -> #crate_name::__private::diesel::serialize::Result {
                            let sql_val = self.__benzina04_as_str();
                            #crate_name::__private::std::io::Write::write_all(out, sql_val.as_bytes())?;

                            #crate_name::__private::std::result::Result::Ok(
                                #crate_name::__private::diesel::serialize::IsNull::No
                            )
                        }
                    }
                });
            }

            #[cfg(not(all(feature = "postgres", feature = "json")))]
            unreachable!()
        } else {
            tokens.append_all(quote! {
                #[automatically_derived]
                impl #crate_name::__private::diesel::deserialize::FromSql<#sql_type, #crate_name::__private::diesel::pg::Pg> for #ident {
                    fn from_sql(bytes: #crate_name::__private::diesel::pg::PgValue<'_>) -> #crate_name::__private::diesel::deserialize::Result<Self> {
                        match Self::__benzina04_from_bytes(bytes.as_bytes()) {
                            #crate_name::__private::std::option::Option::Some(this) => {
                                #crate_name::__private::std::result::Result::Ok(this)
                            },
                            #crate_name::__private::std::option::Option::None => {
                                #crate_name::__private::std::result::Result::Err(
                                    #crate_name::__private::std::convert::Into::into(
                                        "Unrecognized enum variant"
                                    )
                                )
                            },
                        }
                    }
                }

                #[automatically_derived]
                impl #crate_name::__private::diesel::serialize::ToSql<#sql_type, #crate_name::__private::diesel::pg::Pg> for #ident {
                    fn to_sql<'b>(&'b self, out: &mut #crate_name::__private::diesel::serialize::Output<'b, '_, #crate_name::__private::diesel::pg::Pg>) -> #crate_name::__private::diesel::serialize::Result {
                        let sql_val = self.__benzina04_as_str();
                        #crate_name::__private::std::io::Write::write_all(out, sql_val.as_bytes())?;

                        #crate_name::__private::std::result::Result::Ok(
                            #crate_name::__private::diesel::serialize::IsNull::No
                        )
                    }
                }
            });
        }

        #[cfg(feature = "mysql")]
        if !self.has_json_fields() {
            tokens.append_all(quote! {
                #[automatically_derived]
                impl #crate_name::"mysql"__private::diesel::deserialize::FromSql<#sql_type, #crate_name::__private::diesel::mysql::Mysql> for #ident {
                    fn from_sql(bytes: #crate_name::__private::diesel::mysql::MysqlValue<'_>) -> #crate_name::__private::diesel::deserialize::Result<Self> {
                        match Self::__benzina04_from_bytes(bytes.as_bytes()) {
                            #crate_name::__private::std::option::Option::Some(this) => {
                                #crate_name::__private::std::result::Result::Ok(this)
                            },
                            #crate_name::__private::std::option::Option::None => {
                                #crate_name::__private::std::result::Result::Err(
                                    #crate_name::__private::std::convert::Into::into(
                                        "Unrecognized enum variant"
                                    )
                                )
                            },
                        }
                    }
                }

                #[automatically_derived]
                impl #crate_name::__private::diesel::serialize::ToSql<#sql_type, #crate_name::__private::diesel::mysql::Mysql> for #ident {
                    fn to_sql<'b>(&'b self, out: &mut #crate_name::__private::diesel::serialize::Output<'b, '_, #crate_name::__private::diesel::mysql::Mysql>) -> #crate_name::__private::diesel::serialize::Result {
                        let sql_val = self.__benzina04_as_str();
                        #crate_name::__private::std::io::Write::write_all(out, sql_val.as_bytes())?;

                        #crate_name::__private::std::result::Result::Ok(#crate_name::__private::diesel::serialize::IsNull::No)
                    }
                }
            });
        }
    }
}

impl EnumVariant {
    fn gen_from_bytes(&self, has_fields: bool, rename_rule: RenameRule) -> impl ToTokens + use<'_> {
        struct EnumVariantFromBytes<'a>(&'a EnumVariant, RenameRule);

        impl ToTokens for EnumVariantFromBytes<'_> {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                let Self(
                    EnumVariant {
                        original_name,
                        original_name_span,
                        rename,
                        #[cfg(all(feature = "postgres", feature = "json"))]
                        payload,

                        crate_name,
                    },
                    rename_rule,
                ) = self;
                let crate_name = crate::crate_name(crate_name);

                let rename = rename
                    .clone()
                    .unwrap_or_else(|| rename_rule.format(original_name));

                let original_name_ident = Ident::new(original_name, *original_name_span);
                let rename_bytes = LitByteStr::new(rename.as_bytes(), *original_name_span);
                tokens.append_all(quote! {
                    #rename_bytes => #crate_name::__private::std::option::Option::Some(Self::#original_name_ident),
                });
            }
        }

        EnumVariantFromBytes(self, rename_rule)
    }

    fn gen_to_str(&self, has_fields: bool, rename_rule: RenameRule) -> impl ToTokens + use<'_> {
        struct EnumVariantToStr<'a>(&'a EnumVariant, RenameRule);

        impl ToTokens for EnumVariantToStr<'_> {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                let Self(
                    EnumVariant {
                        original_name,
                        original_name_span,
                        rename,
                        #[cfg(all(feature = "postgres", feature = "json"))]
                        payload,

                        crate_name: _,
                    },
                    rename_rule,
                ) = self;

                let rename = rename
                    .clone()
                    .unwrap_or_else(|| rename_rule.format(original_name));

                let original_name_ident = Ident::new(original_name, *original_name_span);
                tokens.append_all(quote! {
                    Self::#original_name_ident => #rename,
                });
            }
        }

        EnumVariantToStr(self, rename_rule)
    }
}
