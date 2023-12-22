use std::borrow::Cow;

use crate::type_ref::{TemplateArg, TypeRef, TypeRefDesc, TypeRefKind};
use crate::{CppNameStyle, Element, IteratorExt, StringExt};

pub trait TypeRefRenderer<'a> {
	type Recursed: TypeRefRenderer<'a> + Sized;

	fn render<'t>(self, type_ref: &'t TypeRef) -> Cow<'t, str>;
	fn recurse(&self) -> Self::Recursed;
}

#[derive(Debug)]
pub struct CppRenderer<'s> {
	pub name_style: CppNameStyle,
	pub name: &'s str,
	/// true for rendering in extern contexts, references are treated as pointers, this is used for declaring the types of
	/// callbacks in C++ code and for `cpp_safe_id`
	pub extern_types: bool,
}

impl<'s> CppRenderer<'s> {
	pub fn new(name_style: CppNameStyle, name: &'s str, extern_types: bool) -> Self {
		Self {
			name_style,
			name,
			extern_types,
		}
	}
}

impl<'a> TypeRefRenderer<'a> for CppRenderer<'_> {
	type Recursed = Self;

	fn render<'t>(self, type_ref: &'t TypeRef) -> Cow<'t, str> {
		let cnst = type_ref.inherent_constness().cpp_qual();
		let (space_name, space_const_name) = if self.name.is_empty() {
			("".to_string(), "".to_string())
		} else {
			(format!(" {}", self.name), format!(" {}{}", cnst, self.name))
		};
		let kind = type_ref.kind();
		match kind.as_ref() {
			TypeRefKind::Primitive(_, cpp) => {
				format!("{cnst}{cpp}{space_name}")
			}
			TypeRefKind::Array(inner, size) => {
				if let Some(size) = size {
					if self.name.is_empty() {
						format!("{cnst}{typ}**", typ = self.recurse().render(inner))
					} else {
						format!(
							"{cnst}{typ}(*{name})[{size}]",
							typ = self.recurse().render(inner),
							name = self.name
						)
					}
				} else {
					format!("{typ}*{space_name}", typ = self.recurse().render(inner))
				}
			}
			TypeRefKind::StdVector(vec) => {
				format!(
					"{cnst}{vec_type}<{elem_type}>{space_name}",
					vec_type = vec.cpp_name(self.name_style),
					elem_type = self.recurse().render(&vec.element_type()),
				)
			}
			TypeRefKind::StdTuple(tuple) => {
				let elem_types = tuple
					.elements()
					.iter()
					.map(|tref| {
						// fixme: hack to keep backwards compatible behavior after tuple rendering changes
						// ideal fix would be to use CppName::Reference in the recurse() function globally
						// but it changes the function identifier generation
						let mut renderer = self.recurse();
						renderer.name_style = CppNameStyle::Reference;
						renderer.render(tref)
					})
					.join(", ");
				format!("{cnst}{typ}<{elem_types}>{space_name}", typ = tuple.cpp_name(self.name_style))
			}
			TypeRefKind::Reference(inner) if !self.extern_types => {
				format!("{typ}&{name}", typ = self.recurse().render(inner), name = space_const_name)
			}
			TypeRefKind::RValueReference(inner) if !self.extern_types => {
				format!("{typ}&&{name}", typ = self.recurse().render(inner), name = space_const_name)
			}
			TypeRefKind::Pointer(inner) | TypeRefKind::Reference(inner) | TypeRefKind::RValueReference(inner) => {
				format!("{typ}*{space_const_name}", typ = self.recurse().render(inner))
			}
			TypeRefKind::SmartPtr(ptr) => {
				format!(
					"{cnst}{ptr_type}<{inner_type}>{space_name}",
					ptr_type = ptr.cpp_name(self.name_style),
					inner_type = self.recurse().render(&ptr.pointee()),
				)
			}
			TypeRefKind::Class(cls) => {
				let mut out = cls.cpp_name(self.name_style).into_owned();
				if !kind.is_std_string(type_ref.type_hint()) {
					// fixme prevents emission of std::string<char>
					out += &render_cpp_tpl(self, type_ref);
				}
				format!("{cnst}{out}{space_name}")
			}
			TypeRefKind::Enum(enm) => {
				format!("{cnst}{typ}{space_name}", typ = enm.cpp_name(self.name_style))
			}
			TypeRefKind::Typedef(tdef) => {
				let underlying_type = tdef.underlying_type_ref();
				let typ = if underlying_type.kind().as_reference().is_some() {
					// references can't be used in lvalue position
					self.recurse().render(&underlying_type)
				} else {
					tdef.cpp_name(self.name_style).into_owned().into()
				};
				format!("{cnst}{typ}{space_name}")
			}
			TypeRefKind::Generic(generic_name) => {
				format!("{cnst}{generic_name}{space_name}")
			}
			TypeRefKind::Function(func) => {
				let mut typ = func.cpp_name(self.name_style);
				if typ.contains("(*)") {
					if !self.name.is_empty() {
						typ.to_mut()
							.replacen_in_place("(*)", 1, &format!("(*{name})", name = self.name));
					}
					typ.into_owned()
				} else {
					format!("{typ}{space_name}")
				}
			}
			TypeRefKind::Ignored => {
				format!("<ignored>{space_name}")
			}
		}
		.into()
	}

	fn recurse(&self) -> Self::Recursed {
		Self {
			name_style: self.name_style,
			name: "",
			extern_types: self.extern_types,
		}
	}
}

#[derive(Clone, Debug)]
pub struct CppExternReturnRenderer;

impl<'a> TypeRefRenderer<'a> for CppExternReturnRenderer {
	type Recursed = CppRenderer<'a>;

	fn render<'t>(self, type_ref: &'t TypeRef) -> Cow<'t, str> {
		let kind = type_ref.kind();
		let type_ref = if kind.as_string(type_ref.type_hint()).is_some() {
			Cow::Owned(TypeRef::new_pointer(TypeRefDesc::void()))
		} else if kind.extern_pass_kind().is_by_void_ptr() && !kind.as_abstract_class_ptr().is_some() {
			Cow::Owned(TypeRef::new_pointer(type_ref.clone()))
		} else {
			Cow::Borrowed(type_ref)
		};
		self.recurse().render(&type_ref).into_owned().into()
	}

	fn recurse(&self) -> Self::Recursed {
		CppRenderer::new(CppNameStyle::Reference, "", true)
	}
}

fn render_cpp_tpl<'a>(renderer: impl TypeRefRenderer<'a>, type_ref: &TypeRef) -> String {
	let generic_types = type_ref.template_specialization_args();
	if generic_types.is_empty() {
		return "".to_string();
	}
	let generic_types = generic_types
		.iter()
		.filter_map(|t| match t {
			TemplateArg::Typename(type_ref) => Some(renderer.recurse().render(type_ref)),
			TemplateArg::Constant(literal) => Some(literal.into()),
			TemplateArg::Unknown => None,
		})
		.collect::<Vec<_>>();
	format!("<{}>", generic_types.join(", "))
}
