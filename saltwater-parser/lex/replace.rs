//! Macro replacement
//!
//! This module does no parsing and accepts only tokens.

use super::{cpp::CppResult, files::FileProcessor};
use crate::{
    error::CppError, CompileError, CompileResult, InternedStr, LiteralToken, Locatable, Location,
    Token,
};
use std::collections::{HashMap, HashSet, VecDeque};

use arcstr::Substr;

/// All known macro definitions.
///
/// Note that this is a simple HashMap and not a `Scope`, because
/// the preprocessor has no concept of scope other than `undef`.
pub type Definitions = HashMap<InternedStr, Definition>;

/// An iterator which allows you to `peek()` at the next token.
///
/// This is required by `replace` for implementation reasons (function macros).
/// The intended usage for most API users is to call `iter.peekable()` on an existing iterator.
/// Alternatively, you can pass in an empty iterator with `std::iter::empty()`.
pub trait Peekable: Iterator {
    fn peek(&mut self) -> Option<&Self::Item>;
}

impl Peekable for FileProcessor {
    fn peek(&mut self) -> Option<&Self::Item> {
        self.peek()
    }
}

impl<T: Iterator> Peekable for std::iter::Peekable<T> {
    fn peek(&mut self) -> Option<&Self::Item> {
        self.peek()
    }
}

impl<T> Peekable for std::iter::Empty<T> {
    fn peek(&mut self) -> Option<&Self::Item> {
        None
    }
}

impl<I: Peekable + ?Sized> Peekable for &mut I {
    fn peek(&mut self) -> Option<&Self::Item> {
        (**self).peek()
    }
}

impl<I: Peekable + ?Sized> Peekable for Box<I> {
    fn peek(&mut self) -> Option<&Self::Item> {
        (**self).peek()
    }
}

/// A macro definition.
#[derive(Clone, Debug, PartialEq)]
pub enum Definition {
    /// An object macro: `#define a b + 1`
    Object(Vec<Token>),
    /// A function macro: `#define f(a) a + 1`
    Function {
        /// The function parameters.
        ///
        /// In the example above, `a` is a function parameter.
        /// A macro may have 0 or more parameters.
        /// Variadic macros (`__VA_ARGS__`) are not yet implemented.
        ///
        /// Note that function macros may be called with an empty replacement list for any parameter.
        /// For example, `f()` is valid and exapands to `+ 1`.
        /// Similarly, for `#define g(a, b) a + b`, `g(,)` is valid and expands to `+`.
        params: Vec<InternedStr>,
        /// The body for a function macro.
        ///
        /// The function body itself undergoes recursive macro replacement.
        body: Vec<Token>,
    },
}

pub struct Replace<'a, I: Iterator> {
    iter: std::iter::Peekable<I>,
    definitions: &'a Definitions,
}

pub fn replace_iter<I: Iterator>(iter: I, definitions: &Definitions) -> Replace<'_, I> {
    Replace {
        iter: iter.peekable(),
        definitions,
    }
}

impl<I: Iterator<Item = CompileResult<Locatable<Token>>>> Iterator for Replace<'_, I> {
    type Item = Vec<CompileResult<Locatable<Token>>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(Ok(t)) => Some(replace(
                self.definitions,
                t.data,
                &mut self.iter,
                t.location,
            )),
            Some(Err(err)) => Some(vec![Err(err)]),
            None => None,
        }
    }
}

