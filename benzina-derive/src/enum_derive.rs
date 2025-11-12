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
    #[expect(clippy::too_many_lines)]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            ident,
            sql_type,
            rename_all,
            variants,

            crate_name,
        } = &self;
        let impls_ident = if self.has_json_fields() {
            Ident::new(&format!("{ident}Kind"), ident.span())
        } else {
            ident.clone()
        };
        let crate_name = crate::crate_name(crate_name);

        let has_json_fields = self.has_json_fields();
        let from_bytes_arms = variants
            .iter()
            .map(|variant| variant.gen_from_bytes(has_json_fields, *rename_all))
            .collect::<Vec<_>>();
        let to_byte_str_arms = variants
            .iter()
            .map(|variant| variant.gen_to_byte_str(has_json_fields, *rename_all))
            .collect::<Vec<_>>();

        let (queryable_sql_type, queryable_row_type, queryable_impl) = if self.has_json_fields() {
            #[cfg(all(feature = "postgres", feature = "json"))]
            {
                let from_queryable_arms = variants
                    .iter()
                    .map(|variant| variant.gen_from_queryable(&impls_ident));

                (
                    quote! { (#sql_type, #crate_name::__private::diesel::pg::sql_types::Jsonb) },
                    quote! { (#impls_ident, #crate_name::__private::json::RawJsonb) },
                    quote! {
                        match row.0 {
                            #(#from_queryable_arms)*
                        }
                    },
                )
            }

            #[cfg(not(all(feature = "postgres", feature = "json")))]
            unreachable!()
        } else {
            (
                quote! { #sql_type },
                quote! { Self },
                quote! { #crate_name::__private::std::result::Result::Ok(row) },
            )
        };

        #[cfg(feature = "postgres")]
        let postgres = quote! {
            #[automatically_derived]
            impl #crate_name::__private::diesel::deserialize::Queryable<#queryable_sql_type, #crate_name::__private::diesel::pg::Pg> for #ident {
                type Row = #queryable_row_type;

                fn build(row: Self::Row) -> #crate_name::__private::diesel::deserialize::Result<Self> {
                    #queryable_impl
                }
            }

            #[automatically_derived]
            impl #crate_name::__private::diesel::deserialize::FromSql<#sql_type, #crate_name::__private::diesel::pg::Pg> for #impls_ident {
                fn from_sql(bytes: #crate_name::__private::diesel::pg::PgValue<'_>) -> #crate_name::__private::diesel::deserialize::Result<Self> {
                    match bytes.as_bytes() {
                        #(#from_bytes_arms)*
                        _ => {
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
            impl #crate_name::__private::diesel::serialize::ToSql<#sql_type, #crate_name::__private::diesel::pg::Pg> for #impls_ident {
                fn to_sql<'b>(&'b self, out: &mut #crate_name::__private::diesel::serialize::Output<'b, '_, #crate_name::__private::diesel::pg::Pg>) -> #crate_name::__private::diesel::serialize::Result {
                    let s: &[u8] = match self {
                        #(#to_byte_str_arms)*
                    };
                    #crate_name::__private::std::io::Write::write_all(out, s)?;

                    #crate_name::__private::std::result::Result::Ok(
                        #crate_name::__private::diesel::serialize::IsNull::No
                    )
                }
            }
        };
        #[cfg(not(feature = "postgres"))]
        let postgres = quote! {};

        let postgres_extra = if self.has_json_fields() {
            let entries = self.variants.iter().map(|variant| {
                let original_name_ident = variant.original_name();
                quote! {
                    #original_name_ident,
                }
            });

            let impls_enum = Self {
                ident: impls_ident.clone(),
                sql_type: self.sql_type.clone(),
                rename_all: self.rename_all,
                variants: self
                    .variants
                    .iter()
                    .map(
                        |EnumVariant {
                             original_name,
                             original_name_span,
                             rename,
                             payload,
                             crate_name,
                         }| EnumVariant {
                            original_name: original_name.clone(),
                            original_name_span: *original_name_span,
                            rename: rename.clone(),
                            payload: None,
                            crate_name: crate_name.clone(),
                        },
                    )
                    .collect(),
                crate_name: self.crate_name.clone(),
            };
            quote! {
                #[derive(Debug, Copy, Clone, PartialEq, Eq)]
                pub struct #impls_ident {
                    #(#entries)*
                }

                #impls_enum
            }
        } else {
            quote! {}
        };

        #[cfg(feature = "mysql")]
        let mysql = if self.has_json_fields() {
            unreachable!()
        } else {
            quote! {
                #[automatically_derived]
                impl #crate_name::__private::diesel::deserialize::Queryable<#queryable_sql_type, #crate_name::__private::diesel::mysql::Mysql> for #ident {
                    type Row = #queryable_row_type;

                    fn build(row: Self::Row) -> #crate_name::__private::diesel::deserialize::Result<Self> {
                        #queryable_impl
                    }
                }

                #[automatically_derived]
                impl #crate_name::__private::diesel::deserialize::FromSql<#sql_type, #crate_name::__private::diesel::mysql::Mysql> for #impls_ident {
                    fn from_sql(bytes: #crate_name::__private::diesel::mysql::MysqlValue<'_>) -> #crate_name::__private::diesel::deserialize::Result<Self> {
                        match bytes.as_bytes() {
                            #(#from_bytes_arms)*
                            _ => {
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
                impl #crate_name::__private::diesel::serialize::ToSql<#sql_type, #crate_name::__private::diesel::mysql::Mysql> for #impls_ident {
                    fn to_sql<'b>(&'b self, out: &mut #crate_name::__private::diesel::serialize::Output<'b, '_, #crate_name::__private::diesel::mysql::Mysql>) -> #crate_name::__private::diesel::serialize::Result {
                        let s: &[u8] = match self {
                            #(#to_byte_str_arms)*
                        };
                        #crate_name::__private::std::io::Write::write_all(out, s)?;

                        #crate_name::__private::std::result::Result::Ok(#crate_name::__private::diesel::serialize::IsNull::No)
                    }
                }
            }
        };
        #[cfg(not(feature = "mysql"))]
        let mysql = quote! {};

        tokens.append_all(quote! {
            #postgres
            #postgres_extra
            #mysql
        });
    }
}

