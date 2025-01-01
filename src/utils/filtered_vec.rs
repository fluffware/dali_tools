use std::cell::RefCell;
use std::ops::{Index, IndexMut};

pub struct Iter<'a, T>
where T: 'a
{
    vec: &'a FilteredVec<T>,
    next: usize,
}

impl<'a, T> Iterator for Iter<'a, T>
{
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item>
    {
        self.vec.build_until(self.next);
        let map = self.vec.map.borrow();
        if map.len() <= self.next {
            return None;
        }
        let v =  &self.vec.vec[map[self.next]];
        self.next += 1;
        Some(v)
    }
}

pub struct FilteredVec<T> {
    vec: Vec<T>,
    map: RefCell<Vec<usize>>,
    predicate: Box<dyn Fn(&T) -> bool + Send>,
}


impl<T> FilteredVec<T> {
    pub fn new<P>(vec: Vec<T>, predicate: P) -> FilteredVec<T>
    where
        P: Fn(&T) -> bool + 'static + Send,
    {
        FilteredVec {
            vec,
            map: RefCell::new(Vec::new()),
            predicate: Box::new(predicate),
        }
    }

    fn build_until(&self, until: usize) {
        let mut map = self.map.borrow_mut();
        let mut i = map.last().map(|i| i + 1).unwrap_or(0);
        while i < self.vec.len() {
            if (self.predicate)(&self.vec[i]) {
                map.push(i);
                if map.len() > until {
                    break;
                }
            }
            i += 1;
        }
    }

    fn build_all(&self) {
        let mut map = self.map.borrow_mut();
        let mut i = map.last().map(|i| i + 1).unwrap_or(0);
        while i < self.vec.len() {
            if (self.predicate)(&self.vec[i]) {
                map.push(i);
            }
            i += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.build_all();
        let map = self.map.borrow();
        map.len()
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, T>
    {
        Iter{vec: self, next: 0}
    }

    pub fn predicate<P>(&mut self, predicate: P)
    where
        P: Fn(&T) -> bool + 'static + Send
    {
        self.map.borrow_mut().clear();
        self.predicate = Box::new(predicate);
    }

    pub fn push(&mut self, v: T)
    {
        self.vec.push(v)
    }
    
    pub fn clear(&mut self)
    {
        self.vec.clear();
        self.map.borrow_mut().clear();
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.build_until(index);
        let map = self.map.borrow();
        if index < map.len() {
            Some(&self.vec[map[index]])
        } else {
            None
        }
    }

    pub fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {
        self.build_until(index);
        let map = self.map.borrow_mut();
        if index < map.len() {
            Some(&mut self.vec[map[index]])
        } else {
            None
        }
    }
}

impl<T> Index<usize> for FilteredVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("Index out of bounds")
    }
}

impl<T> IndexMut<usize> for FilteredVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.build_until(index);
        let mut map = self.map.borrow_mut();
        let v = &mut self.vec[map[index]];
        map.truncate(index); // Recheck the predicate on next operation
        v
    }
}
impl<T> From<Vec<T>> for FilteredVec<T>
{
    fn from(vec: Vec<T>) -> FilteredVec<T> {
        FilteredVec::new(vec, |_| true)
    }
}

impl<T> Into<Vec<T>> for FilteredVec<T>
{
    fn into(self) -> Vec<T> {
        self.vec
    }
}

#[test]
fn test_index() {
    let sv = FilteredVec::new(vec![1, 2, 3, 4, 5, 6, 7, 8], |x| x % 2 == 0);
    assert_eq!(sv[1], 4);
    assert_eq!(sv[3], 8);
    assert_eq!(sv[2], 6);
    assert_eq!(sv[0], 2);
    assert_eq!(sv.len(), 4);

    let sv = FilteredVec::new(vec![1, 2, 3, 4, 5, 6, 7, 8], |x| x % 2 == 0);
    assert_eq!(sv[1], 4);
    assert_eq!(sv.len(), 4);
    assert_eq!(sv[3], 8);
}

#[test]
fn test_index_mut() {
    let mut sv = FilteredVec::new(vec![1, 2, 3, 4, 5, 6, 7, 8], |x| x % 2 == 0);
    assert_eq!(sv.len(), 4);
    sv[1] = 5;
    assert_eq!(sv.len(), 3);
    assert_eq!(sv[2], 8);
}

#[test]
fn test_iter() {
    let sv = FilteredVec::new(vec![1, 2, 3, 4, 5, 6, 7, 8], |x| x % 2 == 0);

    for (i,v) in sv.iter().enumerate() {
        assert_eq!(*v, sv[i]);
    }

    let sv = FilteredVec::new(vec![1, 2, 3, 4, 5, 6, 7, 8], |x| x % 2 == 0);
    let even: Vec<i32> = sv.iter().cloned().collect();
    assert_eq!(&even, &[2,4,6,8]);

}
#[test]
fn test_into() {
    let v1:Vec<i32> = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let sv = FilteredVec::from(v1.clone());
    let v2: Vec<i32> = sv.into();
    assert_eq!(v1, v2);
}

#[test]
fn test_predicate()
{
    let mut v1 = FilteredVec::from(vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8], v1.iter().cloned().collect::<Vec<i32>>().as_slice());
    v1.predicate(|x| x%2==0);
    assert_eq!(&[2, 4, 6, 8], v1.iter().cloned().collect::<Vec<i32>>().as_slice());
    v1.predicate(|x| x%2==1);
    assert_eq!(&[1,3,5,7], v1.iter().cloned().collect::<Vec<i32>>().as_slice());
    v1.predicate(|_| false);
    assert_eq!(v1.len(),0);
    
}