/// Perform recursive macro replacement on `token`.
///
/// This first performs object-macro replacement, then function-macro replacement.
/// For example, consider this C program:
/// ```c
/// #define f(a, b) a + b
/// #define g f
/// #define c d
/// g(c, 1)
/// ```
/// First, the name of the function will be replaced: `f(c, 1)`.
/// Next, all arguments are replaced: `f(d, 1)`.
/// Finally, function-macros are replaced: `d + 1`.
///
/// If at any point there is a cyclic replacement, no error is generated.
/// Instead, replacement immediately stops for the current token.
/// However, future tokens may still be replaced. For example, take
/// ```c
/// #define b c
/// #define f(a) g(a + 1)
/// #define g(a) f(a)
/// f(b)
/// ```
/// The cycle f -> g will be detected after the first time through: `f(b)` -> `g(b + 1)` -> `f(b + 1)`.
/// Then, replacements of later tokens occur: `f(c + 1)`.
///
/// Note that known cycles do not persist between calls to `replace`.
/// This means that very long cycles will be recalculated on each call.
///
/// WARNING: if you call `replace()` the wrong way, you can cause an infinite loop.
/// Take for example this code (from [#298](https://github.com/jyn514/rcc/issues/298)):
/// ```c
/// #define sa_handler   __sa_handler.sa_handler
/// sa_handler
/// ```
/// If you implement a naive iterator on top of `MacroReplacer`,
/// you'll have a list of pending tokens (since a single underlying token can expand to many tokens).
/// If you then feed those tokens back to `replace` in a loop, they will generate infinitely many tokens:
/// `replace(sa_handler) -> yield __sa_handler; yield .; replace(sa_handler) -> ...`
/// The solution is to keep track of which tokens have already been replaced and not replace them a second time.
///
/// In most cases the above will not be relevant since you will only call `replace()` once,
/// or only call it on tokens which have not yet been replaced.
///
/// `inner` is needed for implementation reasons.
/// In most cases, you can pass in `std::iter::empty()` instead.
/// You can also use it if you have an underlying stream of tokens
/// that you want to use in addition to the tokens generated by replacing `token`.
///
/// `location` is used only for errors; in all other cases it is ignored.
#[must_use = "does not change internal state"]
pub fn replace<I>(
    definitions: &Definitions,
    token: Token,
    inner: I,
    location: Location,
) -> Vec<CompileResult<Locatable<Token>>>
where
    I: Iterator<Item = CppResult<Token>> + Peekable,
{
    replace_boxed(definitions, token, Box::new(inner), location)
}

fn replace_boxed<'a>(
    definitions: &Definitions,
    token: Token,
    mut inner: Box<dyn Peekable<Item = CppResult<Token>> + 'a>,
    location: Location,
) -> Vec<CompileResult<Locatable<Token>>> {
    // The ids seen while replacing the current token.
    //
    // This allows cycle detection. It should be reset after every replacement list
    // - _not_ after every token, since otherwise that won't catch some mutual recursion
    // See https://github.com/jyn514/rcc/issues/427 for examples.
    let mut ids_seen = HashSet::new();
    let mut replacements = Vec::new();
    let mut pending = VecDeque::new();
    pending.push_back(Ok(location.with(token)));

    // outer loop: replace all tokens in the replacement list
    while let Some(token) = pending.pop_front() {
        // first step: perform (recursive) substitution on the ID
        if let Ok(Locatable {
            data: Token::Id(id),
            ..
        }) = token
        {
            if !ids_seen.contains(&id) {
                match definitions.get(&id) {
                    Some(Definition::Object(replacement_list)) => {
                        ids_seen.insert(id);
                        // prepend the new tokens to the pending tokens
                        // They need to go before, not after. For instance:
                        // ```c
                        // #define a b c d
                        // #define b 1 + 2
                        // a
                        // ```
                        // should replace to `1 + 2 c d`, not `c d 1 + 2`
                        let mut new_pending = VecDeque::new();
                        // we need a `clone()` because `self.definitions` needs to keep its copy of the definition
                        new_pending.extend(
                            replacement_list
                                .iter()
                                .cloned()
                                .map(|t| Ok(location.with(t))),
                        );
                        new_pending.append(&mut pending);
                        pending = new_pending;
                        continue;
                    }
                    // TODO: so many allocations :(
                    Some(Definition::Function { .. }) => {
                        ids_seen.insert(id);
                        let func_replacements =
                            replace_function(definitions, id, location, &mut pending, &mut inner);
                        let mut func_replacements: VecDeque<_> =
                            func_replacements.into_iter().collect();
                        func_replacements.append(&mut pending);
                        pending = func_replacements;
                        continue;
                    }
                    None => {}
                }
            }
        }
        replacements.push(token);
    }

    replacements
}

