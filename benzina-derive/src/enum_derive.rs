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

    #[cfg(all(feature = "postgres", feature = "json"))]
    table: Option<Path>,
    #[cfg(all(feature = "postgres", feature = "json"))]
    column: Option<Ident>,
    #[cfg(all(feature = "postgres", feature = "json"))]
    data_column: Option<Ident>,

    crate_name: Option<Path>,
}

struct EnumVariant {
    original_name: String,
    original_name_span: Span,
    rename: Option<String>,
    #[cfg(all(feature = "postgres", feature = "json"))]
    has_payload: bool,

    crate_name: Option<Path>,
}

impl Enum {
    #[expect(clippy::too_many_lines)]
    pub(crate) fn parse(input: DeriveInput) -> Result<Self, syn::Error> {
        let Data::Enum(e) = input.data else {
            fail!(input, "`benzina::Enum` macro available only for enums");
        };

        let mut first_attr = None;
        let mut sql_type = None;
        let mut rename_all = None;
        #[cfg(all(feature = "postgres", feature = "json"))]
        let mut table = None;
        #[cfg(all(feature = "postgres", feature = "json"))]
        let mut column = None;
        #[cfg(all(feature = "postgres", feature = "json"))]
        let mut data_column = None;
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
                } else if meta.path.is_ident("table") {
                    #[cfg(all(feature = "postgres", feature = "json"))]
                    {
                        meta.input.parse::<Token![=]>()?;
                        let val: Path = meta.input.parse()?;
                        try_set!(table, val, val);
                    }
                    #[cfg(not(all(feature = "postgres", feature = "json")))]
                    {
                        let _ = meta.input.parse::<Token![=]>()?;
                        let _: Path = meta.input.parse()?;
                    }
                } else if meta.path.is_ident("column") {
                    #[cfg(all(feature = "postgres", feature = "json"))]
                    {
                        meta.input.parse::<Token![=]>()?;
                        let val: Ident = meta.input.parse()?;
                        try_set!(column, val, val);
                    }
                    #[cfg(not(all(feature = "postgres", feature = "json")))]
                    {
                        let _ = meta.input.parse::<Token![=]>()?;
                        let _: Ident = meta.input.parse()?;
                    }
                } else if meta.path.is_ident("data_column") {
                    #[cfg(all(feature = "postgres", feature = "json"))]
                    {
                        meta.input.parse::<Token![=]>()?;
                        let val: Ident = meta.input.parse()?;
                        try_set!(data_column, val, val);
                    }
                    #[cfg(not(all(feature = "postgres", feature = "json")))]
                    {
                        let _ = meta.input.parse::<Token![=]>()?;
                        let _: Ident = meta.input.parse()?;
                    }
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

        let rename_all = rename_all.unwrap_or(RenameRule::None);

        let variants = e
            .variants
            .into_iter()
            .map(|variant| {
                let has_payload = match &variant.fields {
                    Fields::Unit => false,
                    #[cfg(all(feature = "postgres", feature = "json"))]
                    Fields::Unnamed(fields) => {
                        let mut fields = fields.unnamed.iter();
                        if !matches!((fields.next(), fields.next()), (Some(_),None)){
                            fail!(variant, "only single-item variants are supported");
                        }

                        true
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
                let _ = has_payload;

                let original_name_span = variant.span();
                Ok(EnumVariant {
                    original_name: name,
                    original_name_span,
                    rename,
                    #[cfg(all(feature = "postgres", feature = "json"))]
                    has_payload,

                    crate_name: crate_name.clone(),
                })
            })
            .collect::<Result<Vec<_>, syn::Error>>()?;
        Ok(Self {
            ident: input.ident,
            sql_type,
            rename_all,
            variants,

            #[cfg(all(feature = "postgres", feature = "json"))]
            table,
            #[cfg(all(feature = "postgres", feature = "json"))]
            column,
            #[cfg(all(feature = "postgres", feature = "json"))]
            data_column,

            crate_name,
        })
    }

    #[cfg(all(feature = "postgres", feature = "json"))]
    fn has_json_fields(&self) -> bool {
        self.variants.iter().any(|variant| variant.has_payload)
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

            #[cfg(all(feature = "postgres", feature = "json"))]
                table: _,
            #[cfg(all(feature = "postgres", feature = "json"))]
                column: _,
            #[cfg(all(feature = "postgres", feature = "json"))]
                data_column: _,

            crate_name,
        } = &self;
        let crate_name = crate::crate_name(crate_name);

        let has_json_fields = self.has_json_fields();
        let impls_ident = Ident::new(&format!("{ident}Kind"), ident.span());

        let as_expression = quote! {
            #[automatically_derived]
            impl #crate_name::__private::diesel::expression::AsExpression<#sql_type> for #ident {
                type Expression = #crate_name::__private::diesel::internal::derives::as_expression::Bound<
                    #sql_type,
                    Self,
                >;

                fn as_expression(self) -> Self::Expression {
                    #crate_name::__private::diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            #[automatically_derived]
            impl<'__expr> #crate_name::__private::diesel::expression::AsExpression<#sql_type> for &'__expr #ident {
                type Expression = #crate_name::__private::diesel::internal::derives::as_expression::Bound<
                    #sql_type,
                    Self,
                >;

                fn as_expression(self) -> Self::Expression {
                    #crate_name::__private::diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            #[automatically_derived]
            impl<'__expr, '__expr2> #crate_name::__private::diesel::expression::AsExpression<#sql_type> for &'__expr2 &'__expr #ident {
                type Expression = #crate_name::__private::diesel::internal::derives::as_expression::Bound<
                    #sql_type,
                    Self,
                >;

                fn as_expression(self) -> Self::Expression {
                    #crate_name::__private::diesel::internal::derives::as_expression::Bound::new(self)
                }
            }
        };

        let from_bytes_arms = variants
            .iter()
            .map(|variant| variant.gen_from_bytes(has_json_fields, *rename_all))
            .collect::<Vec<_>>();
        #[cfg(feature = "postgres")]
        let to_byte_str_arms = variants
            .iter()
            .map(|variant| variant.gen_to_byte_str(has_json_fields, *rename_all))
            .collect::<Vec<_>>();

        #[cfg(feature = "postgres")]
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
        let postgres_from_to_sql = if has_json_fields {
            quote! {}
        } else {
            quote! {
                #[automatically_derived]
                impl #crate_name::__private::diesel::deserialize::FromSql<#sql_type, #crate_name::__private::diesel::pg::Pg> for #ident {
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
                impl #crate_name::__private::diesel::serialize::ToSql<#sql_type, #crate_name::__private::diesel::pg::Pg> for #ident {
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
            }
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

            #postgres_from_to_sql
        };
        #[cfg(not(feature = "postgres"))]
        let postgres = quote! {};

        #[cfg(all(feature = "postgres", feature = "json"))]
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
                             has_payload: _,
                             crate_name,
                         }| EnumVariant {
                            original_name: original_name.clone(),
                            original_name_span: *original_name_span,
                            rename: rename.clone(),
                            has_payload: false,
                            crate_name: crate_name.clone(),
                        },
                    )
                    .collect(),
                table: None,
                column: None,
                data_column: None,
                crate_name: self.crate_name.clone(),
            };
            let selectable_insertable_impl = if let (Some(table), Some(column), Some(data_column)) =
                (&self.table, &self.column, &self.data_column)
            {
                let to_insertable_arms = self
                    .variants
                    .iter()
                    .map(|variant| variant.gen_to_insertable(ident, &impls_ident));

                quote! {
                    #[automatically_derived]
                    impl #crate_name::__private::diesel::expression::Selectable<#crate_name::__private::diesel::pg::Pg> for #ident {
                        type SelectExpression = (#table::#column, #table::#data_column);

                        fn construct_selection() -> Self::SelectExpression {
                            (#table::#column, #table::#data_column)
                        }
                    }

                    #[automatically_derived]
                    impl<'__ins> #crate_name::__private::diesel::Insertable<#table::table> for &'__ins #ident {
                        type Values = <(
                            #crate_name::__private::diesel::dsl::Eq<#table::#column, #impls_ident>,
                            #crate_name::__private::diesel::dsl::Eq<#table::#data_column, #crate_name::__private::json::RawJsonb>,
                        ) as #crate_name::__private::diesel::Insertable<#table::table>>::Values;

                        fn values(self) -> Self::Values {
                            use #crate_name::__private::diesel::ExpressionMethods;
                            let (kind, data) = match self {
                                #(#to_insertable_arms)*
                            };
                            #crate_name::__private::diesel::Insertable::values((
                                #table::#column.eq(kind),
                                #table::#data_column.eq(data),
                            ))
                        }
                    }

