use std::collections::BTreeMap;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{Ident, Index, Token, punctuated::Punctuated};

use self::{
    quantity::Quantity,
    utils::{Identifiable, NewIndexMap},
};

mod parse;
mod quantity;
mod utils;

pub(crate) struct Join {
    input: Ident,
    transformation: Transformation,
}

pub(super) enum NestedOrNot {
    Nested(Transformation),
    Not(NoTransformation),
}

pub(super) struct Transformation {
    quantity: Quantity,
    output_type: Ident,
    entries: Punctuated<(Ident, NestedOrNot), Token![,]>,
}

pub(super) struct NoTransformation {
    quantity: Quantity,
    tuple_index: usize,
}

impl Join {
    fn map_type(&self) -> TokenStream {
        self.transformation.map_type()
    }

    fn accumulator(&self) -> TokenStream {
        let Self {
            input,
            transformation: _,
        } = self;
        let accumulator = self.transformation.accumulator(None);
        quote! {
            for row in #input {
                #accumulator
            }
        }
    }

    fn presenter(&self) -> TokenStream {
        let accumulator = quote! { accumulator };
        self.transformation.presenter(&accumulator)
    }
}

impl ToTokens for Join {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let map_type = self.map_type();
        let accumulator = self.accumulator();
        let presenter = self.presenter();
        tokens.extend(quote! {
            {
                let mut accumulator: #map_type = ::benzina::__private::new_indexmap();
                #accumulator
                #presenter
            }
        });
    }
}

impl NestedOrNot {
    fn map_type_values(&self) -> Vec<TokenStream> {
        match self {
            Self::Nested(nested) => vec![nested.map_type()],
            Self::Not(not) => not.map_type_values(),
        }
    }

    fn accumulator(&self, accumulator_index: usize) -> TokenStream {
        match self {
            Self::Nested(nested) => nested.accumulator(Some(accumulator_index)),
            Self::Not(not) => not.accumulator(accumulator_index),
        }
    }

    fn or_insert(&self, tuple_index_overwrites: &BTreeMap<usize, TokenStream>) -> Vec<TokenStream> {
        match self {
            Self::Nested(_nested) => {
                vec![NewIndexMap.into_token_stream()]
            }
            Self::Not(not) => not.or_insert(tuple_index_overwrites),
        }
    }

    fn presenter(&self, accumulator: &TokenStream) -> TokenStream {
        match self {
            Self::Nested(nested) => nested.presenter(accumulator),
            Self::Not(not) => not.presenter(accumulator),
        }
    }
}

