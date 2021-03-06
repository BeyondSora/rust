
extern mod std;
use std::timer::sleep;
use std::uv;

proto! oneshot (
    waiting:send {
        signal -> !
    }
)

fn main() {
    let (c, p) = oneshot::init();

    assert !pipes::peek(&p);

    oneshot::client::signal(move c);

    assert pipes::peek(&p);
}
