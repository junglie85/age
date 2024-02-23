use std::{collections::VecDeque, ops::Index};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GenIdx(u32);

impl GenIdx {
    pub(crate) const INVALID: Self = Self::new(0xFFFFFF, 0xFF);

    pub(crate) const fn new(index: usize, gen: u8) -> Self {
        Self(((gen as u32) << 24) | index as u32)
    }

    pub(crate) fn idx(&self) -> u32 {
        self.0
    }

    pub(crate) fn split(&self) -> (usize, u8) {
        let index = (self.0 & 0xFFFFFF) as usize;
        let gen = ((self.0 >> 24) & 0xFF) as u8;
        (index, gen)
    }
}

struct Resource<T> {
    gen: u8,
    item: Option<T>,
}

pub(crate) struct GenVec<T> {
    resources: Vec<Resource<T>>,
    free: VecDeque<usize>,
}

impl<T> Default for GenVec<T> {
    fn default() -> Self {
        Self {
            resources: Vec::new(),
            free: VecDeque::new(),
        }
    }
}

impl<T> GenVec<T> {
    pub(crate) fn add(&mut self, resource: T) -> GenIdx {
        let index = if let Some(index) = self.free.pop_front() {
            index
        } else {
            self.resources.push(Resource { gen: 0, item: None });
            self.resources.len() - 1
        };

        self.resources[index].item = Some(resource);

        GenIdx::new(index, self.resources[index].gen)
    }

    pub(crate) fn remove(&mut self, idx: GenIdx) -> Option<T> {
        let (index, gen) = idx.split();
        assert_eq!(
            gen, self.resources[index].gen,
            "resource generation does not match"
        );

        // Recycle generation if we get to u8 max.
        if self.resources[index].gen == 255 {
            self.resources[index].gen = 0;
        } else {
            self.resources[index].gen += 1;
        }

        self.resources[index].item.take()
    }

    pub(crate) fn iter(&self) -> GenVecIter<'_, T> {
        GenVecIter {
            next: 0,
            resources: &self.resources,
        }
    }
}

pub(crate) struct GenVecIter<'a, T> {
    next: usize,
    resources: &'a [Resource<T>],
}

impl<'a, T> Iterator for GenVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let resource = self.resources.get(self.next);

        match resource {
            Some(resource) => {
                let gen_idx = GenIdx::new(self.next, resource.gen);
                match &resource.item {
                    Some(item) => Some(item),
                    None => self.next(),
                }
            }
            None => None,
        }
    }
}

impl<T> Index<GenIdx> for GenVec<T> {
    type Output = T;

    fn index(&self, idx: GenIdx) -> &Self::Output {
        let (index, gen) = idx.split();
        assert_eq!(
            gen, self.resources[index].gen,
            "resource generation does not match"
        );

        self.resources[index].item.as_ref().unwrap()
    }
}