impl Transformation {
    fn map_type(&self) -> TokenStream {
        let values = self
            .entries
            .iter()
            .flat_map(|(_key, value)| value.map_type_values());
        quote! { ::benzina::__private::IndexMap::<_, (#(#values),*)> }
    }

    fn accumulator(&self, accumulator_index: Option<usize>) -> TokenStream {
        let accumulator_index = if let Some(accumulator_index) = accumulator_index {
            let accumulator_index = Index::from(accumulator_index);
            quote! { accumulator.#accumulator_index }
        } else {
            quote! { accumulator }
        };
        let one = self
            .entries
            .iter()
            .find_map(|(_name, entry)| match entry {
                NestedOrNot::Nested(_nested) => None,
                NestedOrNot::Not(not) => Some(not),
            })
            .unwrap();
        let one_tuple_index = Index::from(one.tuple_index);

        let mut tuple_index_overwrites = BTreeMap::new();
        let wrapper = if matches!(self.quantity, Quantity::AtLeastZero) {
            let name = Ident::new(&format!("unwrapped{}", one.tuple_index), Span::call_site());
            tuple_index_overwrites.insert(one.tuple_index, quote! { #name });
            quote! { if let ::benzina::__private::std::option::Option::Some(#name) = row.#one_tuple_index }
        } else {
            quote! {}
        };

        let or_insert = self.or_insert(&tuple_index_overwrites);
        let accumulator = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, (_name, entry))| entry.accumulator(i));

        let one_name = if let Some(overwrite) = tuple_index_overwrites.get(&one.tuple_index) {
            overwrite.clone()
        } else {
            quote! { row.#one_tuple_index }
        };
        let id = Identifiable { table: one_name };
        quote! {
            #wrapper {
                let mut accumulator = ::benzina::__private::indexmap::map::Entry::or_insert(
                    ::benzina::__private::IndexMap::entry(&mut #accumulator_index, #id),
                    (#(#or_insert),*)
                );
                #(#accumulator)*
            }
        }
    }

    fn or_insert(&self, tuple_index_overwrites: &BTreeMap<usize, TokenStream>) -> Vec<TokenStream> {
        self.entries
            .iter()
            .flat_map(|(_name, entry)| entry.or_insert(tuple_index_overwrites))
            .collect()
    }

    fn presenter(&self, accumulator: &TokenStream) -> TokenStream {
        let Self {
            quantity,
            output_type,
            entries,
        } = self;
        let is_result = self.is_result();

        let entries = entries.iter().enumerate().map(|(i, (name, entry))| {
            let item = Ident::new("item", Span::call_site());
            let ii = Index::from(i);
            let item = quote! { #item.#ii };
            let entry = entry.presenter(&item);
            quote! {
                #name: #entry
            }
        });
        let map_closure = if is_result {
            quote! {
                |item| ::benzina::__private::std::result::Result::Ok::<
                    #output_type,
                    ::benzina::__private::diesel::result::Error
                >(#output_type {
                    #(#entries),*
                })
            }
        } else {
            quote! {
                |item| #output_type {
                    #(#entries),*
                }
            }
        };
        let iterator = quote! {
            ::benzina::__private::std::iter::Iterator::map(
                ::benzina::__private::IndexMap::into_values(#accumulator),
                #map_closure
            )
        };
        match quantity {
            Quantity::MaybeOne => {
                if is_result {
                    quote! {
                        ::benzina::__private::std::option::Option::transpose(
                            ::benzina::__private::std::iter::Iterator::next(
                                &mut #iterator
                            )
                        )?
                    }
                } else {
                    quote! {
                        ::benzina::__private::std::iter::Iterator::next(
                            &mut #iterator
                        )
                    }
                }
            }
            Quantity::One | Quantity::AssumeOne => {
                quote! {
                    match ::benzina::__private::std::iter::Iterator::next(
                        &mut #iterator
                    ) {
                        ::benzina::__private::std::option::Option::Some(item) => item,
                        ::benzina::__private::std::option::Option::None => return ::benzina::__private::std::result::Result::Err(
                            ::benzina::__private::diesel::result::Error::NotFound
                        )
                    }
                }
            }
            Quantity::AtLeastZero | Quantity::AtLeastOne => {
                if is_result {
                    quote! {
                        ::benzina::__private::std::iter::Iterator::collect::<
                            ::benzina::__private::std::result::Result<
                                ::benzina::__private::std::vec::Vec<_>,
                                ::benzina::__private::diesel::result::Error,
                            >
                        >(
                            #iterator
                        )?
                    }
                } else {
                    quote! {
                        ::benzina::__private::std::iter::Iterator::collect::<
                            ::benzina::__private::std::vec::Vec<_>
                        >(
                            #iterator
                        )
                    }
                }
            }
        }
    }

    fn is_result(&self) -> bool {
        match self.quantity {
            Quantity::AtLeastZero | Quantity::AtLeastOne => true,
            _ => self.entries.iter().any(|(_, entry)| match entry {
                NestedOrNot::Nested(nested) => nested.is_result(),
                NestedOrNot::Not(_) => false,
            }),
        }
    }
}

impl NoTransformation {
    fn map_type_values(&self) -> Vec<TokenStream> {
        match self.quantity {
            Quantity::MaybeOne => vec![quote! {
                ::benzina::__private::std::option::Option<_>
            }],
            Quantity::One | Quantity::AssumeOne => vec![quote! {
                _
            }],
            Quantity::AtLeastZero | Quantity::AtLeastOne => vec![quote! {
                ::benzina::__private::IndexMap::<_, _>
            }],
        }
    }

    fn accumulator(&self, accumulator_index: usize) -> TokenStream {
        let tuple_index = Index::from(self.tuple_index);
        let row = quote! { row.#tuple_index };

        let accumulator_index = Index::from(accumulator_index);
        match self.quantity {
            Quantity::MaybeOne => quote! {
                {
                    if let ::benzina::__private::std::option::Option::Some(item) = #row {
                        accumulator.#accumulator_index = ::benzina::__private::std::option::Option::Some(item);
                    }
                }
            },
            Quantity::One | Quantity::AssumeOne => quote! {},
            Quantity::AtLeastZero => {
                let id = Identifiable {
                    table: quote! { item },
                };
                quote! {
                    {
                        if let ::benzina::__private::std::option::Option::Some(item) = #row {
                            ::benzina::__private::indexmap::map::Entry::or_insert(
                                ::benzina::__private::IndexMap::entry(&mut accumulator.#accumulator_index, #id),
                                item
                            );
                        }
                    }
                }
            }
            Quantity::AtLeastOne => {
                let id = Identifiable {
                    table: quote! { item },
                };
                quote! {
                    {
                        let item = #row;
                        ::benzina::__private::indexmap::map::Entry::or_insert(
                            ::benzina::__private::IndexMap(&mut accumulator.#accumulator_index, #id),
                            item
                        );
                    }
                }
            }
        }
    }

    fn or_insert(&self, tuple_index_overwrites: &BTreeMap<usize, TokenStream>) -> Vec<TokenStream> {
        match self.quantity {
            Quantity::MaybeOne => vec![quote! { ::benzina::__private::std::option::Option::None }],
            Quantity::One => {
                if let Some(overwrite) = tuple_index_overwrites.get(&self.tuple_index) {
                    vec![quote! { #overwrite }]
                } else {
                    let tuple_index = Index::from(self.tuple_index);
                    vec![quote! { row.#tuple_index }]
                }
            }
            Quantity::AssumeOne => {
                if let Some(overwrite) = tuple_index_overwrites.get(&self.tuple_index) {
                    vec![quote! { #overwrite }]
                } else {
                    let tuple_index = Index::from(self.tuple_index);
                    vec![quote! {
                        if let ::benzina::__private::std::option::Option::Some(item) = row.#tuple_index {
                            item
                        } else {
                            return ::benzina::__private::std::result::Result::Err(::benzina::__private::diesel::result::Error::DeserializationError(
                                ::benzina::__private::std::boxed::Box::from(
                                    ::benzina::__private::std::borrow::ToOwned::to_owned(
                                        "`AssumeOne` value is null"
                                    )
                                )
                            ));
                        }
                    }]
                }
            }
            Quantity::AtLeastZero | Quantity::AtLeastOne => {
                vec![NewIndexMap.into_token_stream()]
            }
        }
    }

    fn presenter(&self, accumulator: &TokenStream) -> TokenStream {
        match self.quantity {
            Quantity::MaybeOne | Quantity::One | Quantity::AssumeOne => {
                quote! { #accumulator }
            }
            Quantity::AtLeastZero | Quantity::AtLeastOne => {
                quote! {
                    ::benzina::__private::std::iter::Iterator::collect::<::benzina::__private::std::vec::Vec<_>>(
                        ::benzina::__private::IndexMap::into_values(#accumulator)
                    )
                }
            }
        }
    }
}
