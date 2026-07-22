use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::{Data, DeriveInput, Fields, Ident, LitStr, Path, Token, Type, spanned::Spanned};

use crate::rename_rule::RenameRule;

#[cfg(any(feature = "postgres", feature = "mysql"))]
mod backend;
#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "postgres")]
mod postgres;

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

#[cfg_attr(
    not(any(feature = "postgres", feature = "mysql")),
    expect(
        dead_code,
        reason = "the input is still parsed and validated when no backend feature is enabled"
    )
)]
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

#[cfg_attr(
    not(any(feature = "postgres", feature = "mysql")),
    expect(
        dead_code,
        reason = "the input is still parsed and validated when no backend feature is enabled"
    )
)]
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

    fn as_expression(&self) -> TokenStream {
        let Self {
            ident,
            sql_type,
            crate_name,
            ..
        } = self;
        let crate_name = crate::crate_name(crate_name);

        quote! {
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
        }
    }
}

impl ToTokens for Enum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(self.as_expression());

        #[cfg(feature = "postgres")]
        postgres::append(self, tokens);
        #[cfg(feature = "mysql")]
        mysql::append(self, tokens);
    }
}
