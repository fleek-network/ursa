use std::cmp::Ordering;

pub struct ProofEncoder {
    buffer: Box<[u8]>,
    cursor: usize,
    size: usize,
}

impl ProofEncoder {
    /// Create a new proof encoder for encoding a tree with the provided max number of
    /// items.
    pub fn new(n: usize) -> Self {
        // Compute the byte capacity for this encoder, which is 32-byte per hash
        // and 1 byte per 8 one of these.
        let capacity = n * 32 + (n + 8 - 1) / 8;
        // Create a Vec<u8> with the given size and set its len to the byte capacity
        // it is not important for us to take care of initializing the items since
        // the type is a u8 and has no drop logic except the deallocatation of the
        // slice itself.
        let mut vec = Vec::<u8>::with_capacity(capacity);
        if capacity > 0 {
            // SAFETY: The note above explains the use case. The justification of this
            // customization over just using a regular vector is that we need to write
            // from the end of the vector to the beginning (rev push), of course we can
            // use a regular vector and just flip everything at the end, but that will
            // be more complicated.
            unsafe {
                vec.set_len(capacity);
                // Make sure the last item in the vec which is supposed to be holding the
                // non-finalized sign byte is not dirty by setting it to zero.
                *vec.get_unchecked_mut(capacity - 1) = 0;
            }
        }

        let buffer = vec.into_boxed_slice();
        debug_assert_eq!(
            buffer.len(),
            capacity,
            "The buffer is smaller than expected."
        );

        Self {
            buffer,
            cursor: capacity,
            size: 0,
        }
    }

    /// Insert a
    pub fn insert(&mut self, direction: Direction, hash: &[u8; 32]) {
        // Get the current non-finalized sign byte.
        let mut sign = self.buffer[self.cursor - 1];

        self.cursor -= 32;
        self.buffer[self.cursor..self.cursor + 32].copy_from_slice(hash);

        // update the sign byte.
        if direction == Direction::Left {
            sign |= 1 << (self.size & 7);
        }

        self.size += 1;

        // Always put the sign as the leading byte of the data without
        // moving the cursor, this way the finalize can return a valid
        // proof for when it's called when the number of items does not
        // divide 8.
        self.buffer[self.cursor - 1] = sign;

        // If we have consumed a multiple of 8 hashes so far, consume the
        // sign byte by moving the cursor.
        if self.size & 7 == 0 {
            self.cursor -= 1;
            // If we have more data coming in, make sure the dirty which
            // will be used for the next sign byte is set to zero.
            if self.cursor > 0 {
                self.buffer[self.cursor - 1] = 0;
            }
        }
    }

    pub fn finalize(&self) -> &[u8] {
        // Here we don't want to consume or get a mutable reference to the internal
        // buffer we have, but also we might be called when the number of passed
        // hashes does not divide 8. In this case we already have the current sign
        // byte as the leading byte, so we need to return data start one byte before
        // the cursor.
        let mut cursor = self.cursor;

        if self.size & 7 > 0 {
            cursor -= 1;
        }

        &self.buffer[cursor..]
    }
}

/// The logic responsible for walking a full blake3 tree from top to bottom searching
/// for a path.
pub struct TreeWalker {
    /// Index of the node we're looking for.
    target: usize,
    /// Where we're at right now.
    current: usize,
    /// Size of the current sub tree, which is the total number of
    /// leafs under the current node.
    subtree_size: usize,
}

impl TreeWalker {
    /// Construct a new [`TreeWalker`] to walk a tree of `tree_len` items (in the array
    /// representation), looking for the provided `target`-th leaf.
    pub fn new(target: usize, tree_len: usize) -> Self {
        Self {
            // Compute the index of the n-th leaf in the array representation of the
            // tree.
            // see: https://oeis.org/A005187
            target: target * 2 - target.count_ones() as usize,
            // Start the walk from the root of the full tree, which is the last item
            // in the array representation of the tree.
            current: tree_len - 1,
            // for `k` number of leaf nodes, the total nodes of the binary tree will
            // be `n = 2k - 1`, therefore for computing the number of leaf nodes given
            // the total number of all nodes, we can use the formula `k = ceil((n + 1) / 2)`
            // and we have `ceil(a / b) = floor((a + b - 1) / b)`.
            subtree_size: (tree_len + 2) / 2,
        }
    }
}

/// The position of a element in an element in a binary tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The element is the current root of the tree, it's neither on the
    /// left or right side.
    Root,
    /// The element is on the left side of the tree.
    Left,
    /// The element is on the right side of the tree.
    Right,
}

impl Iterator for TreeWalker {
    type Item = (Direction, usize);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        // If we are at a leaf node, we've already finished the traversal, and if the
        // target is greater than the current (which can only happen in the first iteration),
        // the target is already not in this tree anywhere.
        if self.subtree_size == 0 || self.target > self.current {
            return None;
        }

