use std::collections::BTreeMap;

use proc_macro2::{Span, TokenStream};
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

    fn generate_root_converter(&self) -> TokenStream {
        let root = quote! { root };
        self.transformation.root_converter(&root)
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
        let root_converter = self.generate_root_converter();
        let output = quote! {
            let mut root = #hashmap::new();
            #root_filler
            #root_converter
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
            Self::Nested(nested) => nested
                .iter()
                .flat_map(|item| item.generate_hashmap_values())
                .collect::<TokenStream>(),
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

    fn root_converter(&self, root: &TokenStream) -> TokenStream {
        match self {
            Self::Nested(nested) => nested
                .iter()
                .flat_map(|item| item.root_converter(root))
                .collect::<TokenStream>(),
            Self::Not(not) => not.root_converter(root),
        }
    }

    fn or_insert(&self, tuple_index_overwrites: &BTreeMap<usize, TokenStream>) -> TokenStream {
        match self {
            Self::Nested(nested) => {
                let values = nested
                    .iter()
                    .flat_map(|item| item.or_insert(tuple_index_overwrites))
                    .collect::<TokenStream>();
                quote! { HashMap::<_, _>::new(), }
            }
            Self::Not(not) => not.or_insert(tuple_index_overwrites),
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
            .flat_map(|(key, value)| value.generate_hashmap_values())
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

        let mut tuple_index_overwrites = BTreeMap::new();
        let wrapper = if matches!(self.quantity, Quantity::AtLeastZero) {
            let name = Ident::new(&format!("unwrapped{}", one.tuple_index), Span::call_site());
            tuple_index_overwrites.insert(one.tuple_index, quote! { #name });
            quote! { if let Some(#name) = row.#one_tuple_index }
        } else {
            quote! {}
        };

        let or_insert = self.or_insert(&tuple_index_overwrites);
        let entries_mapper = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, (_name, entry))| entry.row_handlers(i))
            .collect::<TokenStream>();

        let one_name = if let Some(overwrite) = tuple_index_overwrites.get(&one.tuple_index) {
            overwrite.clone()
        } else {
            quote! { row.#one_tuple_index }
        };
        quote! {
            #wrapper {
                let mut root = #root_index.entry(identifiable_id(&#one_name).clone()).or_insert((#or_insert));
                #entries_mapper
            }
        }
    }

    fn root_converter(&self, root: &TokenStream) -> TokenStream {
        let Self {
            quantity,
            output_type,
            entries,
        } = self;

        let entries = entries
            .iter()
            .enumerate()
            .map(|(i, (name, entry))| {
                let item = Ident::new("item", Span::call_site());
                let ii = Index::from(i);
                let item = quote! { #item.#ii };
                let entry = entry.root_converter(&item);
                quote! {
                    #name: #entry,
                }
            })
            .collect::<TokenStream>();
        quote! { #root.into_iter().map(|item| #output_type {
            #entries
        }).collect::<Vec<_>>() }
    }

    fn or_insert(&self, tuple_index_overwrites: &BTreeMap<usize, TokenStream>) -> TokenStream {
        self.entries
            .iter()
            .map(|(_name, entry)| entry.or_insert(tuple_index_overwrites))
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
        match self.quantity {
            Quantity::MaybeOne => quote! {
                Option<_>,
            },
            Quantity::One => quote! {
                _,
            },
            Quantity::AtLeastZero => quote! {
                HashMap::<_, _>,
            },
            Quantity::AtLeastOne => quote! {
                HashMap::<_, _>,
            },
        }
    }

    fn row_handlers(&self, root_index: usize) -> TokenStream {
        let tuple_index = Index::from(self.tuple_index);
        let root_index = Index::from(root_index);
        match self.quantity {
            Quantity::MaybeOne => quote! {
                {
                    if let Some(item) = row.#tuple_index {
                        root.#root_index = Some(item);
                    }
                }
            },
            Quantity::One => quote! {},
            Quantity::AtLeastZero => quote! {
                {
                    if let Some(item) = row.#tuple_index {
                        root.#root_index.entry(identifiable_id(&item).clone()).or_insert(item);
                    }
                }
            },
            Quantity::AtLeastOne => quote! {
                {
                    let item = row.#tuple_index;
                    root.#root_index.entry(identifiable_id(&item).clone()).or_insert(item);
                }
            },
        }
    }

    fn root_converter(&self, root: &TokenStream) -> TokenStream {
        match self.quantity {
            Quantity::MaybeOne | Quantity::One => {
                quote! { #root }
            }
            Quantity::AtLeastZero | Quantity::AtLeastOne => {
                quote! { #root.into_values().collect::<Vec<_>>() }
            }
        }
    }

    fn or_insert(&self, tuple_index_overwrites: &BTreeMap<usize, TokenStream>) -> TokenStream {
        match self.quantity {
            Quantity::MaybeOne => quote! { None, },
            Quantity::One => {
                if let Some(overwrite) = tuple_index_overwrites.get(&self.tuple_index) {
                    quote! { #overwrite, }
                } else {
                    let tuple_index = Index::from(self.tuple_index);
                    quote! { row.#tuple_index, }
                }
            }
            Quantity::AtLeastZero | Quantity::AtLeastOne => {
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
