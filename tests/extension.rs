use std::sync::Arc;

use trait_deref::trait_deref;

#[trait_deref(inherit_my_trait)]
trait MyTrait {
    type Item;

    const A: i32;
    const B: i32 = 2;

    fn get_item(&self) -> Self::Item;

    fn get_name(&self) -> &str;

    #[rc]
    fn get_by_rc<RC>(this: RC, get: impl Fn(&RC) -> &Self) -> Self::Item;

    fn get_arc(self: Arc<Self>) -> Self::Item {
        Self::get_by_rc(self, Arc::as_ref)
    }
}

#[derive(Debug, Clone, Copy)]
struct Base<A>(A);

impl<T: Copy> MyTrait for Base<T> {
    type Item = T;

    const A: i32 = 1;

    fn get_item(&self) -> Self::Item {
        self.0
    }

    fn get_name(&self) -> &str {
        "Base"
    }

    fn get_by_rc<RC>(this: RC, get: impl Fn(&RC) -> &Self) -> Self::Item {
        get(&this).0
    }
}

struct Ext<T: MyTrait<Item = i32>> {
    item: T,
    int: i32,
}

inherit_my_trait! {
    @[item: T]
    impl<T: MyTrait<Item = i32>> MyTrait for Ext<T> {

        const B: i32 = 3;

        fn get_item(&self) -> Self::Item {
            self.int
        }
    }
}

struct Ext2<T: MyTrait> {
    item: T,
    int: i32,
}

inherit_my_trait! {
    @[item: T]
    impl<T: MyTrait> MyTrait for Ext2<T> {
        type Item = i32;

        fn get_item(&self) -> Self::Item {
            self.int
        }

        fn get_by_rc<RC>(this:RC, get:impl Fn(&RC) ->  &Self) -> Self::Item{
            get(&this).int
        }
    }
}

#[test]
fn main() {
    let a = Base(4);
    assert_eq!(a.get_item(), 4);
    assert_eq!(a.get_name(), "Base");

    let b = Ext { item: a, int: 3 };
    assert_eq!(b.get_item(), 3);
    assert_eq!(b.get_name(), "Base");

    let c = Ext { item: b, int: 2 };
    assert_eq!(c.get_item(), 2);

    let d = Ext2 { item: a, int: 5 };
    assert_eq!(d.get_item(), 5);

    let e = Ext2 {
        item: Base("Hello"),
        int: 1,
    };
    assert_eq!(e.get_item(), 1);

    assert_eq!(Base::<i32>::A, 1);
    assert_eq!(Base::<i32>::B, 2);

    assert_eq!(Ext::<Base::<i32>>::A, 1);
    assert_eq!(Ext::<Base::<i32>>::B, 3);
}