// TODO: this should probably return Result<VecDeque, CompileError> instead
#[must_use = "does not change internal state"]
fn replace_function(
    definitions: &Definitions,
    id: InternedStr,
    location: Location,
    incoming: &mut VecDeque<CompileResult<Locatable<Token>>>,
    mut inner: impl Iterator<Item = CppResult<Token>> + Peekable,
) -> Vec<Result<Locatable<Token>, CompileError>> {
    use std::mem;

    let mut errors = Vec::new();

    loop {
        match incoming.front().or_else(|| inner.peek()) {
            // handle `f @ ( 1 )`, with arbitrarly many token errors
            Some(Err(_)) => {
                let next = incoming.pop_front().or_else(|| inner.next());
                // TODO: need to figure out what should happen if an error token happens during replacement
                errors.push(Err(next.unwrap().unwrap_err()));
            }
            // f (
            Some(Ok(Locatable {
                data: Token::LeftParen,
                ..
            })) => {
                // pop off the `(` so it isn't counted as part of the first argument
                if incoming.pop_front().is_none() {
                    inner.next();
                }
                break;
            }
            // skip any whitespaces and newlines between `f` and `(`, so that
            // `f (` is also valid.
            Some(Ok(Locatable {
                data: Token::Whitespace(_),
                ..
            })) => {
                let spaces = incoming.pop_front().or_else(|| inner.next()).unwrap();
                let left_paren = incoming.front().or_else(|| inner.peek());
                if let Some(Ok(Locatable {
                    data: Token::LeftParen,
                    ..
                })) = left_paren
                {
                    if incoming.pop_front().is_none() {
                        inner.next();
                    }
                    break;
                }
                let id_token = Ok(location.with(Token::Id(id)));
                errors.push(id_token);
                errors.push(spaces);
                return errors;
            }
            // If this branch is matched, this is not a macro call,
            // since all other cases are covered above.
            // So just append the identifier and the current token to the stack.
            Some(Ok(_)) => {
                let token = incoming.pop_front().or_else(|| inner.next()).unwrap();
                let id_token = Ok(location.with(Token::Id(id)));
                errors.push(id_token);
                errors.push(token);
                return errors;
            }
            // `f<EOF>`
            None => {
                let id_token = Ok(location.with(Token::Id(id)));
                errors.push(id_token);
                return errors;
            }
        }
    }

    // now, expand all arguments
    let mut args = Vec::new();
    let mut current_arg = Vec::new();
    let mut nested_parens = 1;

    fn strip_whitespace(mut args: Vec<Token>) -> Vec<Token> {
        if matches!(args.last(), Some(Token::Whitespace(_))) {
            args.pop();
        }
        assert!(!matches!(args.last(), Some(Token::Whitespace(_))));
        if matches!(args.first(), Some(Token::Whitespace(_))) {
            args.remove(0);
        }
        assert!(!matches!(args.first(), Some(Token::Whitespace(_))));
        args
    }

    loop {
        let next = match incoming.pop_front().or_else(|| inner.next()) {
            // f ( <EOF>
            // TODO: this should give an error
            None => return errors,
            // f ( @
            Some(Err(err)) => {
                errors.push(Err(err));
                continue;
            }
            // f ( +
            Some(Ok(token)) => token,
        };
        match next.data {
            // f ( a,
            // NOTE: `f(,)` is _legal_ and means to replace f with two arguments, each an empty token lists
            // on the bright side, we don't have to check if `current_arg` is empty or not
            Token::Comma if nested_parens == 1 => {
                args.push(strip_whitespace(mem::take(&mut current_arg)));
                continue;
            }
            // f ( (a + 1)
            Token::RightParen => {
                nested_parens -= 1;
                // f ( )
                if nested_parens == 0 {
                    args.push(strip_whitespace(mem::take(&mut current_arg)));
                    break;
                }
            }
            // f ( (
            Token::LeftParen => {
                nested_parens += 1;
            }
            // f( + )
            _ => {}
        }
        // TODO: keep the location
        current_arg.push(next.data);
    }

    let (params, body) = match definitions.get(&id) {
        Some(Definition::Function { params, body }) => (params, body),
        // TODO: it would be nice to pass in `params` and `body` directly, but that runs into borrow errors
        _ => unreachable!("checked above"),
    };

    let macro_name = id.resolve_and_clone();
    let macro_body = body
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    let hash_error = || CppError::HashMissingParameterContext {
        macro_name: macro_name.clone(),
        body: macro_body.clone(),
    };

    let mut replacements = Vec::new();
    if args.len() != params.len() {
        // There is no way to distinguish between a macro-function taking one empty argument
        // and taking no arguments other than knowing the number of parameters.
        if !(args.len() == 1 && params.is_empty() && args[0].is_empty()) {
            // booo, this is the _only_ error in the whole replacer
            return vec![Err(
                location.with(CppError::TooFewArguments(params.len(), args.len()).into())
            )];
        }
    }

    let mut i = 0;
    while i < body.len() {
        if matches!(body[i], Token::Hash) {
            let mut second = i + 1;
            while second < body.len() && matches!(body[second], Token::Whitespace(_)) {
                second += 1;
            }
            if second < body.len() && matches!(body[second], Token::Hash) {
                while matches!(replacements.last(), Some(Token::Whitespace(_))) {
                    replacements.pop();
                }
                let mut right = second + 1;
                while right < body.len() && matches!(body[right], Token::Whitespace(_)) {
                    right += 1;
                }
                if right >= body.len() {
                    return vec![Err(location.with(hash_error().into()))];
                }
                let right_tokens = match body[right].clone() {
                    Token::Id(id) => {
                        if let Some(index) = params.iter().position(|&param| param == id) {
                            args[index].clone()
                        } else {
                            vec![Token::Id(id)]
                        }
                    }
                    token => vec![token],
                };
                let first = match right_tokens.first() {
                    Some(token) => token.clone(),
                    None => {
                        i = right + 1;
                        continue;
                    }
                };
                let previous = match replacements.pop() {
                    Some(token) => token,
                    None => return vec![Err(location.with(hash_error().into()))],
                };
                match paste_identifier_tokens(previous, first) {
                    Some(pasted) => replacements.push(pasted),
                    None => return vec![Err(location.with(hash_error().into()))],
                }
                replacements.extend(right_tokens.into_iter().skip(1));

                // A paste result can itself be the left operand of another ##,
                // as in EXT##ver##_FEATURE_COMPAT_##flagname. Leave the cursor
                // on the just-consumed right operand so the following iteration
                // can detect and continue that chain.
                let mut next_hash = right + 1;
                while next_hash < body.len() && matches!(body[next_hash], Token::Whitespace(_)) {
                    next_hash += 1;
                }
                if next_hash < body.len() && matches!(body[next_hash], Token::Hash) {
                    i = next_hash;
                } else {
                    i = right + 1;
                }
                continue;
            }

            let mut parameter = i + 1;
            while parameter < body.len() && matches!(body[parameter], Token::Whitespace(_)) {
                parameter += 1;
            }
            match body.get(parameter) {
                Some(Token::Id(id)) => {
                    if let Some(index) = params.iter().position(|&param| param == *id) {
                        replacements.push(stringify(args[index].clone()));
                        i = parameter + 1;
                        continue;
                    }
                }
                _ => {}
            }
            return vec![Err(location.with(hash_error().into()))];
        }

        match body[i].clone() {
            Token::Id(id) => {
                let left_tokens = if let Some(index) = params.iter().position(|&param| param == id) {
                    args[index].clone()
                } else {
                    vec![Token::Id(id)]
                };

                let mut hash = i + 1;
                while hash < body.len() && matches!(body[hash], Token::Whitespace(_)) {
                    hash += 1;
                }
                let mut second_hash = hash + 1;
                while second_hash < body.len()
                    && matches!(body[second_hash], Token::Whitespace(_))
                {
                    second_hash += 1;
                }

                if hash < body.len()
                    && matches!(body[hash], Token::Hash)
                    && second_hash < body.len()
                    && matches!(body[second_hash], Token::Hash)
                {
                    replacements.extend(left_tokens);
                    while matches!(replacements.last(), Some(Token::Whitespace(_))) {
                        replacements.pop();
                    }
                    i = hash;
                    continue;
                }

                replacements.extend(left_tokens);
            }
            Token::Whitespace(_) => replacements.push(Token::Whitespace(String::from(" "))),
            token => replacements.push(token),
        }
        i += 1;
    }
    // TODO: this collect is useless
    // The replacement list produced by ## must be rescanned for macro names.
    // Feed the generated preprocessing tokens through the normal replacer once
    // more, while preserving the existing cycle protection at the outer level.
    let mut rescan_input = replacements
        .into_iter()
        .map(|token| Ok(location.with(token)))
        .peekable();
    let mut rescanned = Vec::new();
    while let Some(item) = rescan_input.next() {
        match item {
            Ok(token) => rescanned.extend(replace(
                definitions,
                token.data,
                &mut rescan_input,
                token.location,
            )),
            Err(error) => rescanned.push(Err(error)),
        }
    }

    errors.into_iter().chain(rescanned).collect()
}

