use std::fmt::{Debug, Display};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Node(u16);

const A: u16 = 0x01 << 0x00;
const B: u16 = 0x01 << 0x01;
const C: u16 = 0x01 << 0x02;
const D: u16 = 0x01 << 0x03;
const E: u16 = 0x01 << 0x04;
const F: u16 = 0x01 << 0x05;
const G: u16 = 0x01 << 0x06;
const H: u16 = 0x01 << 0x07;
const I: u16 = 0x01 << 0x08;
const J: u16 = 0x01 << 0x09;
const K: u16 = 0x01 << 0x0a;
const L: u16 = 0x01 << 0x0b;
const M: u16 = 0x01 << 0x0c;
const N: u16 = 0x01 << 0x0d;
const O: u16 = 0x01 << 0x0e;
const P: u16 = 0x01 << 0x0f;
const Q: u16 = A | B;
const R: u16 = C | D;
const S: u16 = E | F;
const T: u16 = G | H;
const U: u16 = I | J;
const V: u16 = K | L;
const W: u16 = M | N;
const X: u16 = O | P;
const Y: u16 = Q | R;
const Z: u16 = S | T;
const AA: u16 = U | V;
const AB: u16 = W | X;
const AC: u16 = Y | Z;
const AD: u16 = AA | AB;
const RHO: u16 = AC | AD;

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self.0 {
            A => "A",
            B => "B",
            C => "C",
            D => "D",
            E => "E",
            F => "F",
            G => "G",
            H => "H",
            I => "I",
            J => "J",
            K => "K",
            L => "L",
            M => "M",
            N => "N",
            O => "O",
            P => "P",
            Q => "Q",
            R => "R",
            S => "S",
            T => "T",
            U => "U",
            V => "V",
            W => "W",
            X => "X",
            Y => "Y",
            Z => "Z",
            AA => "AA",
            AB => "AB",
            AC => "AC",
            AD => "AD",
            RHO => "RHO",
            n => {
                let mut result = String::new();
                result.push('[');
                for i in 0..15 {
                    if n & (0x01 << i) != 0 {
                        let ch = char::from_u32(i + u32::from('A')).unwrap();
                        result.push(ch)
                    }
                }
                result.push(']');
                return Display::fmt(&result, f);
            }
        };

        Display::fmt(s, f)
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

#[derive(Default)]
struct Test {
    result: Vec<Node>,
    stack: Vec<Node>,
    counter: u32,
}

impl Test {
    fn pop(&mut self) -> Node {
        let item = self.stack.pop().unwrap();
        item
    }

    pub fn push(&mut self, mut node: Node) {
        let mut counter = self.counter;
        while counter & 1 == 1 {
            let prev = self.pop();
            self.result.push(node);

            node = Node(node.0 | prev.0);
            counter >>= 1;
        }

        self.counter += 1;
        self.result.push(node);
        self.stack.push(node);
    }
}

#[test]
fn xxx() {
    let mut t = Test::default();
    for i in 0..16 {
        let node = Node(1 << i);
        t.push(node);
    }

    for i in 0..16u32 {
        let node = Node(1 << i);
        // don't even ask me why...
        let index = i * 2 - i.count_ones();
        assert_eq!(t.result[index as usize], node);
    }

    println!("{:?}", t.stack);
    println!("{:?}", t.result);
}

#[test]
fn test() {
    let mut stack = Vec::<Node>::new();

    const START: u32 = 4;
    assert!(0x01 << START == E);

    let mut counter = 1usize;

    for o in START..16u32 {
        let i = o - START;
        let node = Node(0x01 << o);
        println!("----i={i}/{node}");
        stack.push(node);

        if i > 1 {
            if i & 1 == 0 {
                counter += i.trailing_zeros() as usize;
            } else {
                let k = unset_left_most(i);
                counter -= k.trailing_ones() as usize;
            }
        }

        let post_merge_stack_len = i.count_ones() as usize + counter;

        while stack.len() > post_merge_stack_len && stack.len() >= 2 {
            let a = stack.pop().unwrap();
            let b = stack.pop().unwrap();
            stack.push(Node(a.0 | b.0));
        }

        println!("{:?}", stack);
    }
}

#[inline(always)]
fn unset_left_most(n: u32) -> u32 {
    if n == 0 {
        0
    } else {
        !(1 << (u32::BITS - n.leading_zeros() - 1)) & n
    }
}

#[test]
fn x() {
    let mut stack = Vec::<Node>::new();
    const N: u32 = 6;

    // change the trailing zeros to one and use that number as the upper bound.
    let z = N.trailing_zeros();
    let mut tmp = 1;
    let mut upper = N;
    for i in 0..z {
        upper |= tmp;
        tmp <<= 1;
    }
    let upper = upper.min(15);

    for i in 0..=15 {
        let mut node = Node(0x01 << i);

        // println!("\n----");
        // println!("{node}={:#b}", i);

        let mut total_chunks = i + 1;

        let k = i ^ N;
        let mut s = if k == 0 {
            0
        } else {
            u32::BITS - k.leading_zeros()
        };

        while total_chunks & 1 == 0 && s > 1 {
            node = Node(node.0 | stack.pop().unwrap().0);
            total_chunks >>= 1;
            s -= 1;
        }

        stack.push(node);
    }

    println!("{:?}", stack);
}

#[test]
fn fuck() {
    println!("{:#b}", 1 ^ 3);
    println!("{:#b}", 0 ^ 3);
    const N: u32 = 1u32;

    for i in 0..4u32 {
        let mut k = i ^ N;
        println!("{N:#b} ^ {i:#b} = {k:#b}");

        let mut s = if k == 0 {
            0
        } else {
            u32::BITS - k.leading_zeros()
        };

        println!("{s}");
    }
}
