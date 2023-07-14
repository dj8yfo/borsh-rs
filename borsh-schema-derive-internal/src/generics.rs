use std::collections::{HashMap, HashSet};

use quote::quote;
use syn::{
    punctuated::Pair, Field, GenericArgument, Generics, Ident, Macro, Path, PathArguments,
    PathSegment, ReturnType, Type, TypeParamBound, TypePath, WherePredicate,
};

pub fn compute_predicates(params: Vec<Type>, traitname: &Path) -> Vec<WherePredicate> {
    params
        .into_iter()
        .map(|param| {
            syn::parse2(quote! {
                #param: #traitname
            })
            .unwrap()
        })
        .collect()
}

// Remove the default from every type parameter because in the generated impls
// they look like associated types: "error: associated type bindings are not
// allowed here".
pub fn without_defaults(generics: &Generics) -> Generics {
    syn::Generics {
        params: generics
            .params
            .iter()
            .map(|param| match param {
                syn::GenericParam::Type(param) => syn::GenericParam::Type(syn::TypeParam {
                    eq_token: None,
                    default: None,
                    ..param.clone()
                }),
                _ => param.clone(),
            })
            .collect(),
        ..generics.clone()
    }
}

/// a Visitor-like struct, which helps determine, if a type parameter is found in field
#[derive(Clone)]
pub struct FindTyParams {
    // Set of all generic type parameters on the current struct . Initialized up front.
    all_type_params: HashSet<Ident>,
    all_type_params_ordered: Vec<Ident>,

    // Set of generic type parameters used in fields for which filter
    // returns true . Filled in as the visitor sees them.
    relevant_type_params: HashSet<Ident>,

    // [Param] => [Type, containing Param] mapping
    associated_type_params_usage: HashMap<Ident, Type>,
}

fn ungroup(mut ty: &Type) -> &Type {
    while let Type::Group(group) = ty {
        ty = &group.elem;
    }
    ty
}

impl FindTyParams {
    pub fn new(generics: &Generics) -> Self {
        let all_type_params = generics
            .type_params()
            .map(|param| param.ident.clone())
            .collect();

        let all_type_params_ordered = generics
            .type_params()
            .map(|param| param.ident.clone())
            .collect();

        FindTyParams {
            all_type_params,
            all_type_params_ordered,
            relevant_type_params: HashSet::new(),
            associated_type_params_usage: HashMap::new(),
        }
    }
    pub fn process_for_bounds(self) -> Vec<Type> {
        let relevant_type_params = self.relevant_type_params;
        let associated_type_params_usage = self.associated_type_params_usage;
        let mut new_predicates: Vec<Type> = vec![];

        self.all_type_params_ordered.iter().for_each(|param| {
            if relevant_type_params.contains(param) {
                let ty = Type::Path(TypePath {
                    qself: None,
                    path: param.clone().into(),
                });
                new_predicates.push(ty);
            }
            if let Some(type_) = associated_type_params_usage.get(param) {
                new_predicates.push(type_.clone());
            }
        });

        new_predicates
    }
    pub fn process_for_params(self) -> Vec<Ident> {
        let relevant_type_params = self.relevant_type_params;
        let associated_type_params_usage = self.associated_type_params_usage;

        let mut params: Vec<Ident> = vec![];
        self.all_type_params_ordered.iter().for_each(|param| {
            if relevant_type_params.contains(param) {
                params.push(param.clone());
            }
            if associated_type_params_usage.get(param).is_some() {
                params.push(param.clone());
            }
        });
        params
    }
}

impl FindTyParams {
    pub fn visit_field(&mut self, field: &Field) {
        if let Type::Path(ty) = ungroup(&field.ty) {
            if let Some(Pair::Punctuated(t, _)) = ty.path.segments.pairs().next() {
                if self.all_type_params.contains(&t.ident) {
                    self.associated_type_params_usage
                        .insert(t.ident.clone(), field.ty.clone());
                }
            }
        }
        self.visit_type(&field.ty);
    }

    #[allow(unused)]
    pub fn insert_type(&mut self, param: Ident, type_: Type) {
        self.associated_type_params_usage.insert(param, type_);
    }

