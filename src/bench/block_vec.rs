use std::time::Instant;
use crate::block_vec;

#[test]
pub fn push_reallocation_test_b_vec(){
    let size = 100000000;
    let mut bvec = block_vec::BVec::<i32>::new();
    let start = Instant::now();
    for i in 0..size{
        bvec.push(i);
    }
    let duration = start.elapsed();
    println!("Time taken BVec: {:?}", duration);
}

#[test]
pub fn push_reallocation_test_o_vec(){
    let size = 100000000;
    let mut vec = Vec::<i32>::new();
    let start = Instant::now();
    for i in 0..size{
        vec.push(i);
    }
    let duration = start.elapsed();
    println!("Time taken Vec: {:?}", duration);
}