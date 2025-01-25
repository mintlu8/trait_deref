use trait_deref::trait_deref;

#[trait_deref]
trait EmptyTrait {}

struct A<T: EmptyTrait> {
    item: T,
}

impl_empty_trait! {
    @[item: T]
    impl<T: EmptyTrait> EmptyTrait for A<T> {

    }
}