                    #[automatically_derived]
                    impl #crate_name::__private::diesel::Insertable<#table::table> for #ident {
                        type Values = <(
                            #crate_name::__private::diesel::dsl::Eq<#table::#column, #impls_ident>,
                            #crate_name::__private::diesel::dsl::Eq<#table::#data_column, #crate_name::__private::json::RawJsonb>,
                        ) as #crate_name::__private::diesel::Insertable<#table::table>>::Values;

                        fn values(self) -> Self::Values {
                            #crate_name::__private::diesel::Insertable::values(&self)
                        }
                    }
                }
            } else {
                quote! {}
            };

            quote! {
                #[derive(Debug, Copy, Clone, PartialEq, Eq)]
                pub enum #impls_ident {
                    #(#entries)*
                }

                #impls_enum

                #selectable_insertable_impl
            }
        } else {
            quote! {}
        };
        #[cfg(not(all(feature = "postgres", feature = "json")))]
        let postgres_extra = quote! {};

        #[cfg(feature = "mysql")]
        let mysql = if self.has_json_fields() {
            quote! {}
        } else {
            let from_bytes_arms = variants
                .iter()
                .map(|variant| variant.gen_from_bytes(false, *rename_all))
                .collect::<Vec<_>>();
            let to_byte_str_arms = variants
                .iter()
                .map(|variant| variant.gen_to_byte_str(false, *rename_all))
                .collect::<Vec<_>>();

            quote! {
                #[automatically_derived]
                impl #crate_name::__private::diesel::deserialize::Queryable<#sql_type, #crate_name::__private::diesel::mysql::Mysql> for #ident {
                    type Row = Self;

                    fn build(row: Self::Row) -> #crate_name::__private::diesel::deserialize::Result<Self> {
                        #crate_name::__private::std::result::Result::Ok(row)
                    }
                }

                #[automatically_derived]
                impl #crate_name::__private::diesel::deserialize::FromSql<#sql_type, #crate_name::__private::diesel::mysql::Mysql> for #ident {
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
                impl #crate_name::__private::diesel::serialize::ToSql<#sql_type, #crate_name::__private::diesel::mysql::Mysql> for #ident {
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
            #as_expression
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

    fn gen_from_bytes(&self, _has_fields: bool, rename_rule: RenameRule) -> impl ToTokens {
        let Self {
            original_name,
            original_name_span,
            rename,
            #[cfg(all(feature = "postgres", feature = "json"))]
                has_payload: _,

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

    #[cfg(all(feature = "postgres", feature = "json"))]
    fn gen_from_queryable(&self, impls_ident: &Ident) -> impl ToTokens {
        let crate_name = crate::crate_name(&self.crate_name);

        let original_name_ident = self.original_name();

        let inner = if self.has_payload {
            quote! {
                #crate_name::__private::std::result::Result::map(
                    #crate_name::__private::json::RawJsonb::deserialize(&row.1),
                    Self::#original_name_ident
                )
            }
        } else {
            quote! {
                #crate_name::__private::std::result::Result::Ok(Self::#original_name_ident)
            }
        };
        quote! {
            #impls_ident::#original_name_ident => {
                #inner
            },
        }
    }

    fn gen_to_byte_str(&self, _has_fields: bool, rename_rule: RenameRule) -> impl ToTokens {
        let Self {
            original_name,
            original_name_span,
            rename,
            #[cfg(all(feature = "postgres", feature = "json"))]
                has_payload: _,

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

    #[cfg(all(feature = "postgres", feature = "json"))]
    fn gen_to_insertable(&self, ident: &Ident, impls_ident: &Ident) -> impl ToTokens {
        let crate_name = crate::crate_name(&self.crate_name);
        let original_name_ident = self.original_name();

        if self.has_payload {
            quote! {
                #ident::#original_name_ident(payload) => (
                    #impls_ident::#original_name_ident,
                    #crate_name::__private::json::RawJsonb::serialize(payload)
                        .expect("failed to serialize enum payload"),
                ),
            }
        } else {
            quote! {
                #ident::#original_name_ident => (
                    #impls_ident::#original_name_ident,
                    #crate_name::__private::json::RawJsonb::EMPTY,
                ),
            }
        }
    }
}
