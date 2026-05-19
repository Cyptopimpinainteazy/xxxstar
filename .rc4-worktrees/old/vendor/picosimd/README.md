## picosimd

picosimd is a simple, *non*-portable SIMD crate for Rust. It is meant
to be higher level than raw intrinsics, but not as high level as to
completely abstract away the exact SIMD ISA that's being used.

Features:
  - Small and simple.
  - Zero dependencies.
  - Minimal use of generics.
  - Supports SSE2, AVX2 and AVX512.

Not every intrinsic is supported yet as I'm mostly adding things as I need them,
but if you need some functionality that isn't yet available then PRs are welcome.

### Why yet another SIMD crate? Why not `std::simd`?

`std::simd` is good, but it is *portable*, which is great when you want
to speed things up with minimal effort, but it is *not* what you want when
doing serious performance work with the goal of *maximizing* performance.

Moreover, flinging raw SIMD intrinsics when doing anything complex is a pain,
so having slightly higher level wrappers is a big ergonomic win, and if I'm
going to make wrappers I might as well put them in a separate crate.

## License

Licensed under either of

  * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
  * MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
