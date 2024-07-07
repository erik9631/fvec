use std::time::Instant;
use tracy::{frame};
use crate::avec::AVec;
use crate::fvec;

#[test]
pub fn push_reallocation_bench_b_vec(){
    let size = 10000;
    let mut bvec = fvec::FVec::<i32>::new(512);
    let start = Instant::now();
    for i in 0..size{
        bvec.push(i);
        frame!();
    }
    let duration = start.elapsed();

    for i in 0..size{
        assert_eq!(bvec[i as usize], i);
    }

    println!("Time taken BVec: {:?}", duration);
}


#[test]
pub fn push_reallocation_bench_a_vec(){
    let size = 10000000;
    let mut bvec = AVec::<i32>::new(512);
    let start = Instant::now();
    for i in 0..size{
        bvec.push(i);
        frame!();
    }
    let duration = start.elapsed();

    for i in 0..size{
        assert_eq!(bvec[i as usize], i);
    }

    println!("Time taken BVec: {:?}", duration);
}

#[test]
pub fn push_reallocation_bench_o_vec(){
    let size = 10000;
    let mut vec = Vec::<i32>::new();
    let start = Instant::now();
    for i in 0..size{
        vec.push(i);
    }
    let duration = start.elapsed();
    println!("Time taken Vec: {:?}", duration);
}