fn paste_identifier_tokens(left: Token, right: Token) -> Option<Token> {
    fn spelling(token: Token) -> Option<String> {
        match token {
            Token::Id(id) => Some(id.resolve_and_clone()),
            Token::Keyword(keyword) => Some(keyword.to_string()),
            Token::Literal(literal) => Some(literal.to_string()),
            _ => None,
        }
    }

    let pasted = format!("{}{}", spelling(left)?, spelling(right)?);

    // The result of ## is a preprocessing token, not necessarily an identifier.
    // Re-lex it so numeric pastes such as 1 ## 1 become the integer token 11
    // instead of an identifier whose spelling happens to be "11".
    let mut files = crate::Files::new();
    let file = files.add("<token-paste>", crate::Source {
        code: arcstr::ArcStr::from(pasted.as_str()),
        path: std::path::PathBuf::from("<token-paste>"),
    });
    let mut lexer = crate::lex::Lexer::new(file, pasted.as_str(), false);
    let first = lexer.next()?.ok()?.data;
    if lexer.next().is_some() {
        return None;
    }
    Some(first)
}

fn stringify(args: Vec<Token>) -> Token {
    let escape = |s: &str| s.replace('\\', r#"\\"#).replace('"', r#"\""#);
    let ret: String = args
        .into_iter()
        .map(|arg| {
            match arg {
                Token::Whitespace(_) => String::from(" "), // Single space in replacement
                Token::Literal(LiteralToken::Str(rcstrs)) => rcstrs
                    .iter()
                    .map(|rcstr| escape(rcstr.as_str()))
                    .collect::<Vec<_>>()
                    .join(" "),
                Token::Literal(LiteralToken::Char(rcstr)) => escape(rcstr.as_str()),
                other => other.to_string(),
            }
        })
        .collect();
    Token::Literal(LiteralToken::Str(vec![Substr::from(arcstr::format!(
        "\"{}\"",
        ret.trim()
    ))]))
}
