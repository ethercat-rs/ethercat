//! Support for deriving ProcessImage for a struct.

#![recursion_limit="128"]

extern crate proc_macro;  // needed even in 2018

use proc_macro::TokenStream;
use syn::parse_macro_input;
use quote::quote;

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
