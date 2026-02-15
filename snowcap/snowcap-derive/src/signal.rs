use syn::{GenericParam, Generics, parse_quote};

pub fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(Clone));
            type_param.bounds.push(parse_quote!('static));
        }
    }

    generics
}
