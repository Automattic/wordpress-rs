use wp_derive::immutable_test;

#[immutable_test]
#[immutable_test(requires_wp = 5.6)]
#[mutable_test]
#[mutable_test(requires_wp = 5.6)]

fn foo() {}

fn main() {}
