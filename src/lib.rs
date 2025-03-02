#![no_std]
#![doc = include_str!(concat!(env!("OUT_DIR"),"/", "docs.md"))]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[doc(hidden)]
#[macro_export]
macro_rules! __return_if_err {
	($ex:expr) => {{
		if let err @ ::core::result::Result::Err(_) = $ex {
			return err;
		}
	}};
}

// helper trait to force taking variable by `&mut Self` regardless of how it's passed,
// similar to how `write_fmt` does. more or less copied from
// https://stackoverflow.com/a/79153906
#[doc(hidden)]
pub trait WithMut {
	fn with_mut<'a, R>(&'a mut self, f: impl FnOnce(&'a mut Self) -> R) -> R;
}

impl<T> WithMut for T {
	fn with_mut<'a, R>(&'a mut self, f: impl FnOnce(&'a mut Self) -> R) -> R {
		f(self)
	}
}

/// Common underlying implementation of the various `write`-related macros
#[doc(hidden)]
#[macro_export]
macro_rules! __write_interspersed_impl {
	($writer:expr, $iter:expr, $separator:expr, $arg:pat_param => $($fmt:tt)*) => {{
		use $crate::WithMut;

		let separator = $separator;
		let mut iter = $iter.into_iter();

		$writer.with_mut(move |w| {
			if let ::core::option::Option::Some($arg) = iter.next() {
				// can't use ? because we need to state that the error type is specifically
				// that returned by `write!` and not merely `From` it
				$crate::__return_if_err!(write!(w, $($fmt)*));
				for $arg in iter {
					$crate::__return_if_err!(write!(w, "{separator}"));
					$crate::__return_if_err!(write!(w, $($fmt)*));
				}
			}
			::core::result::Result::Ok(())
		})
	}};
	($writer:expr, $iter:expr, $separator:expr $(,)?) => {
		$crate::__write_interspersed_impl!($writer, $iter, $separator, x => "{x}")
	};
}

/// An interspersing version of [`write!`]
///
/// Writes an iterable’s items, separated by a separator, to a destination. Like
/// `write!`, this macro returns a [`Result`] and requires [`std::io::Write`] or
/// [`std::fmt::Write`] to be in scope, depending on the destination.
///
/// Like all macros in this crate, `write_interspersed!` has two forms:
/// `write_interspersed!(w, iterable, sep)` and `write_interspersed!(w, iterable, sep,
/// pat => fmt_args)`. Both forms require that `sep` implements
/// [`Display`](`std::fmt::Display`). The first also requires that the iterable’s items
/// implement `Display`.
///
/// ```
/// use fmt_interspersed::write_interspersed;
/// use std::{fs, io::{Error, Write}};
///
/// let mut f = fs::File::create("test.txt")?;
/// write_interspersed!(f, 1..=5, ";")?;
/// assert_eq!("1;2;3;4;5", fs::read_to_string("test.txt")?);
///
/// let mut f = fs::File::create("test.txt")?;
/// write_interspersed!(f, [("a", 1), ("b", 2)], ", ", (x, y) => "{x:?} => {y}")?;
/// assert_eq!(r#""a" => 1, "b" => 2"#, fs::read_to_string("test.txt")?);
///
/// # fs::remove_file("test.txt")?;
/// # Ok::<(), Error>(())
/// ```
#[macro_export]
macro_rules! write_interspersed {
	($writer:expr, $($args:tt)*) => {{
		(|| {
			$crate::__return_if_err!($crate::__write_interspersed_impl!($writer, $($args)*));

			::core::result::Result::Ok(())
		})()
	}};

}

/// An interspersing version of [`writeln!`]
///
/// Writes an iterable’s items, separated by a separator, to a destination. Like
/// `writeln!`, this macro returns a [`Result`] and requires [`std::io::Write`] or
/// [`std::fmt::Write`] to be in scope, depending on the destination.
///
/// Like all macros in this crate, `writeln_interspersed!` has two forms:
/// `writeln_interspersed!(w, iterable, sep)` and `writeln_interspersed!(w, iterable,
/// sep, pat => fmt_args)`. Both forms require that `sep` implements
/// [`Display`](`std::fmt::Display`). The first also requires that the iterable’s items
/// implement `Display`.
///
/// ```
/// use fmt_interspersed::writeln_interspersed;
/// use std::{fs, io::{Error, Write}};
///
/// let mut f = fs::File::create("test.txt")?;
/// writeln_interspersed!(f, 1..=5, ";")?;
/// assert_eq!("1;2;3;4;5\n", fs::read_to_string("test.txt")?);
///
/// let mut f = fs::File::create("test.txt")?;
/// writeln_interspersed!(f, [("a", 1), ("b", 2)], ", ", (x, y) => "{x:?} => {y}")?;
/// assert_eq!("\"a\" => 1, \"b\" => 2\n", fs::read_to_string("test.txt")?);
///
/// # fs::remove_file("test.txt")?;
/// # Ok::<(), Error>(())
/// ```
#[macro_export]
macro_rules! writeln_interspersed {
	($writer:expr, $($args:tt)*) => {{
		(|| {
			$crate::__return_if_err!($crate::__write_interspersed_impl!($writer, $($args)*));
			writeln!($writer)?;

			::core::result::Result::Ok(())
		})()
	}};
}

/// Underlying implementation of `__format_interspersed_impl`, simply exists to keep
/// `format_interspersed`’s arguments in sync with other macros.
#[cfg(feature = "alloc")]
#[doc(hidden)]
#[macro_export]
macro_rules! __format_interspersed_impl {
	($iter:expr, $separator:expr $(, $($args:tt)*)?) => {{
		use ::core::fmt::Write;

		let iter = $iter.into_iter();
		let separator = $separator;

		// a reasonable heuristic, assuming the iterator's items and the separator both
		// have length at least 1 when Display'd
		let (lower_bd, _) = iter.size_hint();
		let mut buf = ::alloc::string::String::with_capacity(lower_bd * 2);

		$crate::write_interspersed!(buf, iter, separator $(, $($args)*)?).unwrap();

		buf
	}};
}

/// An interspersing version of [`format!`](std::format)
///
/// Make a string from an iterable’s items separated by a separator.
///
/// Like all macros in this crate, `format_interspersed!` has two forms:
/// `format_interspersed!(w, iterable, sep)` and `format_interspersed!(w, iterable, sep,
/// pat => fmt_args)`. Both forms require that `sep` implements [`Display`](`std::fmt::Display`). The first
/// also requires that the iterable’s items implement `Display`.
///
/// ```
/// # extern crate alloc;
/// use fmt_interspersed::format_interspersed;
///
/// let s = format_interspersed!(1..=5, ";");
/// assert_eq!("1;2;3;4;5", s);
///
/// let s = format_interspersed!([("a", 1), ("b", 2)], ", ", (x, y) => "{x:?} => {y}");
/// assert_eq!(r#""a" => 1, "b" => 2"#, s);
/// ```
#[cfg(feature = "alloc")]
#[macro_export]
macro_rules! format_interspersed {
	 ($($args:tt)*) => {
		  $crate::__format_interspersed_impl!($($args)*)
	 };
}

/// Common implementation for various print-related macros
#[cfg(feature = "std")]
#[macro_export]
#[doc(hidden)]
macro_rules! __print_interspersed_impl {
	(print = $print:path; $iter:expr, $separator:expr, $arg:pat_param => $($fmt:tt)*) => {
		let separator = $separator;
		let mut iter = $iter.into_iter();
		if let ::core::option::Option::Some($arg) = iter.next() {
			$print!($($fmt)*);
			for $arg in iter {
				$print!("{separator}");
				$print!($($fmt)*);
			}
		}
	};
	(print = $print:path; $iter:expr, $separator:expr $(,)?) => {
		$crate::__print_interspersed_impl!(print = $print; $iter, $separator, x => "{x}")
	};
}

/// An interspersing version of [`print!`](std::print)
///
/// Prints the string made from an iterable’s items separated by a separator. Does not
/// allocate.
///
/// Like all macros in this crate, `print!` has two forms: `print!(w, iterable, sep)`
/// and `print!(w, iterable, sep, pat => fmt_args)`. Both forms require that `sep`
/// implements [`Display`](`std::fmt::Display`). The first also requires that the
/// iterable’s items implement `Display`.
///
/// ```
/// use fmt_interspersed::print_interspersed;
///
/// print_interspersed!(1..=5, ";");
/// // 1;2;3;4;5
///
/// print_interspersed!([("a", 1), ("b", 2)], ", ", (x, y) => "{x:?} => {y}");
/// // "a" => 1, "b" => 2
/// ```
#[cfg(feature = "std")]
#[macro_export]
macro_rules! print_interspersed {
	($($args:tt)*) => {{
		$crate::__print_interspersed_impl!(print = ::std::print; $($args)*);
	}};
}

/// An interspersing version of [`println!`](std::println)
///
/// Prints the string made from an iterable’s items separated by a separator, followed
/// by a newline. Does not allocate.
///
/// Like all macros in this crate, `println!` has two forms: `println!(w, iterable,
/// sep)` and `println!(w, iterable, sep, pat => fmt_args)`. Both forms require that
/// `sep` implements [`Display`](`std::fmt::Display`). The first also requires that the
/// iterable’s items implement `Display`.
///
/// ```
/// use fmt_interspersed::println_interspersed;
///
/// println_interspersed!(1..=5, ";");
/// // 1;2;3;4;5
/// // <newline>
///
/// println_interspersed!([("a", 1), ("b", 2)], ", ", (x, y) => "{x:?} => {y}");
/// // "a" => 1, "b" => 2
/// // <newline>
/// ```
#[cfg(feature = "std")]
#[macro_export]
macro_rules! println_interspersed {
	($($args:tt)*) => {{
		$crate::__print_interspersed_impl!(print = ::std::print; $($args)*);
		::std::println!();
	}};
}

/// An interspersing version of [`eprint!`](std::eprint)
///
/// Prints the string made from an iterable’s items separated by a separator to standard
/// error. Does not allocate.
///
/// Like all macros in this crate, `eprint!` has two forms: `eprint!(w, iterable, sep)`
/// and `eprint!(w, iterable, sep, pat => fmt_args)`. Both forms require that `sep`
/// implements [`Display`](`std::fmt::Display`). The first also requires that the
/// iterable’s items implement `Display`.
///
/// ```
/// use fmt_interspersed::eprint_interspersed;
///
/// eprint_interspersed!(1..=5, ";");
/// // (stderr) 1;2;3;4;5
///
/// eprint_interspersed!([("a", 1), ("b", 2)], ", ", (x, y) => "{x:?} => {y}");
/// // (stderr) "a" => 1, "b" => 2
/// ```
#[cfg(feature = "std")]
#[macro_export]
macro_rules! eprint_interspersed {
	($($args:tt)*) => {{
		$crate::__print_interspersed_impl!(print = ::std::eprint; $($args)*);
	}};
}

/// An interspersing version of [`eprintln!`](std::eprintln)
///
/// Prints the string made from an iterable’s items separated by a separator, followed
/// by a newline, to standard error. Does not allocate.
///
/// Like all macros in this crate, `eprintln!` has two forms: `eprintln!(w, iterable,
/// sep)` and `eprintln!(w, iterable, sep, pat => fmt_args)`. Both forms require that
/// `sep` implements [`Display`](`std::fmt::Display`). The first also requires that the
/// iterable’s items implement `Display`.
///
/// ```
/// use fmt_interspersed::eprintln_interspersed;
///
/// eprintln_interspersed!(1..=5, ";");
/// // (stderr) 1;2;3;4;5
/// // (stderr) <newline>
///
/// eprintln_interspersed!([("a", 1), ("b", 2)], ", ", (x, y) => "{x:?} => {y}");
/// // (stderr) "a" => 1, "b" => 2
/// // (stderr) <newline>
/// ```
#[cfg(feature = "std")]
#[macro_export]
macro_rules! eprintln_interspersed {
	($($args:tt)*) => {{
		$crate::__print_interspersed_impl!(print = ::std::eprint; $($args)*);
		::std::eprintln!();
	}};
}

pub mod prelude {
	pub use crate::{write_interspersed, writeln_interspersed};

	#[cfg(feature = "alloc")]
	pub use crate::format_interspersed;

	#[cfg(feature = "std")]
	pub use crate::{
		eprint_interspersed, eprintln_interspersed, print_interspersed, println_interspersed,
	};
}

#[cfg(test)]
mod test;
