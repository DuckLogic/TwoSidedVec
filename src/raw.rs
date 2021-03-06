use std::ptr::NonNull;
use std::marker::PhantomData;
use std::ops::Add;
use std::mem;

use std::alloc::{handle_alloc_error, Allocator, Global, Layout};

pub struct RawTwoSidedVec<T> {
    middle: NonNull<T>,
    marker: PhantomData<T>,
    capacity: Capacity
}
impl<T> RawTwoSidedVec<T> {
    #[inline]
    pub fn new() -> Self {
        assert_ne!(mem::size_of::<T>(), 0, "Zero sized type!");
        RawTwoSidedVec {
            middle: NonNull::dangling(),
            marker: PhantomData,
            capacity: Capacity { back: 0, front: 0 }
        }
    }
    pub fn with_capacity(capacity: Capacity) -> Self {
        assert_ne!(mem::size_of::<T>(), 0, "Zero sized type!");
        if capacity.is_empty() {
            return RawTwoSidedVec::new()
        }
        let heap = Global::default();
        let layout = capacity.layout::<T>();
        unsafe {
            let memory = heap.allocate(layout)
                .unwrap_or_else(|_| handle_alloc_error(layout));
            let middle = (memory.as_ptr() as *mut T).add(capacity.back);
            RawTwoSidedVec::from_raw_parts(
                middle,
                capacity
            )
        }
    }
    /// Create a vector based on an existing pointer and capacity
    ///
    /// ## Safety
    /// Undefined behavior if middle doesn't have enough space for `capacity`
    /// elements (in either direction) or the memory was allocated incorrectly.
    #[inline]
    pub unsafe fn from_raw_parts(middle: *mut T, capacity: Capacity) -> Self {
        assert_ne!(mem::size_of::<T>(), 0, "Zero sized type!");
        debug_assert!(!middle.is_null());
        RawTwoSidedVec { middle: NonNull::new_unchecked(middle), marker: PhantomData, capacity }
    }
    #[inline]
    pub fn capacity(&self) -> &Capacity {
        &self.capacity
    }
    #[inline]
    pub fn middle(&self) -> *mut T {
        self.middle.as_ptr()
    }
    /// A pointer to the start of the allocation
    #[inline]
    fn alloc_start(&self) -> *mut T {
        unsafe { self.middle.as_ptr().sub(self.capacity.back) }
    }
    pub fn reserve(&mut self, request: CapacityRequest) {
        assert!(self.capacity.can_fit(request.used));
        let requested_capacity = request.used + request.needed;
        unsafe {
            // Reallocate
            let result = Self::with_capacity(requested_capacity);
            result.middle().sub(request.used.back).copy_from_nonoverlapping(
                self.middle().sub(request.used.back),
                request.used.back
            );
            result.middle().copy_from_nonoverlapping(
                self.middle(),
                request.used.front
            );
            *self = result; // Replace
        }
        debug_assert!(self.capacity.can_fit(requested_capacity));
    }
}
unsafe impl<#[may_dangle] T> Drop for RawTwoSidedVec<T> {
    #[inline]
    fn drop(&mut self) {
        if !self.capacity.is_empty() {
            let heap = Global::default();
            unsafe {
                let layout = self.capacity.layout::<T>();
                heap.deallocate(
                    NonNull::new_unchecked(self.alloc_start() as *mut u8),
                    layout
                );
            }
        }
    }
}
#[derive(Copy, Clone, Debug)]
pub struct Capacity {
    pub back: usize,
    pub front: usize
}
impl Capacity {
    #[inline]
    pub fn empty() -> Self {
        Capacity { back: 0, front: 0 }
    }
    #[inline]
    pub fn checked_total(&self) -> usize {
        self.back.checked_add(self.front).expect("Capacity overflow")
    }
    #[inline]
    pub fn total(&self) -> usize {
        self.back + self.front
    }
    #[inline]
    pub fn can_fit(&self, other: Capacity) -> bool {
        self.back >= other.back && self.front >= other.front
    }
    #[inline]
    fn layout<T>(&self) -> Layout {
        Layout::array::<T>(self.checked_total()).expect("Capacity overflow")
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.back == 0 && self.front == 0
    }
}
impl Add for Capacity {
    type Output = Capacity;

    #[inline]
    fn add(self, rhs: Capacity) -> Capacity {
        match (self.front.checked_add(rhs.front), self.back.checked_add(rhs.back)) {
            (Some(front), Some(back)) => Capacity { front, back },
            _ => panic!("Capacity overflow")
        }
    }
}
#[derive(Copy, Clone, Debug)]
pub struct CapacityRequest {
    pub used: Capacity,
    pub needed: Capacity
}