impl EnumVariant {
    fn original_name(&self) -> Ident {
        Ident::new(&self.original_name, self.original_name_span)
    }

    fn gen_from_bytes(&self, has_fields: bool, rename_rule: RenameRule) -> impl ToTokens {
        let Self {
            original_name,
            original_name_span,
            rename,
            #[cfg(all(feature = "postgres", feature = "json"))]
            payload,

            crate_name,
        } = self;
        let crate_name = crate::crate_name(crate_name);

        let rename = rename
            .clone()
            .unwrap_or_else(|| rename_rule.format(original_name));

        let original_name_ident = self.original_name();
        let rename_bytes = LitByteStr::new(rename.as_bytes(), *original_name_span);
        quote! {
            #rename_bytes => #crate_name::__private::std::result::Result::Ok(Self::#original_name_ident),
        }
    }

    fn gen_from_queryable(&self, impls_ident: &Ident) -> impl ToTokens {
        let Self {
            original_name,
            original_name_span,
            rename,
            #[cfg(all(feature = "postgres", feature = "json"))]
            payload,

            crate_name,
        } = self;
        let crate_name = crate::crate_name(crate_name);

        let original_name_ident = self.original_name();

        let inner = if self.payload.is_some() {
            quote! {
                #crate_name::__private::std::result::Result::map(
                    #crate_name::__private::json::RawJsonb::deserialize(&row.1),
                    Self::#original_name_ident
                )
            }
        } else {
            quote! {
                Self::#original_name_ident
            }
        };
        quote! {
            #impls_ident::#original_name_ident => {
                #inner
            },
        }
    }

    fn gen_to_byte_str(&self, has_fields: bool, rename_rule: RenameRule) -> impl ToTokens {
        let Self {
            original_name,
            original_name_span,
            rename,
            #[cfg(all(feature = "postgres", feature = "json"))]
            payload,

            crate_name: _,
        } = self;

        let rename = rename
            .clone()
            .unwrap_or_else(|| rename_rule.format(original_name));

        let original_name_ident = self.original_name();
        let rename_bytes = LitByteStr::new(rename.as_bytes(), *original_name_span);
        quote! {
            Self::#original_name_ident => #rename_bytes,
        }
    }
}
