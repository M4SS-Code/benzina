use proc_macro2::Ident;
use quote::{ToTokens, quote};
use syn::LitByteStr;

use super::{Enum, EnumVariant};
use crate::rename_rule::RenameRule;

impl Enum {
    #[cfg(all(feature = "postgres", feature = "json"))]
    pub(super) fn has_json_fields(&self) -> bool {
        self.variants.iter().any(|variant| variant.has_payload)
    }

    #[cfg(not(all(feature = "postgres", feature = "json")))]
    #[expect(
        clippy::unused_self,
        reason = "kept for compatibility with the above implementation"
    )]
    pub(super) fn has_json_fields(&self) -> bool {
        false
    }
}

impl EnumVariant {
    pub(super) fn original_name(&self) -> Ident {
        Ident::new(&self.original_name, self.original_name_span)
    }

    pub(super) fn gen_from_bytes(
        &self,
        _has_fields: bool,
        rename_rule: RenameRule,
    ) -> impl ToTokens {
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

    pub(super) fn gen_to_byte_str(
        &self,
        _has_fields: bool,
        rename_rule: RenameRule,
    ) -> impl ToTokens {
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
}
