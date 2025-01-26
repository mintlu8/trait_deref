# trait_deref

Macro that simulates inheritance in a trait.

## Example

```rust
#[trait_deref]
trait Card {
    type Data;
    const IS_FIXED_COST: bool = false;

    fn get_cost(&self) -> i32;
    fn play(&self, data: &Self::Data);
}

struct CardCostExtension<T: Card> {
    base: T,
    cost: i32
}

impl_card! {
    // dereferences to field self.base for missing items.
    @[base: T]
    impl<T: Card> Card for CardCostExtension<T> {
        // overwrites some items.
        fn get_cost(&self) -> i32 {
            self.cost
        }

        const IS_FIXED_COST: bool = true;

        // the rest are inherited from `base` or `T`
    }
}
```

## Caveats

This crate will always be somewhat janky until `macro 2.0`.
Known issues include:

* Imports

We cannot magically obtain import paths of types, either use fully qualified names like
`::std::sync::Arc`, use a `crate::*` path, which will be transformed to `$crate::*`, or
wrap the generated macro inside another macro that adds the necessary imports to your macro.

* `crate::macro!` cannot be used (lint)

Rust currently does not like this because macros can both access `$crate` and put macros in `$crate`,
which causes ordering issues. Either use the macro from a separate crate downstream, which sidesteps the ordering issue,
or blanket import the crate root:

```rust
use crate::*;
macro!();
```
