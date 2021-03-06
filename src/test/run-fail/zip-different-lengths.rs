// In this case, the code should compile but
// the assert should fail at runtime
// error-pattern:Assertion same_length(chars, ints) failed
extern mod std;
use vec::{same_length, zip};

fn enum_chars(start: u8, end: u8) -> ~[char] {
    assert start < end;
    let mut i = start;
    let mut r = ~[];
    while i <= end { r.push(i as char); i += 1 as u8; }
    return r;
}

fn enum_uints(start: uint, end: uint) -> ~[uint] {
    assert start < end;
    let mut i = start;
    let mut r = ~[];
    while i <= end { r.push(i); i += 1; }
    return r;
}

fn main() {
    let a = 'a' as u8, j = 'j' as u8, k = 1, l = 9;
    let chars = enum_chars(a, j);
    let ints = enum_uints(k, l);

    assert same_length(chars, ints);
    let ps = zip(chars, ints);
    fail ~"the impossible happened";
}
