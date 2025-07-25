use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

pub(super) struct NewIndexMap;

impl ToTokens for NewIndexMap {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! {
            ::benzina::__private::new_indexmap::<_, _>()
        });
    }
}

pub(super) struct Identifiable<T> {
    pub(super) table: T,
}

impl<T: ToTokens> ToTokens for Identifiable<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { table } = self;
        tokens.extend(quote! {
            ::benzina::__private::std::clone::Clone::clone(
                <_ as ::benzina::__private::diesel::associations::Identifiable>::id(&#table)
            )
        });
    }
}
