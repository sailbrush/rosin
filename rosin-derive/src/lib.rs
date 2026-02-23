use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, Type, parse_macro_input, spanned::Spanned};

/// This creates an FFI function that returns the app state's typehash in order to make hot-reload work.
///
/// - The TypeHash trait must also be implemented on the type.
/// - Each application is expected to have only one State struct, so this can only be used once.
///
/// This should only be derived in debug builds.
#[proc_macro_derive(Expose)]
pub fn export_typehash_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = &input.ident;

    if !input.generics.params.is_empty() {
        return syn::Error::new_spanned(&input.generics, "Expose does not support generic types")
            .to_compile_error()
            .into();
    }

    TokenStream::from(quote! {
        #[unsafe(no_mangle)]
        pub extern "C" fn rosin_state_typehash(depth: u64) -> u64 {
            <#ty as ::rosin::typehash::TypeHash>::get_typehash(depth)
        }
    })
}

/// The TypeHash trait is used to check if the in-memory representation of a type has changed between rebuilds.
/// It's likely unsound and is only intended to be used during development.
///
/// This should only be derived in debug builds.
#[proc_macro_derive(TypeHash)]
pub fn type_hash_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Reject direct `dyn Trait` fields
    match &input.data {
        Data::Struct(DataStruct { fields, .. }) => {
            for f in fields.iter() {
                if matches!(&f.ty, Type::TraitObject(_)) {
                    return syn::Error::new(f.ty.span(), "TypeHash: trait objects are not supported in fields (`dyn Trait`)")
                        .to_compile_error()
                        .into();
                }
            }
        }
        Data::Enum(DataEnum { variants, .. }) => {
            for v in variants {
                for f in v.fields.iter() {
                    if matches!(&f.ty, Type::TraitObject(_)) {
                        return syn::Error::new(f.ty.span(), "TypeHash: trait objects are not supported in fields (`dyn Trait`)")
                            .to_compile_error()
                            .into();
                    }
                }
            }
        }
        _ => {}
    }

    let body = match &input.data {
        Data::Struct(DataStruct { fields, .. }) => match fields {
            Fields::Named(named) => {
                let mut per_field = Vec::new();
                let fcount = named.named.len();
                for f in &named.named {
                    let Some(ident) = f.ident.as_ref() else {
                        return syn::Error::new(f.span(), "TypeHash: expected named field identifier").to_compile_error().into();
                    };
                    let fname = syn::LitStr::new(&ident.to_string(), proc_macro2::Span::call_site());
                    let fty = &f.ty;

                    per_field.push(quote! {
                        h.write_tag_str(b"field", #fname);
                        h.write_usize(core::mem::offset_of!(Self, #ident));
                        h.write_u64(<#fty as ::rosin::typehash::TypeHash>::get_typehash(depth - 1));
                    });
                }

                quote! {
                    h.write_tag(b"struct");
                    h.write_usize(#fcount);
                    #(#per_field)*
                }
            }
            Fields::Unnamed(unnamed) => {
                let mut per_field = Vec::new();
                let fcount = unnamed.unnamed.len();
                for (i, f) in unnamed.unnamed.iter().enumerate() {
                    let idx = i as u64;
                    let fty = &f.ty;
                    per_field.push(quote! {
                        h.write_tag(b"field");
                        h.write_u64(#idx);
                        h.write_u64(<#fty as ::rosin::typehash::TypeHash>::get_typehash(depth - 1));
                    });
                }

                quote! {
                    h.write_tag(b"tuple_struct");
                    h.write_usize(#fcount);
                    #(#per_field)*
                }
            }
            Fields::Unit => quote! {
                h.write_tag(b"unit_struct");
            },
        },
        Data::Enum(DataEnum { variants, .. }) => {
            let mut per_variant = Vec::new();

            for v in variants {
                let vname = syn::LitStr::new(&v.ident.to_string(), proc_macro2::Span::call_site());

                let fields_body = match &v.fields {
                    Fields::Named(named) => {
                        let mut pf = Vec::new();
                        let fcount = named.named.len();
                        for f in &named.named {
                            let Some(ident) = f.ident.as_ref() else {
                                return syn::Error::new(f.span(), "TypeHash: expected named field identifier").to_compile_error().into();
                            };
                            let fname = syn::LitStr::new(&ident.to_string(), proc_macro2::Span::call_site());
                            let fty = &f.ty;
                            pf.push(quote! {
                                h.write_tag_str(b"field", #fname);
                                h.write_u64(<#fty as ::rosin::typehash::TypeHash>::get_typehash(depth - 1));
                            });
                        }
                        quote! {
                            h.write_tag(b"named");
                            h.write_usize(#fcount);
                            #(#pf)*
                        }
                    }
                    Fields::Unnamed(unnamed) => {
                        let mut pf = Vec::new();
                        let fcount = unnamed.unnamed.len();
                        for (i, f) in unnamed.unnamed.iter().enumerate() {
                            let idx = i as u64;
                            let fty = &f.ty;
                            pf.push(quote! {
                                h.write_tag(b"field");
                                h.write_u64(#idx);
                                h.write_u64(<#fty as ::rosin::typehash::TypeHash>::get_typehash(depth - 1));
                            });
                        }
                        quote! {
                            h.write_tag(b"unnamed");
                            h.write_usize(#fcount);
                            #(#pf)*
                        }
                    }
                    Fields::Unit => quote! { h.write_tag(b"unit"); },
                };

                per_variant.push(quote! {
                    h.write_tag(b"variant");
                    h.write_tag_str(b"name", #vname);
                    #fields_body
                });
            }

            let vcount = variants.len();
            quote! {
                h.write_tag(b"enum");
                h.write_usize(#vcount);
                #(#per_variant)*
            }
        }
        _ => {
            return syn::Error::new_spanned(name, "TypeHash: Unsupported type").to_compile_error().into();
        }
    };

    let expanded = quote! {
        impl #impl_generics ::rosin::typehash::TypeHash for #name #ty_generics #where_clause {
            fn get_typehash(depth: u64) -> u64 {
                use core::mem::{size_of, align_of};

                if depth == 0 { return 1; }

                struct Fnv1a64(u64);
                impl Fnv1a64 {
                    const OFFSET: u64 = 0xcbf29ce484222325;
                    const PRIME:  u64 = 0x00000100000001B3;

                    #[inline] fn new() -> Self { Self(Self::OFFSET) }

                    #[inline] fn write_bytes(&mut self, bytes: &[u8]) {
                        for &b in bytes {
                            self.0 ^= b as u64;
                            self.0 = self.0.wrapping_mul(Self::PRIME);
                        }
                    }

                    #[inline] fn write_u64(&mut self, v: u64) { self.write_bytes(&v.to_le_bytes()); }
                    #[inline] fn write_usize(&mut self, v: usize) { self.write_u64(v as u64); }

                    #[inline] fn write_tag(&mut self, tag: &[u8]) {
                        self.write_bytes(tag);
                        self.write_bytes(&[0u8]);
                    }

                    #[inline] fn write_tag_str(&mut self, tag: &[u8], s: &str) {
                        self.write_tag(tag);
                        self.write_usize(s.len());
                        self.write_bytes(s.as_bytes());
                    }

                    #[inline] fn finish(self) -> u64 { self.0 }
                }

                let mut h = Fnv1a64::new();

                h.write_usize(size_of::<Self>());
                h.write_usize(align_of::<Self>());

                #body

                h.finish()
            }
        }
    };

    TokenStream::from(expanded)
}
