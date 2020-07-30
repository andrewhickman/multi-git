use std::cmp;
use std::marker::PhantomData;

#[derive(Debug)]
/// The sequence of elements indexed by 0, 0 + stride, ...
pub struct SkipRange<'a, T> {
    ptr: *mut T,
    len: usize,
    stride: usize,
    _marker: PhantomData<&'a mut [T]>,
}

impl<'a, T> SkipRange<'a, T> {
    pub fn new(slice: &'a mut [T]) -> Self {
        SkipRange {
            ptr: slice.as_mut_ptr(),
            len: slice.len(),
            stride: 1,
            _marker: PhantomData,
        }
    }

    // Split into (0, stride * 2, ..) and (stride, stride * 3, ..)
    pub fn split(self) -> (Self, Option<Self>) {
        unsafe {
            let new_stride = match self.stride.checked_mul(2) {
                Some(new_stride) => new_stride,
                None => return (self, None),
            };

            let first = SkipRange {
                ptr: self.ptr,
                len: self.len,
                stride: new_stride,
                _marker: PhantomData,
            };

            let second = if self.stride < self.len {
                Some(SkipRange {
                    ptr: self.ptr.add(self.stride),
                    len: self.len - self.stride,
                    stride: new_stride,
                    _marker: PhantomData,
                })
            } else {
                None
            };

            (first, second)
        }
    }
}

impl<'a, T> Iterator for SkipRange<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.len == 0 {
                None
            } else {
                let result = &mut *self.ptr;

                let offset = cmp::min(self.len, self.stride);
                self.ptr = self.ptr.add(offset);
                self.len -= offset;

                Some(result)
            }
        }
    }
}

unsafe impl<'a, T> Send for SkipRange<'a, T> where T: Send {}

#[test]
fn test_skip_range() {
    let mut slice: Vec<i32> = (0..8).collect();

    let a = SkipRange::new(&mut slice);

    let (a, b) = a.split();
    let b = b.unwrap();

    let (aa, ab) = a.split();
    let ab = ab.unwrap();

    let (aba, abb) = ab.split();
    let abb = abb.unwrap();

    let (abba, abbb) = abb.split();
    assert!(abbb.is_none());

    let mut result: Vec<i32> = aa.chain(aba).chain(abba).chain(b).map(|&i| i).collect();
    assert_eq!(result, &[0, 4, 2, 6, 1, 3, 5, 7]);
    result.sort();
    assert_eq!(result, slice);
}
