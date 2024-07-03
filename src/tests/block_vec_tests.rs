use crate::block_vec;

#[test]
pub fn push_test(){
    let mut bvec = block_vec::BVec::<i32>::new();
    bvec.push(1);
    assert_eq!(bvec.len(), 1);
    assert_eq!(bvec.capacity(), 32);
}