#![feature(proc_macro_span)]
#![feature(proc_macro_span_shrink)]

use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};
use std::iter::FromIterator;
use syn::Error;

/// A macro that allows for the use of the `?` operator on monad assignments to
/// emulate bind.
#[proc_macro_attribute]
pub fn monadic(attr: TokenStream, item: TokenStream) -> TokenStream {
	if let Some(attr_span) = attr
		.into_iter()
		.map(|tt| tt.span())
		.reduce(|a, e| a.join(e).unwrap())
	{
		return Error::new(attr_span.into(), "monadic takes no attribute arguments")
			.into_compile_error()
			.into();
	}

	let mut out_item = TokenStream::new();
	for tt in item {
		let out_tt = match tt {
			TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
				let mut in_body = g.stream().into_iter();
				TokenTree::Group(Group::new(Delimiter::Brace, monadic_parser(&mut in_body)))
			}
			_ => tt,
		};
		out_item.extend(TokenStream::from(out_tt));
	}
	out_item
}

fn monadic_parser<I: Iterator<Item = TokenTree>>(in_body: &mut I) -> TokenStream {
	let mut out_body = TokenStream::new();
	while let Some(tt) = in_body.next() {
		let out_tt = match tt {
			TokenTree::Ident(i) if i.to_string() == "let" => translate_bind(in_body),
			TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
				return Error::new(g.span().into(), "blocks cannot be used in monadic")
					.into_compile_error()
					.into()
			}
			TokenTree::Punct(p) if p.as_char() == '?' => return misplaced_question_mark(p.span()),
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

	match in_body.next() {
		Some(TokenTree::Punct(p)) => {
			if p.as_char() != '=' {
				let mut out = output_assign(is_mut, ident);
				out.extend(TokenStream::from(TokenTree::Punct(p)));
				return out;
			}
		}
		Some(tt) => {
			let mut out = output_assign(is_mut, ident);
			out.extend(TokenStream::from(tt));
			return out;
		}
		None => return output_assign(is_mut, ident),
	};

	let mut question_span = Span::call_site();
	let mut expr_tts = Vec::new();
	while let Some(tt) = in_body.next() {
		match tt {
			TokenTree::Punct(p) if p.as_char() == '?' => match in_body.next() {
				Some(TokenTree::Punct(p)) if p.as_char() == ';' => {
					question_span = p.span();
					break;
				}
				Some(_) => return misplaced_question_mark(p.span()),
				None => {
					return Error::new(
						p.span().after().into(),
						"unexpected end after question mark",
					)
					.into_compile_error()
					.into()
				}
			},
			TokenTree::Punct(p) if p.as_char() == ';' => {
				expr_tts.push(TokenTree::Punct(p));
				let mut out = output_assign(is_mut, ident);
				out.extend(TokenStream::from(TokenTree::Punct(Punct::new('=', Spacing::Alone))));
				out.extend(expr_tts);
				return out;
			}
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
		monadic_parser(in_body),
	)));

	TokenStream::from_iter([
		TokenTree::Group(Group::new(
			Delimiter::Parenthesis,
			TokenStream::from_iter(expr_tts),
		)),
		TokenTree::Punct(Punct::new('.', Spacing::Alone)),
		TokenTree::Ident(Ident::new("bind", question_span)),
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

fn misplaced_question_mark(span: Span) -> TokenStream {
	Error::new(
		span.into(),
		"`?` cannot be used outside assignments in monadic",
	)
	.into_compile_error()
	.into()
}
