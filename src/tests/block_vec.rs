use crate::block_vec;

#[test]
pub fn init_test(){
    let mut bvec = block_vec::BVec::<i32>::new();
    println!("Capacity: {}", bvec.capacity());
    assert_eq!(bvec.capacity(), 64);
}


#[test]
pub fn push_reallocation_test(){
    let size = 100000;
    let mut bvec = block_vec::BVec::<i32>::new();
    for i in 0..size{
        bvec.push(i);
    }
    assert_eq!(bvec.len(), 100000);
}