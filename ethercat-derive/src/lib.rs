// Part of ethercat-rs. Copyright 2018-2019 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

//! Support for deriving ethercat-plc traits for a struct.

extern crate proc_macro;  // needed even in 2018

use self::proc_macro::TokenStream;
use syn::parse_macro_input;
use quote::quote;
use quote::ToTokens;


#[proc_macro_derive(SlaveProcessImage, attributes(slave_id, pdos, entry))]
pub fn derive_single_process_image(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let ident = input.ident;

    let id_str = ident.to_string();
    let slave_id = if id_str.starts_with("EK") {
        let nr = id_str[2..6].parse::<u32>().unwrap();
        quote!(ethercat::SlaveId { vendor_id: 2, product_code: (#nr << 16) | 0x2c52 })
    } else if id_str.starts_with("EL") {
        let nr = id_str[2..6].parse::<u32>().unwrap();
        quote!(ethercat::SlaveId { vendor_id: 2, product_code: (#nr << 16) | 0x3052 })
    } else {
        panic!("cannot interpret struct name '{}' into a slave ID", id_str);
    };

    let mut sync_infos = vec![];
    let mut pdo_regs = vec![];
    let mut running_size = 0usize;
    let mut pdo_mapping = std::collections::HashMap::new();

    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(flds), ..
    }) = input.data {
        for field in flds.named {
            let ty = field.ty.into_token_stream().to_string();
            let bitlen = match &*ty {
                "u8"  | "i8"  => 8,
                "u16" | "i16" => 16,
                "u32" | "i32" | "f32" => 32,
                "u64" | "i64" | "f64" => 64,
                _ => panic!("cannot handle type '{}' in image", ty)
            };
            for attr in &field.attrs {
                if attr.path.is_ident("entry") {
                    if let syn::Meta::List(syn::MetaList { nested, .. }) =
                        attr.parse_meta().unwrap()
                    {
                        let (pdo_str, ix, subix) = if nested.len() == 2 {
                            ("".into(), &nested[0], &nested[1])
                        } else {
                            let pdo = &nested[0];
                            (quote!(#pdo).to_string(), &nested[1], &nested[2])
                        };
                        pdo_regs.push(quote! {
                            (ethercat::PdoEntryIndex { index: #ix,
                                                       subindex: #subix },
                             ethercat::Offset { byte: #running_size, bit: 0 })
                        });
                        pdo_mapping.entry(pdo_str).or_insert_with(Vec::new).push(quote! {
                            ethercat::PdoEntryInfo {
                                index: PdoEntryIndex { index: #ix, subindex: #subix },
                                bit_length: #bitlen as u8,
                            }
                        });
                    }
                }
            }
            running_size += bitlen / 8;
        }
    } else {
        panic!("SlaveProcessImage must be a struct with named fields");
    }

    for attr in &input.attrs {
        if attr.path.is_ident("pdos") {
            if let syn::Meta::List(syn::MetaList { nested, .. }) =
                attr.parse_meta().unwrap()
            {
                let sm = &nested[0];
                let sd = &nested[1];
                let mut pdos = vec![];
                for pdo_index in nested.iter().skip(2) {
                    let pdo_str = quote!(#pdo_index).to_string();
                    let entries = &pdo_mapping.get(&pdo_str).map_or(&[][..], |v| &*v);
                    pdos.push(quote! {
                        ethercat::PdoInfo {
                            index: #pdo_index,
                            entries: {
                                const ENTRIES: &[ethercat::PdoEntryInfo] =
                                    &[#( #entries ),*]; ENTRIES
                            }
                        }
                    })
                }
                sync_infos.push(quote! {
                    ethercat::SyncInfo {
                        index: #sm,
                        direction: ethercat::SyncDirection::#sd,
                        watchdog_mode: ethercat::WatchdogMode::Default,
                        pdos: {
                            const INFOS: &[ethercat::PdoInfo<'static>] =
                                &[#( #pdos ),*]; INFOS
                        }
                    }
                });
            }
        }
    }

    let sync_infos = if sync_infos.is_empty() {
        quote!(None)
    } else {
        quote!(Some(vec![#( #sync_infos ),*]))
    };

    let generated = quote! {
        #[automatically_derived]
        impl ProcessImage for #ident {
            const SLAVE_COUNT: usize = 1;
            fn get_slave_ids() -> Vec<SlaveId> { vec![#slave_id] }
            fn get_slave_pdos() -> Vec<Option<Vec<SyncInfo<'static>>>> {
                vec![#sync_infos]
            }
            fn get_slave_regs() -> Vec<Vec<(PdoEntryIndex, Offset)>> {
                vec![vec![ #( #pdo_regs ),* ]]
            }
        }
    };

    // println!("{}", generated);
    generated.into()
}


#[proc_macro_derive(ProcessImage, attributes(sdo))]
pub fn derive_process_image(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let ident = input.ident;

    let mut slave_sdos = vec![];
    let mut slave_tys = vec![];

    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(flds), ..
    }) = input.data {
        for field in flds.named {
            let mut sdos = vec![];
            for attr in &field.attrs {
                if attr.path.is_ident("sdo") {
                    if let syn::Meta::List(syn::MetaList { nested, .. }) =
                        attr.parse_meta().unwrap()
                    {
                        let ix = &nested[0];
                        let subix = &nested[1];
                        let data_expr = &nested[2];
                        let data_str = if let syn::NestedMeta::Lit(syn::Lit::Str(s)) = data_expr {
                            syn::parse_str::<syn::Expr>(&s.value()).unwrap()
                        } else {
                            panic!("invalid SDO value, must be stringified")
                        };
                        sdos.push(quote! {
                            (ethercat::SdoIndex { index: #ix, subindex: #subix },
                             Box::new(#data_str))
                        });
                    }
                }
            }
            let ty = field.ty;
            if sdos.is_empty() {
                slave_sdos.push(quote!( res.extend(#ty::get_slave_sdos()); ));
            } else {
                slave_sdos.push(quote!( res.push(vec![#( #sdos ),*]); ));
            }
            slave_tys.push(ty);
        }
    } else {
        return compile_error("only structs with named fields can be a process image");
    }

    let generated = quote! {
        #[automatically_derived]
        impl ProcessImage for #ident {
            const SLAVE_COUNT: usize = #( #slave_tys::SLAVE_COUNT )+*;
            fn get_slave_ids() -> Vec<ethercat::SlaveId> {
                let mut res = vec![]; #( res.extend(#slave_tys::get_slave_ids()); )* res
            }
            fn get_slave_pdos() -> Vec<Option<Vec<ethercat::SyncInfo<'static>>>> {
                let mut res = vec![]; #( res.extend(#slave_tys::get_slave_pdos()); )* res
            }
            fn get_slave_regs() -> Vec<Vec<(ethercat::PdoEntryIndex, ethercat::Offset)>> {
                let mut res = vec![]; #( res.extend(#slave_tys::get_slave_regs()); )* res
            }
            fn get_slave_sdos() -> Vec<Vec<(ethercat::SdoIndex, Box<dyn ethercat::SdoData>)>> {
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
