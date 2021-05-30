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
equivalent.

## `std::dbg!` doesn't work well in projects with `log` configured

In the context of unit tests, the `cargo test` harness attempts to
capture _stderr_/_stdout_ independently for each test, but the
mechanism is incomplete and fragile: Output from threads other than
the test thread _escapes_ the capture, as does the output of most all
logger implementations, even when on the test thread. Interest in
fixing this has most recently been subsumed by
[_Tracking issue for eRFC 2318, Custom test frameworks_, rust-lang/rust#50297 (comment)][issue 50297]—but
with no clear commitment to a complete implementation and fix.

This results in the practical issue that mixing `log` output with
`println!`, `eprintln!`, or `std::dbg!` results in confusing
buffering, with only the latter being captured. When tests pass, only `log`
output will be shown. When tests fail, `log` and `std::dbg!` output are
both shown, but out of order, with `std::dbg!` captured and buffered
for output after the test panics.

For new users of `log` this can be particularly confusing, since they
are often working through their own project bugs while at the same
time trying to understand this inconsistent `cargo test` behavior.

In such mixed usage, no capture is must less confusing than
partial/broken output capture, so the rather elusive and fun to type
`cargo test -- --nocapture --test-threads=1` can suffice as a
workaround. Anecdotally: The author finally discovered the last flag
of this puzzle just rencently, and previously was incorrectly using the
_documented_ `-j 1` flag.

Even with the hard-earned knowledge of these workarounds, or if test
output capture eventually becomes reliable, when mixing `std::dbg!` with
`log`:

* The output is jarringly in two different formats: one configurable
  by the logging implementation, the other hard-coded by `std::dbg!`
  which is effectively its own micro-logging system.

* `std::dbg!` is hard-coded to use "pretty" multi-line format (via
  `{:#?}`), which is also jarring by normal logging conventions.

* `std::dbg!` isn't beholden to `log`'s level or other filtering and
  thus can't be reasonably kept in a project, at least outside of test
  code.

* Other log metadata or output options like module name (`target`) or
  thread names are not available with `dbg!`

Below is an excerpt of a debugging session combining `log::trace!`,
`log::debug!`, and `std::dbg!`:

```txt
TRACE mio::poll: registering with poller
TRACE tokio_threadpool::builder: build; num-workers=2
TRACE mio::poll: registering with poller
TRACE tokio_threadpool::sender: execute; count=1
TRACE tokio_threadpool::pool:     -> submit external
TRACE tokio_threadpool::pool: signal_work -- notify; idx=1
TRACE tokio_threadpool::pool: signal_work -- spawn; idx=1
[body-image-futio/src/futio_tests/server.rs:73] Tuner::new().set_buffer_size_fs(17).finish() = Tunables {
    max_body_ram: 196608,
    max_body: 1073741824,
    buffer_size_ram: 8192,
    buffer_size_fs: 17,
    size_estimate_deflate: 4,
    size_estimate_gzip: 5,
    size_estimate_brotli: 6,
    temp_dir: "/tmp",
    res_timeout: None,
    body_timeout: Some(
        60s
    )
}
TRACE tokio_threadpool::sender: execute; count=2
TRACE tokio_threadpool::pool:     -> submit external
TRACE tokio_threadpool::pool: signal_work -- notify; idx=0
TRACE tokio_threadpool::pool: signal_work -- spawn; idx=0
TRACE tokio_threadpool::worker: Worker::sleep; worker=WorkerId(1)
TRACE tokio_threadpool::worker:   sleeping -- push to stack; idx=1
TRACE tokio_threadpool::worker:     -> starting to sleep; idx=1
```

With `std::dbg!` released, there is now intrinsic value in at least
offering developers parity and convenience with an extension to `log`
for inline expression and value logging.

# Detailed Design

In addition to the existing set of _formatted logging_ macros, by
level, e.g. `trace!`, `debug!`, etc., we add a new set of _inline
expression and value logging_ macros, with a “_-v_” suffix: `tracev!`,
`debugv!`, and so forth for all levels. The _-v_ macros take a _single_
expression argument, which is evaluated exactly once, regardless of if
the logging level is enabled or not, and returned:

```rust
use std::time::{Duration, Instant};

let remaining = debugv!(deadline - Instant::now());
//               ^-- debug log: deadline - Instant::now() → 950µs
debugv!(remaining);
// or            ^-- debug log: remaining → 950µs
```

The _default_ format string for the _-v_ macros is `"{} → {:?}"`,
where the `stringify!`-ed expression and resulting value are passed,
in that order.  Note that the `std::dbg!` macro currently uses `"{} =
{:#?}"`—the value is "pretty-printed", potentially over multiple
lines.  Given the line-orientation of logging output, the default
format for the _-v_ macros avoids this.  Also we use U+2192 RIGHTWARDS
ARROW (→) as a format separator, which is more easily distinguished
from any commonly typed log message or rust expression. The log
record can be customized via two optional parameters: a message prefix
string, and a format specifier for the value. Note that the former is
required, if passing the later:

```rust
let i = 32;

infov!(i);
infov!("", "{:?}", i);       // equivalent to above
// ^------------------------ info log: i → 32
infov!("index", i);          // contextual prefix specified
infov!("index", "{}", i);    // use `Display` instead of `Debug`
// ^------------------------ info log: index i → 32
infov!("index", "{:#x}", i); // hexadecimal format value
// ^------------------------ info log: index i → 0x20
infov!("index", "{:#?}", i); // use pretty, multi-line format
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

# Expected Usage

Like with `dbg!`, its appropriate to add, then shortly remove, uses of
the _-v_ macros, or to iteratively replace _-v_ macros with the non-v
macro's (low edit distance) for nicer formatting or more English
context. The value of the feature does not hinge on the _-v_ macros
being long lived in code.  The fact that _-v_ macros _could_ be long
lived is just a bonus of inclusion in `log`.  Its also perfectly
appropriate to use the _-v_ macros in statement position (including as
per above design examples).

With the feature in place, while developing and debugging code:

1. Add `tracev!`, `debugv!`, and occasionally, `infov!` calls as
   convenient for debugging and demonstrating correct behavior.

2. When getting closer to release grade changes, refine your logging
   by removing some _-v_ macro calls, and replacing some
   with the existing message formatting macros in statement position,
   making the messages more like English sentences.

3. Check-in (`git commit`) code with `tracev!`, `debugv!` macro
   calls in place. If those calls survived to this step, then they are
   potentially useful in the future to you and other developers, just
   like the current use of `trace` and `debug`.

4. Iteratively repeat with step (1), possibly in parallel with other
   developers.

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
an additional markers, e.g.
`debugv!(prefix: "context", format: "{:x}", i, j)`, for further
complication and bulk.

As suggested in RFC 2361, explicitly passing a multiple expression
tuple works when desired, and avoids complications to both the
syntax and macro implementation:

```rust
let j = 19;
let (q, r) = debugv!((j/4, j%4));
\\           ^-- debug log message: (j / 4, j % 4) → (4, 3)
let (q, r) = debugv!("quarter", (j/4, j%4));
\\           ^-- debug log message: quarter (j / 4, j % 4) → (4, 3)
assert_eq!(q, 4);
assert_eq!(r, 3);
```

## DSL extension of existing log macros

Instead of adding a new set of _-v_ macros, it would be possible to
extend the existing logging macros by using some additional marker
syntax, such as the following:

```rust
debug!(= n/2)
trace!("index", =i);
```

Here the `=` signals that the expression should be `stringify!`-ed for
the message and its value returned from the macro. This complicates
the macro's, but more importantly, considerably complicates the
necessary guide documentation for new and existing users to understand
an evolving logging _DSL_ as new syntax.  This syntax isn't any more
compact. Comprehension is aided when the macro arguments are as
function-like as possible, with macros specific to the feature, as
designed above.

## Allow customizing the entire format

Originally this RFC allowed customizing (and required, for any
customization) the entire format string, in the form:

``` rust
tracev!("contextual prefix: {} = {:x}", i);
```

Since specifying a contextual prefix should be much more common then
changing the value or expression format; in the interest of
convenience, the design was changed to use two separate optional
customization parameters, for the prefix and value format.

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

The [implementation PR] (as of this writing) adds just 86 lines of
non-test code, all of which is `macro_rules!`.

# Unresolved Questions

None.

[RFC 2173]: https://github.com/rust-lang/rfcs/pull/2173
[RFC 2361]: https://github.com/rust-lang/rfcs/pull/2361
[log RFC 296]: https://github.com/rust-lang-nursery/log/pull/296
[implementation PR]: https://github.com/rust-lang-nursery/log/pull/316
[2361-single]: https://github.com/rust-lang/rfcs/blob/master/text/2361-dbg-macro.md#accepting-a-single-expression-instead-of-many
[issue 50297]: https://github.com/rust-lang/rust/issues/50297#issuecomment-388988381
