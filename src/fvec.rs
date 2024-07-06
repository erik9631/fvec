
use std::alloc::{alloc, dealloc, Layout, realloc};
use std::ops::{Index, IndexMut};
use tracy::zone;


pub struct FVec<T> {
    pub data: *mut T,
    free: bool,
    len: usize,
    capacity: usize,
    pub layout: Layout,
    pub tail: *mut T,
}

impl <T> FVec<T>{
    pub fn new(size: usize) -> FVec<T> {
        let layout_result = Layout::array::<T>(size);
        let layout = layout_result.expect("Failed to create memory layout");
        let data = unsafe{alloc(layout) as *mut T};
        if data.is_null(){
            panic!("Failed to allocate memory");
        }

        return FVec {
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

    fn realloc(&mut self, new_size: usize){
        let new_layout = Layout::array::<T>(new_size).expect("Failed to create memory layout");
        let new_adr = unsafe {
            alloc(
                new_layout
            )
        };

        unsafe {
            std::ptr::copy_nonoverlapping(self.data, new_adr as *mut T, self.len);
            dealloc(self.data as *mut u8, self.layout);
        }

        self.capacity = new_size;
        self.layout = new_layout;
        self.data = new_adr as *mut T;
        self.tail = unsafe { self.data.add(self.len) };
    }

    // fn realloc(&mut self, new_size: usize){
    //     let new_layout = Layout::array::<T>(new_size).expect("Failed to create memory layout");
    //     let reallocated_addr = unsafe {
    //         realloc(
    //             self.data as *mut u8,
    //             self.layout,
    //             new_layout.size()
    //         )
    //     };
    //
    //     // Check if reallocation was successful
    //     if reallocated_addr.is_null() {
    //         panic!("Failed to reallocate memory");
    //
    //     }
    //
    //     self.capacity = new_size;
    //     self.layout = new_layout;
    //     self.data = reallocated_addr as *mut T;
    //     self.tail = unsafe { self.data.add(self.len) };
    // }
    pub fn alloc_from_last(&self) -> FVec<T>{
        zone!("alloc_from_last");
        let new_size = self.capacity * 2;
        let mut new_chunk: FVec<T> = FVec::new(new_size);
        new_chunk.tail = unsafe { new_chunk.tail.add(self.capacity) };
        new_chunk.len = self.capacity;
        new_chunk
    }
    #[cfg_attr(release, inline(always))]
    pub fn push(&mut self, val: T){
        zone!("push");
        unsafe{
            *self.tail = val;
            self.tail = self.tail.add(1);
            self.len += 1;
            if(self.len == self.capacity){
                self.realloc(self.capacity * 2);
            }
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

impl<T> Index<usize> for FVec<T>{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {&*self.data.add(index)}
    }
}

impl <T> IndexMut<usize> for FVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {&mut *self.data.add(index)}
    }
}

impl<T> Clone for FVec<T> {
    fn clone(&self) -> Self {
        return FVec {
            data: self.data,
            len: self.len,
            capacity: self.capacity,
            layout: self.layout,
            tail: self.tail,
            free: self.free,
        }
    }
}

impl<T> Copy for FVec<T>{}

