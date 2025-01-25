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
