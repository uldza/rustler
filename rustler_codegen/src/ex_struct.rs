use proc_macro2::{Span, TokenStream};

use syn::{self, Field, Ident};

use super::context::Context;
use super::RustlerAttr;

pub fn transcoder_decorator(ast: &syn::DeriveInput) -> TokenStream {
    let ctx = Context::from_ast(ast);

    let elixir_module = get_module(&ctx);

    let struct_fields = ctx
        .struct_fields
        .as_ref()
        .expect("NifStruct can only be used with structs");

    // Unwrap is ok here, as we already determined that struct_fields is not None
    let field_atoms = ctx.field_atoms().unwrap();

    let atom_defs = quote! {
        rustler::atoms! {
            atom_struct = "__struct__",
            atom_module = #elixir_module,
            #(#field_atoms)*
        }
    };

    let atoms_module_name = ctx.atoms_module_name(Span::call_site());

    let decoder = if ctx.decode() {
        gen_decoder(&ctx, &struct_fields, &atoms_module_name)
    } else {
        quote! {}
    };

    let encoder = if ctx.encode() {
        gen_encoder(&ctx, &struct_fields, &atoms_module_name)
    } else {
        quote! {}
    };

    let gen = quote! {
        mod #atoms_module_name {
            #atom_defs
        }

        #decoder
        #encoder
    };

    gen
}

fn gen_decoder(ctx: &Context, fields: &[&Field], atoms_module_name: &Ident) -> TokenStream {
    let struct_type = &ctx.ident_with_lifetime;
    let struct_name = ctx.ident;
    let struct_name_str = struct_name.to_string();

    let idents: Vec<_> = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect();

    let (assignments, field_defs): (Vec<TokenStream>, Vec<TokenStream>) = fields
        .iter()
        .zip(idents.iter())
        .enumerate()
        .map(|(index, (field, ident))| {
            let atom_fun = Context::field_to_atom_fun(field);
            let variable = Context::escape_ident_with_index(&ident.to_string(), index, "struct");

            let assignment = quote! {
                let #variable = try_decode_field(env, term, #atom_fun())?;
            };

            let field_def = quote! {
                #ident: #variable
            };

            (assignment, field_def)
        })
        .unzip();

    let gen = quote! {
        impl<'a> ::rustler::Decoder<'a> for #struct_type {
            fn decode(term: ::rustler::Term<'a>) -> Result<Self, ::rustler::Error> {
                use #atoms_module_name::*;
                use ::rustler::Encoder;

                let env = term.get_env();

                fn try_decode_field<'a, T>(
                    env: rustler::Env<'a>,
                    term: rustler::Term<'a>,
                    field: rustler::Atom,
                    ) -> Result<T, rustler::Error>
                    where
                        T: rustler::Decoder<'a>,
                    {
                        use rustler::Encoder;
                        match ::rustler::Decoder::decode(term.map_get(field.encode(env))?) {
                            Err(_) => Err(::rustler::Error::RaiseTerm(Box::new(format!(
                                            "Could not decode field :{:?} on %{}{{}}",
                                            field, #struct_name_str
                            )))),
                            Ok(value) => Ok(value),
                        }
                    };

                let module: ::rustler::types::atom::Atom = term.map_get(atom_struct().to_term(env))?.decode()?;
                if module != atom_module() {
                    return Err(::rustler::Error::Atom("invalid_struct"));
                }

                #(#assignments);*

                Ok(#struct_name { #(#field_defs),* })
            }
        }
    };

    gen
}

fn gen_encoder(ctx: &Context, fields: &[&Field], atoms_module_name: &Ident) -> TokenStream {
    let struct_type = &ctx.ident_with_lifetime;

    let field_defs: Vec<TokenStream> = fields
        .iter()
        .map(|field| {
            let field_ident = field.ident.as_ref().unwrap();
            let atom_fun = Context::field_to_atom_fun(field);
            quote! {
                map = map.map_put(#atom_fun().encode(env), self.#field_ident.encode(env)).unwrap();
            }
        })
        .collect();

    let gen = quote! {
        impl<'b> ::rustler::Encoder for #struct_type {
            fn encode<'a>(&self, env: ::rustler::Env<'a>) -> ::rustler::Term<'a> {
                use #atoms_module_name::*;
                let mut map = ::rustler::types::map::map_new(env);
                map = map.map_put(atom_struct().encode(env), atom_module().encode(env)).unwrap();
                #(#field_defs)*
                map
            }
        }
    };

    gen
}

fn get_module(ctx: &Context) -> String {
    ctx.attrs
        .iter()
        .find_map(|attr| match attr {
            RustlerAttr::Module(ref module) => Some(module.clone()),
            _ => None,
        })
        .expect("NifStruct requires a 'module' attribute")
}
