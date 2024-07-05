
use std::alloc::{alloc, dealloc, Layout, realloc};
use std::ops::{Index, IndexMut};
use std::ptr::copy_nonoverlapping;
use std::slice::from_raw_parts_mut;
use tracy::zone;
use tracy::zone::Zone;


pub struct Chunk<T> {
    pub data: *mut T,
    free: bool,
    len: usize,
    capacity: usize,
    pub layout: Layout,
    pub tail: *mut T,
}

pub struct SharedData<T: 'static, const NUM_OF_CHUNKS: usize>{
    chunks_raw: (*mut Chunk<T>, Layout),
    chunks: &'static mut [Chunk<T>],
}
pub struct BVec<T: 'static, const NUM_OF_CHUNKS: usize = 4>{
    shared_data: SharedData<T, NUM_OF_CHUNKS>,
    current_chunk_ptr: *mut Chunk<T>,
    current_chunk: usize,
    last_chunk: usize,
}

impl <T> Chunk<T>{
    pub fn alloc(size: usize) -> Chunk<T> {
        let layout_result = Layout::array::<T>(size);
        let layout = layout_result.expect("Failed to create memory layout");
        let data = unsafe{alloc(layout) as *mut T};
        if data.is_null(){
            panic!("Failed to allocate memory");
        }

        return Chunk{
            data,
            len: 0,
            capacity: size,
            layout,
            tail: data,
            free: false,
        };
    }

    pub fn len(&self) -> usize{
        self.len
    }

    pub fn capacity(&self) -> usize{
        self.capacity
    }

    fn realloc(&self, new_size: usize) -> Result<(Chunk<T>), ()>{
        let new_layout = Layout::array::<T>(new_size).expect("Failed to create memory layout");
        let reallocated_addr = unsafe {
            realloc(
                self.data as *mut u8,
                self.layout,
                new_layout.size()
            )
        };

        // Check if reallocation was successful
        if reallocated_addr.is_null() {
            println!("Failed to reallocate memory");
            return Err(());  // Reallocation failed
        }
        println!("Realloc success");

        // Update Chunk fields
        Ok(Chunk{
            data: reallocated_addr as *mut T,
            len: self.len,
            capacity: new_size,
            layout: new_layout,
            tail: unsafe { self.data.add(self.len) },
            free: false,
        })
    }
    pub fn alloc_from_last(&self) -> Chunk<T>{
        zone!("alloc_from_last");
        let new_size = self.capacity * 2;
        return match { self.realloc(new_size) } {
            Ok(chunk) => {
                chunk
            },
            Err(_) => {
                let mut new_chunk: Chunk<T> = Chunk::alloc(new_size);
                new_chunk.tail = unsafe { new_chunk.tail.add(self.capacity) };
                new_chunk.len = self.capacity;
                new_chunk
            }
        }
    }
    #[cfg_attr(release, inline(always))]
    pub fn push(&mut self, val: T){
        unsafe{
            *self.tail = val;
            self.tail = self.tail.add(1);
            self.len += 1;
        }
    }

    pub fn free(&mut self){
        if self.free{
            panic!("Memory already freed!")
        }
        unsafe {dealloc(self.data as *mut u8, self.layout)}
        self.data = std::ptr::null_mut();
        self.free = true;
    }
}

impl<T> Index<usize> for Chunk<T>{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {&*self.data.add(index)}
    }
}

impl <T> IndexMut<usize> for Chunk<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {&mut *self.data.add(index)}
    }
}

impl<T> Clone for Chunk<T> {
    fn clone(&self) -> Self {
        return Chunk{
            data: self.data,
            len: self.len,
            capacity: self.capacity,
            layout: self.layout,
            tail: self.tail,
            free: self.free,
        }
    }
}

impl<T> Copy for Chunk<T>{}

