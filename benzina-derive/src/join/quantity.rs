use syn::{
    Ident,
    parse::{Parse, ParseStream},
};

#[derive(Debug, Copy, Clone)]
pub(super) enum Quantity {
    MaybeOne,
    One,
    AssumeOne,
    AtLeastZero,
    AtLeastOne,
}

impl Parse for Quantity {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let quantity = input.parse::<Ident>()?;
        match &*quantity.to_string() {
            "Option" => Ok(Self::MaybeOne),
            "One" => Ok(Self::One),
            "AssumeOne" => Ok(Self::AssumeOne),
            "Vec0" => Ok(Self::AtLeastZero),
            "Vec" => Ok(Self::AtLeastOne),
            raw_quantity => Err(syn::Error::new(
                quantity.span(),
                format!(
                    "Unknown quantity `{raw_quantity}`. Expected `Option`, `One`, `AssumeOne`, `Vec0` or `Vec`"
                ),
            )),
        }
    }
}
