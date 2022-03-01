#![feature(proc_macro_span_shrink)]

use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};
use std::iter::FromIterator;
use syn::Error;

/// A macro that allows for the use of Haskell like bind syntax in Rust.
#[proc_macro]
pub fn do_notation(body: TokenStream) -> TokenStream {
	let mut in_body = body.into_iter();
	do_parser(&mut in_body)
}

fn do_parser<I: Iterator<Item = TokenTree>>(in_body: &mut I) -> TokenStream {
	let mut out_body = TokenStream::new();
	while let Some(tt) = in_body.next() {
		let out_tt = match tt {
			TokenTree::Ident(i) if i.to_string() == "let" => translate_bind(in_body),
			TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
				return Error::new(g.span().into(), "blocks cannot be used in do notation")
					.into_compile_error()
					.into()
			}
			_ => TokenStream::from(tt),
		};
		out_body.extend(out_tt);
	}
	out_body
}

fn translate_bind<I: Iterator<Item = TokenTree>>(in_body: &mut I) -> TokenStream {
	let (is_mut, ident) = match in_body.next() {
		Some(TokenTree::Ident(i)) => {
			if i.to_string() == "mut" {
				(
					true,
					match in_body.next() {
						Some(TokenTree::Ident(i)) => i,
						Some(tt) => {
							let mut out = output_let();
							out.extend(TokenStream::from(TokenTree::Ident(i)));
							out.extend(TokenStream::from(tt));
							return out;
						}
						None => {
							let mut out = output_let();
							out.extend(TokenStream::from(TokenTree::Ident(i)));
							return out;
						}
					},
				)
			} else {
				(false, i)
			}
		}
		Some(tt) => {
			let mut out = output_let();
			out.extend(TokenStream::from(tt));
			return out;
		}
		None => return output_let(),
	};

	let arrow_span = match in_body.next() {
		Some(TokenTree::Punct(p)) if p.as_char() == '<' => match in_body.next() {
			Some(TokenTree::Punct(p)) if p.as_char() == '-' => p.span(),
			Some(tt) => {
				return Error::new(p.span().after().into(), format!("expected `-` found {tt}"))
					.into_compile_error()
					.into()
			}
			None => {
				return Error::new(
					p.span().after().into(),
					"unexpected end of do notation body",
				)
				.into_compile_error()
				.into()
			}
		},
		Some(tt) => {
			let mut out = output_assign(is_mut, ident);
			out.extend(TokenStream::from(tt));
			return out;
		}
		None => return output_assign(is_mut, ident),
	};

	let mut expr_tts = Vec::new();
	while let Some(tt) = in_body.next() {
		match tt {
			TokenTree::Punct(p) if p.as_char() == ';' => break,
			_ => expr_tts.push(tt),
		}
	}

	let mut closure_tts = Vec::new();
	closure_tts.push(TokenTree::Punct(Punct::new('|', Spacing::Alone)));
	if is_mut {
		closure_tts.push(TokenTree::Ident(Ident::new("mut", Span::call_site())));
	}
	closure_tts.push(TokenTree::Ident(ident));
	closure_tts.push(TokenTree::Punct(Punct::new('|', Spacing::Alone)));
	closure_tts.push(TokenTree::Group(Group::new(
		Delimiter::Brace,
		do_parser(in_body),
	)));

	TokenStream::from_iter([
		TokenTree::Group(Group::new(
			Delimiter::Parenthesis,
			TokenStream::from_iter(expr_tts),
		)),
		TokenTree::Punct(Punct::new('.', Spacing::Alone)),
		TokenTree::Ident(Ident::new("bind", arrow_span)),
		TokenTree::Group(Group::new(
			Delimiter::Parenthesis,
			TokenStream::from_iter(closure_tts),
		)),
	])
}

fn output_let() -> TokenStream {
	let mut out = TokenStream::new();
	out.extend(TokenStream::from(TokenTree::Ident(Ident::new(
		"let",
		Span::call_site(),
	))));
	out
}
fn output_assign(is_mut: bool, ident: Ident) -> TokenStream {
	let mut out = output_let();
	if is_mut {
		out.extend(TokenStream::from(TokenTree::Ident(Ident::new(
			"mut",
			Span::call_site(),
		))));
	}
	out.extend(TokenStream::from(TokenTree::Ident(ident)));
	out
}
