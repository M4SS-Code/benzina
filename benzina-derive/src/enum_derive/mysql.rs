use proc_macro2::TokenStream;
use quote::{TokenStreamExt, quote};

use super::Enum;

pub(super) fn append(e: &Enum, tokens: &mut TokenStream) {
    if e.has_json_fields() {
        return;
    }

    let Enum {
        ident,
        sql_type,
        rename_all,
        variants,
        crate_name,
        ..
    } = e;
    let crate_name = crate::crate_name(crate_name);

    let from_bytes_arms = variants
        .iter()
        .map(|variant| variant.gen_from_bytes(false, *rename_all))
        .collect::<Vec<_>>();
    let to_byte_str_arms = variants
        .iter()
        .map(|variant| variant.gen_to_byte_str(false, *rename_all))
        .collect::<Vec<_>>();

    tokens.append_all(quote! {
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
    });
}
