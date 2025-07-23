use syn::{
    Ident, LitInt, Token, braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use super::{Join, NestedOrNot, NoTransformation, Transformation};

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

impl Parse for NestedOrNot {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(not) = input.fork().parse() {
            // FIXME: can we advance the above `fork`?
            let _ = input.parse::<NoTransformation>()?;
            Ok(Self::Not(not))
        } else {
            let conversions = Punctuated::parse_terminated(input)?;
            Ok(Self::Nested(conversions))
        }
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
