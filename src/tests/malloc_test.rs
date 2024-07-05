use crate::block_vec::{Chunk, malloc};

#[test]
pub fn malloc_write_and_read(){
    let size = 1000000usize;
    let (data, layout) = malloc::<i32>(size);
    unsafe {
        let mut chunk_iter = data;
        let chunk_end = data.add(size);
        let mut counter = 0usize;
        while chunk_iter != chunk_end{
            *chunk_iter = counter as i32;
            counter += 1;
            chunk_iter = chunk_iter.add(1);
        }
    }
    let data = unsafe{std::slice::from_raw_parts_mut(data, size)};

    for i in 0..size{
        assert_eq!(data[i], i as i32);
    }
}


struct TestStruct<T>{
    a: T,
    b: T,
    c: T,

}

#[test]
pub fn malloc_write_and_read_struct(){
    let size = 1000000usize;
    type InType = usize;
    let (data, layout) = malloc::<TestStruct<InType>>(size);
    unsafe {
        let mut chunk_iter = data;
        let chunk_end = data.add(size);
        let mut counter = 0usize;
        while chunk_iter != chunk_end{
            (*chunk_iter).a = counter as InType;
            (*chunk_iter).b = counter as InType + 1;
            (*chunk_iter).c = counter as InType + 2;
            counter += 1;
            chunk_iter = chunk_iter.add(1);
        }
    }
    let data = unsafe{std::slice::from_raw_parts_mut(data, size)};

    for i in 0..size{
        assert_eq!(data[i].a, i as InType);
        assert_eq!(data[i].b, i as InType + 1);
        assert_eq!(data[i].c, i as InType + 2 );
    }
}

#[test]
pub fn malloc_write_and_read_struct2(){
    let size = 1000000usize;
    type InType = usize;
    let (data, layout) = malloc::<TestStruct<InType>>(size);
    unsafe {
        let mut chunk_iter = data;
        let chunk_end = data.add(size);
        let mut counter = 0usize;
        while chunk_iter != chunk_end{
            let new_struct = TestStruct{
                a: counter as InType,
                b: counter as InType + 1,
                c: counter as InType + 2,
            };
            *chunk_iter = new_struct;
            counter += 1;
            chunk_iter = chunk_iter.add(1);
        }
    }
    let data = unsafe{std::slice::from_raw_parts_mut(data, size)};

    for i in 0..size{
        assert_eq!(data[i].a, i as InType);
        assert_eq!(data[i].b, i as InType + 1);
        assert_eq!(data[i].c, i as InType + 2 );
    }
}