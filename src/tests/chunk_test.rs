use crate::block_vec::Chunk;

#[test]
pub fn chunk_init_test(){
    let chunk = Chunk::<i32>::alloc(10);
    assert_eq!(chunk.len(), 0);
    assert_eq!(chunk.capacity(), 10);
}

#[test]
pub fn chunk_push_test(){
    let size = 1000000usize;
    let mut chunk = Chunk::<i32>::alloc(size);
    for i in 0..size{
        chunk.push(i as i32);
    }

    for i in 0..size{
        assert_eq!(chunk[i], i as i32);
    }
    assert_eq!(chunk.len(), size);
    assert_eq!(chunk.capacity(), size);
}