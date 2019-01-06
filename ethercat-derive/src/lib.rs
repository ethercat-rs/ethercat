//! Support for deriving ProcessImage for a struct.

#![recursion_limit="128"]

extern crate proc_macro;  // needed even in 2018

use proc_macro::TokenStream;
use syn::parse_macro_input;
use quote::quote;
use quote::ToTokens;


#[proc_macro_derive(SlaveProcessImage, attributes(slave_id, pdo))]
pub fn derive_single_process_image(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let ident = input.ident;

    let id_str = ident.to_string();
    let slave_id = if id_str.starts_with("EK") {
        let nr = id_str[2..].parse::<u32>().unwrap();
        quote!(ethercat::SlaveId { vendor_id: 2, product_code: (#nr << 16) | 0x2c52 })
    } else if id_str.starts_with("EL") {
        let nr = id_str[2..].parse::<u32>().unwrap();
        quote!(ethercat::SlaveId { vendor_id: 2, product_code: (#nr << 16) | 0x2c52 })
    } else {
        panic!("cannot interpret struct name '{}' into a slave ID", id_str);
    };

    let mut pdo_regs = vec![];
    let mut running_size = 0usize;

    if let syn::Data::Struct(syn::DataStruct { fields: syn::Fields::Named(flds), .. }) = input.data {
        for field in flds.named {
            for attr in &field.attrs {
                if attr.path.is_ident("pdo") {
                    if let syn::Meta::List(syn::MetaList { nested, .. }) =
                        attr.parse_meta().unwrap()
                    {
                        let ix = &nested[0];
                        let subix = &nested[1];
                        pdo_regs.push(quote! {
                            (ethercat::PdoEntryIndex { index: #ix,
                                                       subindex: #subix },
                             ethercat::Offset { byte: #running_size, bit: 0 })
                        });
                    }
                }
            }
            let ty = field.ty.into_token_stream().to_string();
            match &*ty {
                "u8"  | "i8"  => running_size += 1,
                "u16" | "i16" => running_size += 2,
                "u32" | "i32" | "f32" => running_size += 4,
                "u64" | "i64" | "f64" => running_size += 8,
                _ => panic!("cannot handle type '{}' in image", ty)
            }
        }
    } else {
        panic!("SlaveProcessImage must be a struct with named fields");
    }

    let generated = quote! {
        #[automatically_derived]
        impl ProcessImage for #ident {
            const SLAVE_COUNT: usize = 1;
            fn get_slave_ids() -> Vec<SlaveId> { vec![#slave_id] }
            fn get_slave_regs() -> Vec<Vec<(PdoEntryIndex, Offset)>> {
                vec![vec![ #( #pdo_regs ),* ]]
            }
        }
    };

    // println!("{}", generated);
    generated.into()
}


#[proc_macro_derive(ProcessImage, attributes(plc))]
pub fn derive_process_image(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let ident = input.ident;

    let mut slave_count = vec![];
    let mut slave_ids = vec![];
    let mut slave_pdos = vec![];
    let mut slave_regs = vec![];
    let mut slave_sdos = vec![];

    if let syn::Data::Struct(syn::DataStruct { fields: syn::Fields::Named(flds), .. }) = input.data {
        for field in flds.named {
            let ty = field.ty;
            slave_count.push(quote!( #ty :: SLAVE_COUNT ));
            slave_ids.push(quote!( res.extend(#ty::get_slave_ids()); ));
            slave_pdos.push(quote!( res.extend(#ty::get_slave_pdos()); ));
            slave_regs.push(quote!( res.extend(#ty::get_slave_regs()); ));
            slave_sdos.push(quote!( res.extend(#ty::get_slave_sdos()); ));
        }
    } else {
        return compile_error("only structs with named fields can be a process image");
    }

    let generated = quote! {
        #[automatically_derived]
        impl ProcessImage for #ident {
            const SLAVE_COUNT: usize = #(#slave_count)+*;
            fn get_slave_ids() -> Vec<ethercat::SlaveId> {
                let mut res = vec![]; #(#slave_ids)* res
            }
            fn get_slave_pdos() -> Vec<Option<Vec<ethercat::SyncInfo<'static>>>> {
                let mut res = vec![]; #(#slave_pdos)* res
            }
            fn get_slave_regs() -> Vec<Vec<(ethercat::PdoEntryIndex, ethercat::Offset)>> {
                let mut res = vec![]; #(#slave_regs)* res
            }
            fn get_slave_sdos() -> Vec<Vec<()>> {
                let mut res = vec![]; #(#slave_sdos)* res
            }
        }
    };

    // println!("{}", generated);
    generated.into()
}

#[proc_macro_derive(ExternImage, attributes(plc))]
pub fn derive_extern_image(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let ident = input.ident;

    // currently a no-op, later: auto-generate Default from #[plc] attributes
    let generated = quote! {
        impl ExternImage for #ident {}
    };
    generated.into()
}

fn compile_error(message: impl Into<String>) -> TokenStream {
    let message = message.into();
    quote!(compile_error! { #message }).into()
}
