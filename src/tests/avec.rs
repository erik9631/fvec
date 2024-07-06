use crate::avec::AVec;
use crate::fvec;

#[test]
pub fn init_test(){
    let bvec = AVec::<i32>::new(512);
    println!("Capacity: {}", bvec.capacity());
    assert_eq!(bvec.capacity(), 4096);
}


#[test]
pub fn push_reallocation_test(){
    let size = 100;
    let mut bvec = AVec::<i32>::new(2);
    for i in 0..size{
        bvec.push(i);
    }

    for i in 0..size{
        assert_eq!(bvec[i as usize], i);
    }
    assert_eq!(bvec.len(), size as usize);
}