        if self.current == self.target {
            self.subtree_size = 0;
            return Some((Direction::Root, self.current));
        }

        // The left subtree in a blake3 tree is always guranteed to contain a power of two
        // number of leaf (chunks), therefore the number of items on the left subtree can
        // be easily computed as the previous power of two (less than but not equal to)
        // the current items that we know our current subtree has, anything else goes
        // to the right subtree.
        let left_subtree_size = previous_pow_of_two(self.subtree_size);
        let right_subtree_size = self.subtree_size - left_subtree_size;
        // Use the formula `n = 2k - 1` to compute the total number of items on the
        // right side of this node, the index of the left node will be `n`-th item
        // before where we currently are at.
        let right_subtree_total_nodes = right_subtree_size * 2 - 1;
        let left = self.current - right_subtree_total_nodes - 1;
        let right = self.current - 1;

        match left.cmp(&self.target) {
            // The target is on the left side, so we need to prune the right subtree.
            Ordering::Equal | Ordering::Greater => {
                self.subtree_size = left_subtree_size;
                self.current = left;
                Some((Direction::Right, right))
            }
            // The target is on the right side, prune the left subtree.
            Ordering::Less => {
                self.subtree_size = right_subtree_size;
                self.current = right;
                Some((Direction::Left, left))
            }
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        // Return the upper bound as the result of the size estimation, the actual lower bound
        // can be computed more accurately but we don't really care about the accuracy of the
        // size estimate and the upper bound should be small enough for most use cases we have.
        //
        // This line is basically `ceil(log2(self.subtree_size)) + 1` which is the max depth of
        // the current subtree and one additional element + 1.
        let upper =
            usize::BITS as usize - self.subtree_size.saturating_sub(1).leading_zeros() as usize + 1;
        (upper, Some(upper))
    }
}

/// Returns the previous power of two of a given number, the returned
/// value is always less than the provided `n`.
#[inline(always)]
fn previous_pow_of_two(n: usize) -> usize {
    n.next_power_of_two() / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_walker() {
        let mut walker = TreeWalker::new(3, 12);
    }

    #[test]
    fn encoder_zero_capacity() {
        let encoder = ProofEncoder::new(0);
        assert_eq!(encoder.finalize().len(), 0);
        assert_eq!(0, encoder.buffer.len());
    }

    #[test]
    fn encoder_one_item() {
        let mut expected = Vec::<u8>::new();
        let mut hash = [0; 32];
        hash[0] = 1;
        hash[31] = 31;

        // sign byte on the left
        let mut encoder = ProofEncoder::new(1);
        encoder.insert(Direction::Left, &hash);
        expected.push(1); // sign byte
        expected.extend_from_slice(&hash);
        assert_eq!(encoder.finalize(), expected.as_slice());
        assert_eq!(expected.len(), encoder.buffer.len());

        // sign byte on the right
        let mut encoder = ProofEncoder::new(1);
        encoder.insert(Direction::Right, &hash);
        expected.clear();
        expected.push(0); // sign byte
        expected.extend_from_slice(&hash);
        assert_eq!(encoder.finalize(), expected.as_slice());
    }

    #[test]
    fn encoder_two_item() {
        let mut expected = Vec::<u8>::new();

        let mut encoder = ProofEncoder::new(2);
        encoder.insert(Direction::Right, &[0; 32]);
        encoder.insert(Direction::Right, &[1; 32]);
        expected.push(0); // sign byte
        expected.extend_from_slice(&[1; 32]);
        expected.extend_from_slice(&[0; 32]);
        assert_eq!(encoder.finalize(), expected.as_slice());

        let mut encoder = ProofEncoder::new(2);
        encoder.insert(Direction::Left, &[0; 32]);
        encoder.insert(Direction::Right, &[1; 32]);
        expected.clear();
        expected.push(1); // sign byte
        expected.extend_from_slice(&[1; 32]);
        expected.extend_from_slice(&[0; 32]);
        assert_eq!(encoder.finalize(), expected.as_slice());

        let mut encoder = ProofEncoder::new(2);
        encoder.insert(Direction::Left, &[0; 32]);
        encoder.insert(Direction::Left, &[1; 32]);
        expected.clear();
        expected.push(0b11); // sign byte
        expected.extend_from_slice(&[1; 32]);
        expected.extend_from_slice(&[0; 32]);
        assert_eq!(encoder.finalize(), expected.as_slice());

        let mut encoder = ProofEncoder::new(2);
        encoder.insert(Direction::Right, &[0; 32]);
        encoder.insert(Direction::Left, &[1; 32]);
        expected.clear();
        expected.push(0b10); // sign byte
        expected.extend_from_slice(&[1; 32]);
        expected.extend_from_slice(&[0; 32]);
        assert_eq!(encoder.finalize(), expected.as_slice());
        assert_eq!(expected.len(), encoder.buffer.len());
    }
}
