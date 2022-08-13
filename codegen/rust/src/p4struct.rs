use crate::Context;
use p4::ast::{Struct, Type, AST};
use quote::{format_ident, quote};

pub(crate) struct StructGenerator<'a> {
    ast: &'a AST,
    ctx: &'a mut Context,
}

impl<'a> StructGenerator<'a> {
    pub(crate) fn new(ast: &'a AST, ctx: &'a mut Context) -> Self {
        Self { ast, ctx }
    }

    pub(crate) fn generate(&mut self) {
        for s in &self.ast.structs {
            self.generate_struct(s);
        }
    }

    fn generate_struct(&mut self, s: &Struct) {
        let mut members = Vec::new();
        let mut valid_member_size = Vec::new();
        let mut to_bitvec_stmts = Vec::new();
        let mut dump_statements = Vec::new();
        let fmt = "{}: {}\n".repeat(s.members.len());
        let fmt = fmt.trim();

        for member in &s.members {
            let name = format_ident!("{}", member.name);
            let name_s = &member.name;
            match &member.ty {
                Type::UserDefined(ref typename) => {
                    if let Some(_) = self.ast.get_header(typename) {
                        let ty = format_ident!("{}", typename);

                        // member generation
                        members.push(quote! { pub #name: #ty });

                        // valid header size statements
                        valid_member_size.push(quote! {
                            if self.#name.valid {
                                x += #ty::size();
                            }
                        });

                        // to bitvec statements
                        to_bitvec_stmts.push(quote!{
                            if self.#name.valid {
                                x[off..off+#ty::size()] |= self.#name.to_bitvec();
                                off += #ty::size();
                            }
                        });

                        dump_statements.push(quote! {
                            #name_s.blue(),
                            self.#name.dump()
                        });
                    } else {
                        panic!(
                            "Struct member {:#?} undefined in {:#?}",
                            member, s
                        );
                    }
                }
                Type::Bit(_size) => {
                    members.push(quote! { pub #name: BitVec::<u8, Msb0> });
                }
                Type::Bool => {
                    members.push(quote! { pub #name: bool });
                }
                x => {
                    todo!("struct member {}", x)
                }
            }
        }

        let dump = quote! {
            format!(#fmt, #(#dump_statements),*)
        };

        let name = format_ident!("{}", s.name);

        let mut structure = quote! {
            #[derive(Debug, Default)]
            pub struct #name {
                #(#members),*
            }
        };
        if !valid_member_size.is_empty() {
            structure.extend(quote! {
                impl #name {
                    pub fn valid_header_size(&self) -> usize {
                        let mut x: usize = 0;
                        #(#valid_member_size)*
                        x
                    }

                    pub fn to_bitvec(&self) -> BitVec<u8, Msb0> {
                        let mut x =
                            bitvec![u8, Msb0; 0; self.valid_header_size()];
                        let mut off = 0;
                        #(#to_bitvec_stmts)*
                        x
                    }

                    pub fn dump(&self) -> String {
                        #dump
                    }
                }
            })
        }

        self.ctx.structs.insert(s.name.clone(), structure);
    }
}
