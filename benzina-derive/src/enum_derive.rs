use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::{Data, DeriveInput, Fields, Ident, LitByteStr, LitStr, Token, Type, spanned::Spanned};

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
}

struct EnumVariant {
    original_name: String,
    rename: Option<String>,
    span: Span,
}

impl Enum {
    pub(crate) fn parse(input: DeriveInput) -> Result<Self, syn::Error> {
        let Data::Enum(e) = input.data else {
            fail!(input, "`benzina::Enum` macro available only for enums");
        };

        let (rename_all, sql_type) = {
            let Some(attr) = input
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("benzina"))
            else {
                fail!(e.enum_token, "expected #[benzina(...)] attribute");
            };

            let mut sql_type = None;
            let mut rename_all = None;
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
                }

                Ok(())
            })?;

            let Some(sql_type) = sql_type else {
                fail!(attr, "expected `sql_type`");
            };

            (rename_all.unwrap_or(RenameRule::None), sql_type)
        };

        let variants = e
            .variants
            .into_iter()
            .map(|variant| {
                if !matches!(variant.fields, Fields::Unit) {
                    fail!(variant, "only unit variants are supported");
                }

                let name = variant.ident.to_string();
                let rename = if let Some(attr) = variant
                    .attrs
                    .iter()
                    .find(|attr| attr.path().is_ident("benzina"))
                {
                    let mut rename = None;
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("rename") {
                            meta.input.parse::<Token![=]>()?;
                            let val: LitStr = meta.input.parse()?;
                            try_set!(rename, val.value(), val);
                        }

                        Ok(())
                    })?;

                    rename
                } else {
                    None
                };

                let span = variant.span();
                Ok(EnumVariant {
                    original_name: name,
                    rename,
                    span,
                })
            })
            .collect::<Result<Vec<_>, syn::Error>>()?;
        Ok(Self {
            ident: input.ident,
            sql_type,
            rename_all,
            variants,
        })
    }
}

impl ToTokens for Enum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            ident,
            sql_type,
            rename_all,
            variants,
        } = &self;

        let from_bytes_arms = variants
            .iter()
            .map(|variant| variant.gen_from_bytes(*rename_all));
        let to_str_arms = variants
            .iter()
            .map(|variant| variant.gen_to_str(*rename_all));

        tokens.append_all(quote! {
            impl #ident {
                #[doc(hidden)]
                fn __benzina03_from_bytes(val: &[u8]) -> ::std::option::Option<Self> {
                    match val {
                        #(#from_bytes_arms)*
                        _ => ::std::option::Option::None,
                    }
                }

                #[doc(hidden)]
                fn __benzina03_as_str(&self) -> &'static str {
                    match self {
                        #(#to_str_arms)*
                    }
                }
            }
        });

        #[cfg(feature = "postgres")]
        tokens.append_all(quote! {
            #[automatically_derived]
            impl ::diesel::deserialize::FromSql<#sql_type, ::diesel::pg::Pg> for #ident {
                fn from_sql(bytes: ::diesel::pg::PgValue<'_>) -> ::diesel::deserialize::Result<Self> {
                    match Self::__benzina03_from_bytes(bytes.as_bytes()) {
                        ::std::option::Option::Some(this) => ::std::result::Result::Ok(this),
                        ::std::option::Option::None => ::std::result::Result::Err("Unrecognized enum variant".into()),
                    }
                }
            }

            #[automatically_derived]
            impl ::diesel::serialize::ToSql<#sql_type, ::diesel::pg::Pg> for #ident {
                fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, ::diesel::pg::Pg>) -> ::diesel::serialize::Result {
                    let sql_val = self.__benzina03_as_str();
                    ::std::io::Write::write_all(out, sql_val.as_bytes())?;

                    ::std::result::Result::Ok(diesel::serialize::IsNull::No)
                }
            }
        });

        #[cfg(feature = "mysql")]
        tokens.append_all(quote! {
            #[automatically_derived]
            impl ::diesel::deserialize::FromSql<#sql_type, ::diesel::mysql::Mysql> for #ident {
                fn from_sql(bytes: ::diesel::mysql::MysqlValue<'_>) -> ::diesel::deserialize::Result<Self> {
                    match Self::__benzina03_from_bytes(bytes.as_bytes()) {
                        ::std::option::Option::Some(this) => ::std::result::Result::Ok(this),
                        ::std::option::Option::None => ::std::result::Result::Err("Unrecognized enum variant".into()),
                    }
                }
            }

            #[automatically_derived]
            impl ::diesel::serialize::ToSql<#sql_type, ::diesel::mysql::Mysql> for #ident {
                fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, ::diesel::mysql::Mysql>) -> ::diesel::serialize::Result {
                    let sql_val = self.__benzina03_as_str();
                    ::std::io::Write::write_all(out, sql_val.as_bytes())?;

                    ::std::result::Result::Ok(diesel::serialize::IsNull::No)
                }
            }
        });
    }
}

impl EnumVariant {
    fn gen_from_bytes(&self, rename_rule: RenameRule) -> impl ToTokens + use<'_> {
        struct EnumVariantFromBytes<'a>(&'a EnumVariant, RenameRule);

        impl ToTokens for EnumVariantFromBytes<'_> {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                let Self(
                    EnumVariant {
                        original_name,
                        rename,
                        span,
                    },
                    rename_rule,
                ) = self;

                let rename = rename
                    .clone()
                    .unwrap_or_else(|| rename_rule.format(original_name));

                let original_name_ident = Ident::new(original_name, *span);
                let rename_bytes = LitByteStr::new(rename.as_bytes(), *span);
                tokens.append_all(quote! {
                    #rename_bytes => ::std::option::Option::Some(Self::#original_name_ident),
                });
            }
        }

        EnumVariantFromBytes(self, rename_rule)
    }

    fn gen_to_str(&self, rename_rule: RenameRule) -> impl ToTokens + use<'_> {
        struct EnumVariantToStr<'a>(&'a EnumVariant, RenameRule);

        impl ToTokens for EnumVariantToStr<'_> {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                let Self(
                    EnumVariant {
                        original_name,
                        rename,
                        span,
                    },
                    rename_rule,
                ) = self;

                let rename = rename
                    .clone()
                    .unwrap_or_else(|| rename_rule.format(original_name));

                let original_name_ident = Ident::new(original_name, *span);
                tokens.append_all(quote! {
                    Self::#original_name_ident => #rename,
                });
            }
        }

        EnumVariantToStr(self, rename_rule)
    }
}
