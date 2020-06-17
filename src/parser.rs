extern crate alloc;
use alloc::vec::Vec;
use crate::impls::{SimpleError, SimplePosition};
use core::marker::PhantomData;

pub trait Position: core::ops::Sub<Self, Output = i32> + Copy {
	fn index(&self) -> u32;
	fn line(&self) -> u32;
	fn column(&self) -> u32;
}

pub trait Error {
	type Position;

	fn reasons(&self) -> &[(Option<Self::Position>, &'static str)];
	fn add_reason(self, position: Option<Self::Position>, reason: &'static str) -> Self;
	fn plain_str(reason: &'static str) -> Self;
}

pub trait Input: Default {
	type Position: Position;
	type Error: Error<Position = Self::Position>;
	fn next(&self, pos: Self::Position) -> Result<(char, Self::Position), Self::Error>;
	fn next_range(
		&self,
		start: Self::Position,
		counts: u32,
	) -> Result<(&str, Self::Position), Self::Error>;
	fn error_at(&self, pos: Self::Position, reason: &'static str) -> Self::Error;
}

pub type ResultOf<I, O> = Result<(O, <I as Input>::Position), <I as Input>::Error>;

pub trait Parser<I: Input> {
	type Output;
	fn parse(input: &I, current: I::Position) -> ResultOf<I, Self::Output>;
}

pub trait Predicate<T> {
	fn eval(t: &T) -> bool;
}

pub struct ExpectChar<P>(PhantomData<P>);

impl<P: Predicate<char>, I: Input> Parser<I> for ExpectChar<P> {
	type Output = char;
	fn parse(input: &I, current: I::Position) -> ResultOf<I, Self::Output> {
		let (c, next) = input
			.next(current)
			.map_err(|e| e.add_reason(Some(current), "ExpectChar"))?;
		if P::eval(&c) {
			Ok((c, next))
		} else {
			Err(input.error_at(current, "ExpectChar"))
		}
	}
}

pub struct Null;

impl<I: Input> Parser<I> for Null {
	type Output = ();
	fn parse(_input: &I, current: I::Position) -> ResultOf<I, Self::Output> {
		Ok(((), current))
	}
}

pub struct Concat<P, P2>(PhantomData<(P, P2)>);

impl<I: Input, P: Parser<I>, P2: Parser<I>> Parser<I> for Concat<P, P2> {
	type Output = (P::Output, P2::Output);
	fn parse(input: &I, current: I::Position) -> ResultOf<I, Self::Output> {
		let (output1, pos) =
			P::parse(input, current).map_err(|e| e.add_reason(Some(current), "Concat1"))?;
		let (output2, pos) = P2::parse(input, pos).map_err(|e| e.add_reason(Some(current), "Concat2"))?;
		Ok(((output1, output2), pos))
	}
}

pub type Concat3<P, P2, P3> = Concat<P, Concat<P2, P3>>;
pub type Concat4<P, P2, P3, P4> = Concat<P, Concat<P2, Concat<P3, P4>>>;
pub type Concat5<P, P2, P3, P4, P5> = Concat<P, Concat<P2, Concat<P3, Concat<P4, P5>>>>;

#[cfg_attr(feature = "std", derive(Debug))]
pub enum Either<A, B> {
	A(A),
	B(B),
}

pub struct OneOf<P, P2>(PhantomData<(P, P2)>);

impl<I: Input, P: Parser<I>, P2: Parser<I>> Parser<I> for OneOf<P, P2> {
	type Output = Either<P::Output, P2::Output>;
	fn parse(input: &I, current: I::Position) -> ResultOf<I, Self::Output> {
		P::parse(input, current)
			.map(|(output, pos)| (Either::A(output), pos))
			.or_else(|_| P2::parse(input, current).map(|(output, pos)| (Either::B(output), pos)))
			.map_err(|e| e.add_reason(Some(current), "OneOf"))
	}
}

pub type OneOf3<P, P2, P3> = OneOf<P, OneOf<P2, P3>>;
pub type OneOf4<P, P2, P3, P4> = OneOf<P, OneOf3<P2, P3, P4>>;
pub type OneOf5<P, P2, P3, P4, P5> = OneOf<P, OneOf4<P2, P3, P4, P5>>;
pub type OneOf6<P, P2, P3, P4, P5, P6> = OneOf<P, OneOf5<P2, P3, P4, P5, P6>>;
pub type OneOf7<P, P2, P3, P4, P5, P6, P7> = OneOf<P, OneOf6<P2, P3, P4, P5, P6, P7>>;
pub type OneOf8<P, P2, P3, P4, P5, P6, P7, P8> = OneOf<P, OneOf7<P2, P3, P4, P5, P6, P7, P8>>;
pub type OneOf9<P, P2, P3, P4, P5, P6, P7, P8, P9> =
	OneOf<P, OneOf8<P2, P3, P4, P5, P6, P7, P8, P9>>;

pub type ZeroOrOne<P> = OneOf<P, Null>;

pub type ZeroOrMore<P> = OneOf<OneOrMore<P>, Null>;

//pub type OneOrMore<P> = Concat<P, ZeroOrMore<P>>;
pub struct OneOrMore<P>(PhantomData<P>);

impl<I: Input, P: Parser<I>> Parser<I> for OneOrMore<P> {
	type Output = Vec<P::Output>;
	fn parse(input: &I, current: I::Position) -> ResultOf<I, Self::Output> {
		let mut output_list = Vec::new();
		let (output, mut pos) =
			P::parse(input, current).map_err(|e| e.add_reason(Some(current), "OneOrMore"))?;
		output_list.push(output);
		loop {
			if let Ok((output, next_pos)) = P::parse(input, pos) {
				pos = next_pos;
				output_list.push(output);
			} else {
				return Ok((output_list, pos));
			}
		}
	}
}

impl Input for &str {
	type Position = SimplePosition;
	type Error = SimpleError;

	fn next(&self, pos: Self::Position) -> Result<(char, Self::Position), Self::Error> {
		self.chars()
			.nth(pos.index() as usize)
			.ok_or_else(|| self.error_at(pos, "Out of bounds"))
			.map(|c| (c, pos.next(c)))
	}

	fn next_range(
		&self,
		start: Self::Position,
		counts: u32,
	) -> Result<(&str, Self::Position), Self::Error> {
		let start_index = start.index() as usize;
		let range = start_index..start_index + counts as usize;
		self.get(range)
			.map(|s| {
				let mut pos = start;
				for c in s.chars() {
					pos = pos.next(c);
				}
				(s, pos)
			})
			.ok_or_else(|| self.error_at(start, "Out of bounds"))
	}

	fn error_at(&self, pos: Self::Position, reason: &'static str) -> Self::Error {
		let mut reasons = Vec::new();
		reasons.push((Some(pos), reason));
		SimpleError { reasons }
	}
}

#[macro_export]
macro_rules! literals {
	(
		$(
			$( #[ $attr:meta ] )*
			$vis:vis $name:ident => $($($value:literal)..=+)|+;
		)*
	) => {
		$(
			$crate::literals!{
				IMPL
				$( #[ $attr ] )*
				$vis $name => $($($value)..=+)|+
			}
		)*
	};
	(
		IMPL
		$( #[ $attr:meta ] )*
		$vis:vis $name:ident => $($($value:literal)..=+)|+
	) => (
		paste::item! {
			$vis struct [< $name Predicate >];
			impl $crate::parser::Predicate<char> for [< $name Predicate >] {
				fn eval(c: &char) -> bool {
					match *c {
						$($($value)..=+)|+ => true,
						_ => false
					}
				}
			}

			$( #[ $attr ] )*
			$vis type $name = $crate::parser::ExpectChar<[< $name Predicate >]>;
		}
	);
}

#[macro_export]
macro_rules! parsers {
	(
		$(
			$( #[ $attr:meta ] )*
			$vis:vis $name:ident = $type:ty, $output_type:ty, ($output:ident) => $body:block;
		)*
	) => {
		$(
			$vis struct $name;
			impl<I: $crate::parser::Input> $crate::parser::Parser<I> for $name {
				type Output = $output_type;
				fn parse(input: &I, current: I::Position) -> $crate::parser::ResultOf<I, Self::Output> {
					let ($output, pos) = <$type as $crate::parser::Parser<I>>::parse(input, current)
						.map_err(|e| <I::Error as $crate::parser::Error>::add_reason(e, Some(current), stringify!($name)))?;
					let res = $body;
					Ok((res, pos))
				}
			}
		)*
	};
}
