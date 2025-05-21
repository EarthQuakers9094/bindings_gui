#[derive(Debug, Clone, Copy)]
pub enum SingleLinkedList<'a, A> {
    End,
    Value(A, &'a SingleLinkedList<'a, A>),
}

impl<'a, A> SingleLinkedList<'a, A> {
    pub fn snoc(&'a self, a: A) -> SingleLinkedList<'a, A> {
        SingleLinkedList::Value(a, self)
    }

    pub fn new() -> Self {
        Self::End
    }

    pub fn to_vec(&self) -> Vec<A>
    where
        A: Clone,
    {
        let mut at = self;

        let mut res = Vec::new();
        while let SingleLinkedList::Value(a, n) = at {
            res.push(a.clone());

            at = n;
        }

        res.reverse();

        res
    }
}