    fn visit_return_type(&mut self, return_type: &ReturnType) {
        match return_type {
            ReturnType::Default => {}
            ReturnType::Type(_, output) => self.visit_type(output),
        }
    }

    fn visit_path_segment(&mut self, segment: &PathSegment) {
        self.visit_path_arguments(&segment.arguments);
    }

    fn visit_path_arguments(&mut self, arguments: &PathArguments) {
        match arguments {
            PathArguments::None => {}
            PathArguments::AngleBracketed(arguments) => {
                for arg in &arguments.args {
                    match arg {
                        GenericArgument::Type(arg) => self.visit_type(arg),
                        GenericArgument::AssocType(arg) => self.visit_type(&arg.ty),
                        GenericArgument::Lifetime(_)
                        | GenericArgument::Const(_)
                        | GenericArgument::AssocConst(_)
                        | GenericArgument::Constraint(_) => {}
                        #[cfg_attr(
                            feature = "force_exhaustive_checks",
                            deny(non_exhaustive_omitted_patterns)
                        )]
                        _ => {}
                    }
                }
            }
            PathArguments::Parenthesized(arguments) => {
                for argument in &arguments.inputs {
                    self.visit_type(argument);
                }
                self.visit_return_type(&arguments.output);
            }
        }
    }

    fn visit_path(&mut self, path: &Path) {
        if let Some(seg) = path.segments.last() {
            if seg.ident == "PhantomData" {
                // Hardcoded exception, because PhantomData<T> implements
                // Serialize and Deserialize and Schema whether or not T implements it.
                return;
            }
        }
        if path.leading_colon.is_none() && path.segments.len() == 1 {
            let id = &path.segments[0].ident;
            if self.all_type_params.contains(id) {
                self.relevant_type_params.insert(id.clone());
            }
        }
        for segment in &path.segments {
            self.visit_path_segment(segment);
        }
    }

    fn visit_type_param_bound(&mut self, bound: &TypeParamBound) {
        match bound {
            TypeParamBound::Trait(bound) => self.visit_path(&bound.path),
            TypeParamBound::Lifetime(_) | TypeParamBound::Verbatim(_) => {}
            #[cfg_attr(
                feature = "force_exhaustive_checks",
                deny(non_exhaustive_omitted_patterns)
            )]
            _ => {}
        }
    }
    // Type parameter should not be considered used by a macro path.
    //
    //     struct TypeMacro<T> {
    //         mac: T!(),
    //         marker: PhantomData<T>,
    //     }
    fn visit_macro(&mut self, _mac: &Macro) {}

    fn visit_type(&mut self, ty: &Type) {
        match ty {
            Type::Array(ty) => self.visit_type(&ty.elem),
            Type::BareFn(ty) => {
                for arg in &ty.inputs {
                    self.visit_type(&arg.ty);
                }
                self.visit_return_type(&ty.output);
            }
            Type::Group(ty) => self.visit_type(&ty.elem),
            Type::ImplTrait(ty) => {
                for bound in &ty.bounds {
                    self.visit_type_param_bound(bound);
                }
            }
            Type::Macro(ty) => self.visit_macro(&ty.mac),
            Type::Paren(ty) => self.visit_type(&ty.elem),
            Type::Path(ty) => {
                if let Some(qself) = &ty.qself {
                    self.visit_type(&qself.ty);
                }
                self.visit_path(&ty.path);
            }
            Type::Ptr(ty) => self.visit_type(&ty.elem),
            Type::Reference(ty) => self.visit_type(&ty.elem),
            Type::Slice(ty) => self.visit_type(&ty.elem),
            Type::TraitObject(ty) => {
                for bound in &ty.bounds {
                    self.visit_type_param_bound(bound);
                }
            }
            Type::Tuple(ty) => {
                for elem in &ty.elems {
                    self.visit_type(elem);
                }
            }

            Type::Infer(_) | Type::Never(_) | Type::Verbatim(_) => {}

            #[cfg_attr(
                feature = "force_exhaustive_checks",
                deny(non_exhaustive_omitted_patterns)
            )]
            _ => {}
        }
    }
}