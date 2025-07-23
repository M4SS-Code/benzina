use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Ident, Index, LitInt, Token, Type, TypePath, braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub(crate) struct Join {
    input: Ident,
    transformation: Transformation,
}

enum NestedOrNot {
    Nested(Punctuated<Transformation, Token![,]>),
    Not(NoTransformation),
}

struct Transformation {
    quantity: Quantity,
    output_type: Ident,
    entries: Punctuated<(Ident, NestedOrNot), Token![,]>,
}

struct NoTransformation {
    quantity: Quantity,
    tuple_index: usize,
}

#[derive(Debug, Copy, Clone)]
enum Quantity {
    MaybeOne,
    One,
    AtLeastZero,
    AtLeastOne,
}

impl Join {
    fn generate_hashmap(&self) -> TokenStream {
        self.transformation.generate_hashmap_values()
    }

    fn generate_root_filler(&self) -> TokenStream {
        let input = &self.input;
        let row_handlers = self.transformation.row_handlers(None);
        quote! {
            for row in #input {
                #row_handlers
            }
        }
    }
}

impl Parse for Join {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input_ = input.parse()?;
        input.parse::<Token![,]>()?;
        let transformation = input.parse()?;
        input.parse::<Token![,]>()?;

        Ok(Self {
            input: input_,
            transformation,
        })
    }
}

impl ToTokens for Join {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let hashmap = self.generate_hashmap();
        let root_filler = self.generate_root_filler();
        let output = quote! {
            let mut root = #hashmap::new();
            #root_filler

            //root.into_values().map(|element| element).collect::<Vec<_>>()
            todo!()
        };

        tokens.extend(quote! {
            {
                #output
            }
        });
    }
}

impl NestedOrNot {
    fn generate_hashmap_values(&self) -> TokenStream {
        match self {
            Self::Nested(nested) => {
                let values = nested
                    .iter()
                    .flat_map(|item| item.generate_hashmap_values())
                    .collect::<TokenStream>();
                quote! { HashMap::<_, (#values)> }
            }
            Self::Not(not) => not.generate_hashmap_values(),
        }
    }

    fn row_handlers(&self, root_index: usize) -> TokenStream {
        match self {
            Self::Nested(nested) => nested
                .iter()
                .flat_map(|item| item.row_handlers(Some(root_index)))
                .collect(),
            Self::Not(not) => not.row_handlers(root_index),
        }
    }

    fn or_insert(&self, can_be_zero: bool) -> TokenStream {
        match self {
            Self::Nested(nested) => {
                let values = nested
                    .iter()
                    .flat_map(|item| item.or_insert())
                    .collect::<TokenStream>();
                quote! { HashMap::<_, _>::new(), }
            }
            Self::Not(not) => not.or_insert(can_be_zero),
        }
    }
}

impl Parse for NestedOrNot {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(not) = input.fork().parse() {
            let _ = input.parse::<NoTransformation>()?;
            Ok(Self::Not(not))
        } else {
            let conversions = Punctuated::parse_terminated(input)?;
            Ok(Self::Nested(conversions))
        }
    }
}

impl Transformation {
    fn generate_hashmap_values(&self) -> TokenStream {
        let values = self
            .entries
            .iter()
            .flat_map(|(_key, value)| value.generate_hashmap_values())
            .collect::<TokenStream>();
        quote! { HashMap::<_, (#values)> }
    }

    fn row_handlers(&self, root_index: Option<usize>) -> TokenStream {
        let root_index = if let Some(root_index) = root_index {
            let root_index = Index::from(root_index);
            quote! { root.#root_index }
        } else {
            quote! { root }
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

        let (wrapper, unwrapper) = if matches!(self.quantity, Quantity::AtLeastZero) {
            (
                quote! { if row.#one_tuple_index.is_some() },
                quote! { .unwrap() },
            )
        } else {
            (quote! {}, quote! {})
        };

        let or_insert = self.or_insert();
        let entries_mapper = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, (_name, entry))| entry.row_handlers(i))
            .collect::<TokenStream>();

        quote! {
            #wrapper {
                let mut root = #root_index.entry(identifiable_id(&row.#one_tuple_index #unwrapper).clone()).or_insert((#or_insert));
                #entries_mapper
            }
        }
    }

    fn or_insert(&self) -> TokenStream {
        self.entries
            .iter()
            .map(|(_name, entry)| entry.or_insert(matches!(self.quantity, Quantity::AtLeastZero)))
            .collect()
    }
}

impl Parse for Transformation {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let quantity = input.parse()?;
        input.parse::<Token![<]>()?;

        let output_type = input.parse()?;
        let content;
        braced!(content in input);

        let entries = Punctuated::parse_terminated_with(&content, |input| {
            let field = input.parse::<Ident>()?;
            input.parse::<Token![:]>()?;
            let value = input.parse::<NestedOrNot>()?;
            Ok((field, value))
        })?;

        input.parse::<Token![>]>()?;

        Ok(Self {
            quantity,
            output_type,
            entries,
        })
    }
}

impl NoTransformation {
    fn generate_hashmap_values(&self) -> TokenStream {
        quote! { _, }
    }

    fn row_handlers(&self, root_index: usize) -> TokenStream {
        let tuple_index = Index::from(self.tuple_index);
        let root_index = Index::from(root_index);
        match self.quantity {
            Quantity::MaybeOne => quote! {
                if let Some(item) = row.#tuple_index {
                    root.#root_index = Some(item);
                }
            },
            Quantity::One => quote! {},
            Quantity::AtLeastZero => quote! {
                if let Some(item) = row.#tuple_index {
                    root.#root_index.insert(identifiable_id(&item).clone(), item);
                }
            },
            Quantity::AtLeastOne => quote! {
                let item = row.#tuple_index;
                root.#root_index.insert(identifiable_id(&item).clone(), item);
            },
        }
    }

    fn or_insert(&self, can_be_zero: bool) -> TokenStream {
        let tuple_index = Index::from(self.tuple_index);
        match (self.quantity, can_be_zero) {
            (Quantity::MaybeOne, _) => quote! { None, },
            (Quantity::One, false) => quote! { row.#tuple_index, },
            (Quantity::One, true) => quote! { row.#tuple_index.unwrap(), },
            (Quantity::AtLeastZero | Quantity::AtLeastOne, _) => {
                quote! { HashMap::<_, _>::new(), }
            }
        }
    }
}

impl Parse for NoTransformation {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let quantity = input.parse()?;
        input.parse::<Token![<]>()?;
        let tuple_index = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Token![>]>()?;
        Ok(Self {
            quantity,
            tuple_index,
        })
    }
}

impl Parse for Quantity {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let quantity = input.parse::<Ident>()?;
        match &*quantity.to_string() {
            "Option" => Ok(Self::MaybeOne),
            "One" => Ok(Self::One),
            "Vec0" => Ok(Self::AtLeastZero),
            "Vec" => Ok(Self::AtLeastOne),
            raw_quantity => Err(syn::Error::new(
                quantity.span(),
                format!(
                    "Unknown quantity `{raw_quantity}`. Expected `Option`, `One`, `Vec0` or `Vec`"
                ),
            )),
        }
    }
}
