use proc_macro2::TokenStream;
use quote::{TokenStreamExt, quote};

#[cfg(feature = "json")]
use proc_macro2::Ident;
#[cfg(feature = "json")]
use quote::ToTokens;

use super::Enum;
#[cfg(feature = "json")]
use super::EnumVariant;

pub(super) fn append(e: &Enum, tokens: &mut TokenStream) {
    let Enum {
        ident,
        sql_type,
        rename_all,
        variants,
        crate_name,
        ..
    } = e;
    let crate_name = crate::crate_name(crate_name);

    let has_json_fields = e.has_json_fields();

    let (queryable_sql_type, queryable_row_type, queryable_impl) = if has_json_fields {
        #[cfg(feature = "json")]
        {
            let impls_ident = kind_ident(ident);
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

        #[cfg(not(feature = "json"))]
        unreachable!()
    } else {
        (
            quote! { #sql_type },
            quote! { Self },
            quote! { #crate_name::__private::std::result::Result::Ok(row) },
        )
    };

    let from_to_sql = if has_json_fields {
        quote! {}
    } else {
        let from_bytes_arms = variants
            .iter()
            .map(|variant| variant.gen_from_bytes(has_json_fields, *rename_all))
            .collect::<Vec<_>>();
        let to_byte_str_arms = variants
            .iter()
            .map(|variant| variant.gen_to_byte_str(has_json_fields, *rename_all))
            .collect::<Vec<_>>();

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

    tokens.append_all(quote! {
        #[automatically_derived]
        impl #crate_name::__private::diesel::deserialize::Queryable<#queryable_sql_type, #crate_name::__private::diesel::pg::Pg> for #ident {
            type Row = #queryable_row_type;

            fn build(row: Self::Row) -> #crate_name::__private::diesel::deserialize::Result<Self> {
                #queryable_impl
            }
        }

        #from_to_sql
    });

    #[cfg(feature = "json")]
    append_json_extra(e, tokens);
}

#[cfg(feature = "json")]
fn kind_ident(ident: &Ident) -> Ident {
    Ident::new(&format!("{ident}Kind"), ident.span())
}

#[cfg(feature = "json")]
fn append_json_extra(e: &Enum, tokens: &mut TokenStream) {
    if !e.has_json_fields() {
        return;
    }

    let Enum {
        ident, crate_name, ..
    } = e;
    let crate_name = crate::crate_name(crate_name);

    let impls_ident = kind_ident(ident);

    let entries = e.variants.iter().map(|variant| {
        let original_name_ident = variant.original_name();
        quote! {
            #original_name_ident,
        }
    });

    let impls_enum = Enum {
        ident: impls_ident.clone(),
        sql_type: e.sql_type.clone(),
        rename_all: e.rename_all,
        variants: e
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
        crate_name: e.crate_name.clone(),
    };
    let selectable_insertable_impl = if let (Some(table), Some(column), Some(data_column)) =
        (&e.table, &e.column, &e.data_column)
    {
        let to_insertable_arms = e
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

    tokens.append_all(quote! {
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub enum #impls_ident {
            #(#entries)*
        }

        #impls_enum

        #selectable_insertable_impl
    });
}

#[cfg(feature = "json")]
impl EnumVariant {
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