/// TODO better solution
/// 1. of pushing to the tail and checking if the grow has locked writing to the tail
/// 2. Allocate a new array
/// 3. Since the tail is atomic, just offset it to the new array by the len of the previous array
/// 4. They can continue pushing to the tail
/// 5. In the mean time we can copy the data from the old array to the new array and we don't need to sync as they don't overlap
impl<T, const NUM_OF_CHUNKS: usize> BVec<T, NUM_OF_CHUNKS>{
    const NUM_OF_CHUNKS: usize = NUM_OF_CHUNKS;
    const MASK: usize = NUM_OF_CHUNKS - 1;
    pub fn new() -> Self {
        let size: usize = 512;
        let (chunks, layout) = malloc::<Chunk<T>>(NUM_OF_CHUNKS);
        unsafe {
            let chunk: Chunk<T> = Chunk::alloc(size);
            *chunks = chunk;

            let mut chunk_iter = chunks.add(1);
            let chunk_end = chunks.add(NUM_OF_CHUNKS);
            while chunk_iter != chunk_end{
                let chunk = &mut *chunk_iter;
                *chunk = (*chunk_iter.offset(-1)).alloc_from_last();
                chunk_iter = chunk_iter.add(1);
            }
        }


        let vec = BVec {
            shared_data: SharedData{
                chunks_raw: (chunks, layout),
                chunks: unsafe {from_raw_parts_mut(chunks, NUM_OF_CHUNKS)},

            },
            current_chunk: 0,
            current_chunk_ptr: chunks,
            last_chunk: NUM_OF_CHUNKS - 1,
        };
        return vec;
    }

    pub fn capacity (&self) -> usize{
        self.shared_data.chunks[self.current_chunk].capacity
    }

    pub fn len(&self) -> usize{
        self.shared_data.chunks[self.current_chunk].len
    }
    #[cfg_attr(release, inline(always))]
    pub fn push(&mut self, item: T){
        zone!("Push");
        let current_chunk: &mut Chunk<T> = self.get_curret_chunk_mut();
        current_chunk.push(item);

        if current_chunk.len() == current_chunk.capacity{
            self.current_chunk = Self::rotate_left(self.current_chunk);
            self.current_chunk_ptr = &mut self.shared_data.chunks[self.current_chunk] as *mut Chunk<T>;
            self.grow();
        }
    }

    #[cfg_attr(release, inline(always))]
    fn rotate_left(val: usize) -> usize {
        (val + 1) & Self::MASK
    }

    #[cfg_attr(release, inline(always))]
    fn rotate_right(val: usize) -> usize {
        (val.wrapping_sub(1)) & Self::MASK
    }

    #[cfg_attr(release, inline(always))]
    fn get_curret_chunk_mut(&mut self) -> &mut Chunk<T>{
        unsafe {&mut *self.current_chunk_ptr}
    }

    #[cfg_attr(release, inline(always))]
    fn get_curret_chunk(&self) -> &Chunk<T>{
        unsafe {& *self.current_chunk_ptr}
    }
    fn grow(&mut self){
        zone!("grow");
        //Lets do the simple case first
        let current_chunk: &Chunk<T> = &self.shared_data.chunks[self.current_chunk];
        let mut old_chunk: Chunk<T> = self.shared_data.chunks[Self::rotate_right(self.current_chunk)];
        let last_chunk: &Chunk<T> = &self.shared_data.chunks[self.last_chunk];
        let allocated_chunk: Chunk<T> = last_chunk.alloc_from_last();


        unsafe {
            zone!("copy");
            copy_nonoverlapping(old_chunk.data, current_chunk.data, old_chunk.capacity)
        }
        self.shared_data.chunks[Self::rotate_right(self.current_chunk)] = allocated_chunk;

        self.last_chunk = Self::rotate_left(self.last_chunk);
        old_chunk.free();
    }

}

impl<T> Index<usize> for BVec<T>{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get_curret_chunk().index(index)
    }
}

impl <T> IndexMut<usize> for BVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_curret_chunk_mut().index_mut(index)
    }
}


pub fn malloc<T>(size: usize) -> (*mut T, Layout){
    let layout_result = Layout::array::<T>(size);
    let layout = layout_result.expect("Failed to create memory layout");
    let data = unsafe{ alloc(layout) as *mut T};
    if data.is_null(){
        panic!("Failed to allocate memory");
    }
    (data, layout)
}