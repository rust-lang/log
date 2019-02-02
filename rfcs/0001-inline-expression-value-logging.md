# Summary

Add support to `log` for inline expression and value logging, as a
superset of the `dbg!` macro recently added to rust `std`.

# Motivation

The motivation is in part the same as for the accepted `dbg!` macro of
[RFC 2361], as implemented in rust 1.32. To summarize:

* It is convenient to be able to insert logging into larger expressions
  without needing to restructure using additional `let` bindings, or
  by duplicating sub-expressions.

* For debug/trace level logging in particular, automatic formatting of
  the expression with its value can give sufficient context in the log
  (particularly with _target_, _file_ and _line_ information) and avoids
  additional boilerplate.

In a project where configuring `log` and an output logger
implementation hasn't (yet) happened, one can conveniently use
[`std::dbg!`](https://doc.rust-lang.org/std/macro.dbg.html), with no
initial setup cost, for `expression = value` printing to STDERR. From
the linked rustdoc:

> Note that the macro is intended as a debugging tool and therefore
> you should avoid having uses of it in version control for longer
> periods. Use cases involving debug output that should be added to
> version control may be better served by macros such as `debug!` from
> the `log` crate.

Indeed, a major point of the `log` package and `Level`s, is the
ability to keep `debug!` and `trace!` logging in place for further
use, including by other contributors, without paying a cost for
unlogged messages in release builds.

It follows that for projects that _do already have_ `log` and an
output logger dependency and configuration, and particularly with some
debug/trace logging already in place, use of `std::dbg!` would be
unwelcome in PRs, and likely less productive than a `log`-based
equivalent. For parity of convenience, the `log` API needs an
extension for logging expressions and passing through values inline.

# Detailed Design

In addition to the existing set of _formatted logging_ macros, by
level, e.g. `trace!`, `debug!`, etc., we add a new set of _inline
expression and value logging_ macros, with a “_-v_” suffix: `tracev!`,
`debugv!`, and so forth for all levels. The _-v_ macros take a _single_
expression argument, which is evaluated exactly once, regardless of if
the logging level is enabled or not, and returned:

```rust
let n = 12;
let m = debugv!(n / 2) - 1;
//      ^-- debug log message: "n / 2 = 6"
assert_eq!(m, 5);
```

The _default_ format string for the _-v_ macros is `"{} = {:?}"`,
where the `stringify!`-ed expression and resulting value are passed,
in that order.  Note that the `std::dbg!` macro currently uses `"{} =
{:#?}"`—the value is "pretty-printed", potentially over multiple
lines.  Given the line-orientation of logging output, the default
format for the _-v_ macros avoids this. However, a custom format may
also be passed before the expression, which adds considerably more
output flexibility:

```rust
let i = 32;
tracev!("{} = {}", i);            // use `Display` instead of `Debug`
tracev!("{} = {:x}", i);          // hexadecimal format value
tracev!("{} = {:#?}", i);         // use pretty, multi-line format

let rem = infov!("{1:?} remaining ({0})", deadline - Instance::now());
                                  // value first, with additional context
```

For symmetry with the existing `log!` macro, a `logv!` is also
included, which allows passing the `Level` as a parameter.

Finally, like all the other public logging macros, the _-v_ macros
allows overriding the default module-path target with a string
literal:

```rust
let i = 33;
let j = warnv!(target: "maths", (i-1) / 2);
assert_eq!(j, 16);
```

See also the [implementation PR], which includes guide level
documentation, in the form of rustdoc with doc-tests.

# Alternatives

## Multiple expression support

[RFC 2173] included multiple expression printing and return of values
via tuple for `std::dbg!`, but was closed in preference to [RFC 2361] as
merged and implemented.  RFC 2361 on this particular [design
aspect][2361-single]:

> If the macro accepts more than one expression (returning a tuple),
> there is a question of what to do with a single
> expression. Returning a one-value tuple `($expr,)` is probably
> unexpected, but _not_ doing so creates a discontinuity in the macro's
> behavior as things are added. With only one expression accepted,
> users can still pass a tuple expression or call the macro multiple
> times.

In relation to the proposed design of this RFC, accepting multiple
expressions would also be at odds with allowing an optional custom
format string as a preceding parameter.  To support both would require
an additional marker, e.g. `debugv!(format: "{} = {:x}", i, j)`, as
further complication and bulk.

As suggested in RFC 2361, explicitly passing a multiple expression
tuple works when desired, and avoids complications to both the
syntax and macro implementation:

```rust
let j = 19;
let (q, r) = debugv!((j/4, j%4));
\\           ^-- debug log message: (j / 4, j % 4) = (4, 3)
let (q, r) = debugv!("quarter {} = {:?}", (j/4, j%4));
\\           ^-- debug log message: quarter (j / 4, j % 4) = (4, 3)
assert_eq!(q, 4);
assert_eq!(r, 3);
```

## DSL extension of existing log macros

Instead of adding a new set of _-v_ macros, it would be possible to
extend the existing logging macros by using some additional marker
syntax, such as the following:

```rust
debug!(= n/2)
trace!("{} = {:x}", =i);
```

Here the `=` signals that the expression should be `stringify!`-ed for
the message and its value returned from the macro. This complicates
the macro's, but more importantly, considerably complicates the
necessary guide documentation for new and existing users to understand
an evolving logging _DSL_ as new syntax.  This syntax isn't any more
compact. Comprehension is aided when the macro arguments are as
function-like as possible, with macros specific to the feature, as
designed above.

## Only customize the value part of the format

As proposed above, the entire format string may be customized. Until
understanding that the single `i` argument is expanded to two
arguments for formatting, it is surprising to see the following, as
used above, with two placeholders in the format string:

``` rust
tracev!("{} = {:x}", i);
```

A workable alternative would be to only allow customizing the value
part of the format, and with the expression part fixed as `"{} ="`:

``` rust
tracev!("{:x}", i);
```

While this alternative is more compact for the subset of compatible
use cases, it offers less customization options.  For example, it does
not support adding a custom literal prefix to the message, for additional
context. It is also less symmetric with the _formatted logging_
macros, where the format string literal represents the _entire_ text
message of the log `Record`.

## Release this as a separate crate, not in `log`

The proposed additional logging macros play the same role as, for
example, the existing `debug!` macro in `log`, which is just more
convenient than using `log!(Level::Debug, …)` with an extra import.

Ease of use was also an important part of the design and decision to
add `dbg!` to rust `std` and the prelude.

While the proposed additions would still require import for the
macro(s), at least in 2018 edition projects, adding this feature to
`log` avoids:

* Needing to _discover_, add, and maintain an additional library
  dependency. Discovery may be the biggest issue, and the inclusion of
  `dbg!` in `std` and the prelude raises a high bar. This could be
  partially mitigated by documented such an extension crate in the log
  README and/or top-level library rustdoc.

* The community effort to maintain such a separate library with
  compatibility to the `log` crate, as it evolves.  For example, the
  _-v_ macros will likely need to be adapted, when structured logging
  is implemented ([log RFC 296]).

The [implementation PR] (as of this writing) adds just 65 lines
of non-test code, all of which is `macro_rules!`.

# Unresolved Questions

None.

[RFC 2173]: https://github.com/rust-lang/rfcs/pull/2173
[RFC 2361]: https://github.com/rust-lang/rfcs/pull/2361
[log RFC 296]: https://github.com/rust-lang-nursery/log/pull/296
[implementation PR]: https://github.com/rust-lang-nursery/log/pull/316
[2361-single]: https://github.com/rust-lang/rfcs/blob/master/text/2361-dbg-macro.md#accepting-a-single-expression-instead-of-many
