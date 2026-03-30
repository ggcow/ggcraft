extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use std::fs;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, LitStr, Token};

struct EnumArgs {
    name: LitStr,
    _comma: Token![,],
    folder: LitStr,
}

impl Parse for EnumArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(EnumArgs {
            name: input.parse()?,
            _comma: input.parse()?,
            folder: input.parse()?,
        })
    }
}

fn to_camel_case(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|x| !x.is_empty())
        .map(|word| {
            let mut c = word.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect()
}

#[proc_macro]
pub fn generate_enum_from_files(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as EnumArgs);

    let enum_name = Ident::new(&args.name.value(), proc_macro2::Span::call_site());
    let folder_path = args.folder.value();

    let mut variants = vec![];
    let mut file_paths = vec![];
    let mut file_stems = vec![];
    let mut file_names = vec![];

    let abs_folder = std::fs::canonicalize(&folder_path).expect("dossier invalide");

    if let Ok(entries) = fs::read_dir(&abs_folder) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                file_names.push(file_name.to_string());
                let stem = file_name.split('.').next().unwrap();

                let variant_name = to_camel_case(stem);
                let ident = Ident::new(&variant_name, proc_macro2::Span::call_site());

                variants.push(ident);
                file_stems.push(stem.to_string());

                let abs_path = abs_folder.join(file_name);
                let path_str = abs_path.to_str().unwrap().to_string();
                file_paths.push(path_str);
            }
        }
    }

    let expanded = quote! {
        #[derive(Debug, Clone, Copy)]
        #[repr(u32)]
        pub enum #enum_name {
            #(#variants),*
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #( #enum_name::#variants => write!(f, stringify!(#variants)) ),*
                }
            }
        }

        impl #enum_name {
            pub const ALL: &'static [#enum_name] = &[
                #( #enum_name::#variants ),*
            ];

            pub fn path(&self) -> &'static str {
                match self {
                    #( #enum_name::#variants => #file_paths ),*
                }
            }

            pub fn from_stem(stem: &str) -> Option<Self> {
                match stem {
                    #( #file_stems => Some(#enum_name::#variants), )*
                    _ => None,
                }
            }

            pub fn name(&self) -> &'static str {
                match self {
                    #( #enum_name::#variants => #file_names ),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
