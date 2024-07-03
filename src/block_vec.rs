use std::{alloc, thread};
use std::alloc::{alloc, Layout};
use std::mem::{size_of, swap};
use std::ptr::copy_nonoverlapping;
use std::sync::{Arc, atomic, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize};
use std::sync::atomic::Ordering::Relaxed;

#[inline(always)]
fn default_alloc_strategy<T: 'static>(vec: &BVec<T>) -> bool{
    return vec.len() * 10 >= vec.capacity() * 8;
}

pub struct SharedData<T>{
    data: AtomicPtr<T>,
    tail: AtomicPtr<T>,
    is_growing: s,
    capacity: AtomicUsize,
    grow_pow: AtomicUsize,
    push_mutex: Mutex<()>,
    len: AtomicUsize,
    layout: RwLock<Option<Layout>>,
}
pub struct BVec<T>{
    shared_data: Arc<SharedData<T>>,
    alloc_strategy: fn(vec: &Self) -> bool,
    grow_thread: Option<thread::JoinHandle<()>>,
}

/// TODO better solution
/// 1. of pushing to the tail and checking if the grow has locked writing to the tail
/// 2. Allocate a new array
/// 3. Since the tail is atomic, just offset it to the new array by the len of the previous array
/// 4. They can continue pushing to the tail
/// 5. In the mean time we can copy the data from the old array to the new array and we don't need to sync as they don't overlap
impl<T: 'static> BVec<T>{
    pub fn new() -> Self {
        let mut vec = BVec {
            shared_data: Arc::new(SharedData{
                data: AtomicPtr::new(std::ptr::null_mut()),
                tail: AtomicPtr::new(std::ptr::null_mut()),
                is_growing: AtomicBool::new(false),
                capacity: AtomicUsize::new(0),
                grow_pow: AtomicUsize::new(6),
                len: AtomicUsize::new(0),
                push_mutex: Mutex::new(()),
                layout: RwLock::new(None),
            }),
            alloc_strategy: default_alloc_strategy,
            grow_thread: None,
        };
        vec.grow();
            if let Some(grow_thread) = vec.grow_thread.take() {
                grow_thread.join().expect("Failed to join grow thread");
            }
        return vec;
    }

    pub fn capacity (&self) -> usize{
        return self.shared_data.capacity.load(Relaxed);
    }

    pub fn len(&self) -> usize{
        return self.shared_data.len.load(Relaxed);
    }
    #[inline(always)]
    pub fn push(&mut self, item: T){
        if default_alloc_strategy(&self){
            self.grow();
        }

        let mut tail = self.shared_data.tail.load(Relaxed);
        tail = unsafe{tail.offset(1)};
        unsafe{tail.write(item)};
        self.shared_data.len.fetch_add(1, Relaxed);
    }

    fn grow(&mut self){
        if self.shared_data.is_growing.load(Relaxed){
            return;
        }

        let shared_data = self.shared_data.clone();

        if let Some(grow_thread) = self.grow_thread.take(){
            grow_thread.join().expect("Failed to join grow thread");
        }
        self.grow_thread = Some(thread::spawn(move || {
            let (new_mem, layout) = alloc_new::<T>(shared_data.grow_pow.load(Relaxed));
            shared_data.grow_pow.fetch_add(1, Relaxed);

            let new_size = layout.size() / size_of::<T>();
            let len_before_swap = shared_data.len.load(Relaxed);
            unsafe {
                if shared_data.capacity.load(Relaxed) == 0{
                    shared_data.data.store(new_mem, Relaxed);
                    shared_data.tail.store(new_mem, Relaxed);
                    shared_data.capacity.store(new_size, Relaxed);
                    shared_data.layout.write().expect("Failed to get write to layout").clone_from(&Some(layout));
                    return;
                }

                // They can keep pushing
                shared_data.tail.store(new_mem.offset(len_before_swap as isize), Relaxed);

                copy_nonoverlapping(shared_data.data.load(Relaxed), new_mem, len_before_swap);
                shared_data.capacity.store(new_size, Relaxed);

                if let read_layout = shared_data.layout.write(){
                    let result = read_layout.expect("Failed to write layout").take().expect("No layout found");
                    dealloc(shared_data.data.load(Relaxed), result);
                    shared_data.layout.write().expect("Failed to write layout").clone_from(&Some(layout));
                }
                shared_data.data.store(new_mem, Relaxed);
            }
        }));
    }
}

fn dealloc<T>(ptr: *mut T, layout: Layout){
    unsafe{alloc::dealloc(ptr as *mut u8, layout)};
}
fn alloc_new<T>(pow: usize) -> (*mut T, Layout) {
    let base: usize = 2;
    // println!("{}^{}", base, pow);
    let new_size = base.pow(pow as u32);

    let layout_result = Layout::array::<T>(new_size);
    let layout = layout_result.expect("Failed to create memory layout");
    let new_data = unsafe{alloc::alloc(layout) as *mut T};
    if new_data.is_null(){
        panic!("Failed to allocate memory");
    }
    return (new_data, layout);
}
