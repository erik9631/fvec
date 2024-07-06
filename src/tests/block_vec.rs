use crate::block_vec;

#[test]
pub fn init_test(){
    let bvec = block_vec::BVec::<i32>::new(512);
    println!("Capacity: {}", bvec.capacity());
    assert_eq!(bvec.capacity(), 512);
}


#[test]
pub fn push_reallocation_test(){
    let size = 10000;
    let mut bvec = block_vec::BVec::<i32>::new(512);
    for i in 0..size{
        bvec.push(i);
    }

    for i in 0..size{
        assert_eq!(bvec[i as usize], i);
    }
    assert_eq!(bvec.len(), size as usize);
}