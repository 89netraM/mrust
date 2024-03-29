use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use std::iter::Iterator;
use syn::{
	fold::{fold_expr, Fold},
	parse_macro_input, Error, Expr, ItemFn, Local, ReturnType, Stmt, Type,
};

/// A macro that allows for the use of the `?` operator on monad values to
/// emulate bind.
#[proc_macro_attribute]
pub fn monadic(attr: TokenStream1, item: TokenStream1) -> TokenStream1 {
	let mut function = parse_macro_input!(item as ItemFn);
	let mut in_stmts = function.block.stmts.into_iter();
	let monadic_type = parse_macro_input!(attr as Type);
	let out_stmts = monadic_parse(&mut in_stmts, &monadic_type);

	let attrs = TokenStream::from_iter(function.attrs.iter().map(|a| a.to_token_stream()));
	let vis = function.vis.to_token_stream();
	let ReturnType::Type(_, return_type) = function.sig.output else { panic!() };
	function.sig.output = ReturnType::Default;
	let sig = function.sig.to_token_stream();

	quote! {
		#attrs
		#vis #sig -> #monadic_type<#return_type> {
			#out_stmts
		}
	}
	.into()
}

fn monadic_parse<I: Iterator<Item = Stmt>>(input: &mut I, monadic_type: &Type) -> TokenStream {
	let mut out_stmts = TokenStream::new();
	while let Some(stmt) = input.next() {
		match stmt {
			Stmt::Local(local) => {
				if let Some((eq, boxed_expr)) = local.init {
					match *boxed_expr {
						Expr::Try(try_expr) => {
							let expr = monadic_expr_parser(*try_expr.expr, monadic_type);
							let pat = local.pat;
							let rest = monadic_parse(input, monadic_type);
							if rest.is_empty() {
								expr.to_tokens(&mut out_stmts);
							} else {
								out_stmts.extend(quote! {
									(#expr).bind(|#pat| {
										#rest
									})
								});
							}
						}
						_ => (Local {
							init: Some((
								eq,
								Box::new(Expr::Verbatim(monadic_expr_parser(*boxed_expr, monadic_type))),
							)),
							..local
						})
						.to_tokens(&mut out_stmts),
					}
				} else {
					local.to_tokens(&mut out_stmts);
				}
			}
			Stmt::Semi(expr, s) => match expr {
				Expr::Try(try_expr) => {
					let expr = monadic_expr_parser(*try_expr.expr, monadic_type);
					let rest = monadic_parse(input, monadic_type);
					if rest.is_empty() {
						expr.to_tokens(&mut out_stmts);
					} else {
						out_stmts.extend(quote! {
							(#expr).bind(|_| {
								#rest
							})
						});
					}
				}
				_ => Stmt::Semi(Expr::Verbatim(monadic_expr_parser(expr, monadic_type)), s).to_tokens(&mut out_stmts),
			},
			Stmt::Expr(expr) => {
				Stmt::Expr(Expr::Verbatim(monadic_expr_parser(expr, monadic_type))).to_tokens(&mut out_stmts)
			}
			_ => out_stmts.extend(stmt.to_token_stream()),
		}
	}
	out_stmts
}

fn monadic_expr_parser(expr: Expr, monadic_type: &Type) -> TokenStream {
	// Match with allowed expressions and preform special handling
	match expr {
		Expr::Block(block_expr) => {
			let mut ts = TokenStream::new();
			for a in block_expr.attrs {
				a.to_tokens(&mut ts);
			}
			block_expr.label.to_tokens(&mut ts);
			let block_stmts = monadic_parse(&mut block_expr.block.stmts.into_iter(), monadic_type);
			ts.extend(quote_spanned! {block_expr.block.brace_token.span =>
				{
					#block_stmts
				}
			});
			ts
		}
		Expr::If(if_expr) => {
			let mut ts = TokenStream::new();
			for a in if_expr.attrs {
				a.to_tokens(&mut ts);
			}
			if_expr.if_token.to_tokens(&mut ts);
			UnsupportedReporter::fold_expr(*if_expr.cond).to_tokens(&mut ts);
			let mut block_stmts = monadic_parse(&mut if_expr.then_branch.stmts.into_iter(), monadic_type);
			if if_expr.else_branch.is_none() {
				block_stmts.extend(quote! { .bind(|_| ret(()) ) });
			}
			ts.extend(quote_spanned! {if_expr.then_branch.brace_token.span =>
				{
					#block_stmts
				}
			});
			if let Some((e, expr)) = if_expr.else_branch {
				let expr_ts = monadic_expr_parser(*expr, monadic_type);
				ts.extend(quote! {
					#e #expr_ts
				});
			} else {
				ts.extend(quote! {
					else { ret(()) }
				});
			}
			ts
		}
		Expr::ForLoop(for_expr) => {
			let mut ts = TokenStream::new();
			for a in for_expr.attrs {
				a.to_tokens(&mut ts);
			}
			let expr = UnsupportedReporter::fold_expr(*for_expr.expr);
			let pat = for_expr.pat;
			let body = monadic_parse(&mut for_expr.body.stmts.into_iter(), monadic_type);
			ts.extend(quote! {
				(#expr).fold(#monadic_type::pure(()), |m, #pat| m.bind(|_| { #body }))
			});
			ts
		}
		_ => UnsupportedReporter::fold_expr(expr).into_token_stream(),
	}
}

struct UnsupportedReporter();

impl UnsupportedReporter {
	fn fold_expr(expr: Expr) -> Expr {
		Self {}.fold_expr(expr)
	}
}

impl Fold for UnsupportedReporter {
	fn fold_expr(&mut self, expr: Expr) -> Expr {
		match expr {
			Expr::Try(try_expr) => Expr::Verbatim(
				Error::new_spanned(try_expr.question_token, "monadic bind can not be use at this point")
					.into_compile_error(),
			),
			_ => fold_expr(self, expr),
		}
	}
}
