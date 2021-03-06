trait Foo { fn f() -> int; }
trait Bar : Foo { fn g() -> int; }

struct A { x: int }

impl A : Foo { fn f() -> int { 10 } }
impl A : Bar { fn g() -> int { 20 } }

fn ff<T:Foo>(a: &T) -> int {
    a.f()
}

fn gg<T:Bar>(a: &T) -> int {
    a.g()
}

fn main() {
    let a = &A { x: 3 };
    assert ff(a) == 10;
    assert gg(a) == 20;
}

