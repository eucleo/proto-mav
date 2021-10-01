use quote::Ident;
use std::io::Write;

pub fn generate<W: Write>(modules: &[String], out: &mut W) {
    let modules_tokens = modules.iter().map(|module| {
        let module_ident = Ident::from(module.clone());

        quote! {
            pub mod #module_ident;
        }
    });

    let tokens = quote! {
        #(#modules_tokens)*
    };

    writeln!(out, "{}", tokens).unwrap();
}

pub fn generate_bare<W: Write>(modules: &[String], out: &mut W) {
    let modules_tokens = modules.iter().map(|module| {
        let module_ident = Ident::from(module.clone());

        quote! {
            pub mod #module_ident;
        }
    });

    let tokens = quote! {
        #(#modules_tokens)*
    };

    writeln!(out, "{}", tokens).unwrap();
}
