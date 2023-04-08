use arrayref::array_ref;
use arrayvec::ArrayVec;
use std::ptr;
use std::{borrow::Borrow, cmp::Ordering, fmt::Debug};

/// An incremental verifier that can consume a stream of proofs and content
/// and verify the integrity of the content using a blake3 root hash.
pub struct IncrementalVerifier {
    cursor: *mut IncrementalTreeNode,
    index: usize,
    stack: ArrayVec<*mut IncrementalTreeNode, 2>,
    next_head: *mut IncrementalTreeNode,
}

struct IncrementalTreeNode {
    parent: *mut IncrementalTreeNode,
    left: *mut IncrementalTreeNode,
    right: *mut IncrementalTreeNode,
    hash: [u8; 32],
}

impl IncrementalVerifier {
    /// Create a new incremental verifier that verifies an stream of proofs and
    /// content against the provided root hash.
    pub fn new(root_hash: [u8; 32]) -> Self {
        let node = Box::new(IncrementalTreeNode {
            parent: ptr::null_mut(),
            left: ptr::null_mut(),
            right: ptr::null_mut(),
            hash: root_hash,
        });

        Self {
            cursor: Box::into_raw(node),
            index: 0,
            stack: ArrayVec::new(),
            next_head: ptr::null_mut(),
        }
    }

    /// Verify a new
    pub fn verify(&mut self, input: &[u8]) {
        // 1. Hash the content using a block hasher with the current index.
        // 2. Compare to the hash we have under the cursor.
        // 3. Move to the next node.
    }

    /// Go to the next element in the tree.
    fn next(&mut self) {
        // To traverse to the next node in the tree we need to follow the
        // following algorithm:
        //
        // - assume we're currently at node `i`.
        // - `(leading_ones(i) + 1)P . 1R . *L`
        // P: Go to parent node
        // R: Go to the right node
        // L: Go to the left node
        // number on the left determines how many times to perform a step,
        // and * means as much as we can.

        // Step P:
        for _ in 0..self.index.leading_ones() + 1 {
            // TODO(qti3e): Make sure self.current.parent is not null.
            self.cursor = unsafe { (*self.cursor).parent };
        }

        // TODO(qti3e): Here we can drop the left subtree since we no longer need
        // it.

        // Step R:
        self.cursor = unsafe { (*self.cursor).right };

        // Step L:
        self.traverse_to_deepest_left_node();
    }

    /// Feed some new proof to the verifier which it can use to expand its internal
    /// blake3 tree.
    pub fn feed_proof(&mut self, proof: &[u8]) {
        // TODO(qti3e): Soft fail.
        assert!(is_valid_proof_len(proof.len()));

        if proof.is_empty() {
            return;
        }

        for segment in proof.chunks(32 * 8 + 1) {
            let sign = segment[0];
            let n = (segment.len() - 1) / 32;

            for (i, hash) in segment[1..].chunks_exact(32).enumerate() {
                let should_flip = (1 << (n - i - 1)) & sign != 0;
                self.push(should_flip, *array_ref![hash, 0, 32]);
            }
        }

        self.finalize_expansion();
    }

    fn push(&mut self, flip: bool, hash: [u8; 32]) {
        if self.stack.is_full() {
            self.merge_stack(false);
        }

        let node = Box::into_raw(Box::new(IncrementalTreeNode {
            parent: ptr::null_mut(),
            left: ptr::null_mut(),
            right: ptr::null_mut(),
            hash,
        }));

        self.stack.push(node);

        if flip && self.stack.is_full() {
            self.stack.swap(0, 1);
        }

        // Always remember the first node generated using a new proof, since this
        // is where we want to go next if we're currently at root.
        if self.next_head.is_null() {
            self.next_head = node;
        }
    }

    fn finalize_expansion(&mut self) {
        assert!(self.stack.is_full());

        self.merge_stack(self.is_root());
        debug_assert_eq!(self.stack.len(), 1);

        let node = self.stack.pop().unwrap();

        unsafe {
            // the cursor *must* not have children.
            debug_assert!((*self.cursor).left.is_null());
            debug_assert!((*self.cursor).right.is_null());
            // the new parent node *must* have children.
            debug_assert!(!(*node).left.is_null());
            debug_assert!(!(*node).right.is_null());
        }

        unsafe {
            // TODO(qti3e): Make this check into a safe fail.
            assert_eq!(&(*node).hash, self.current_hash());

            // Set the left and right children of the current cursor.
            (*self.cursor).left = (*node).left;
            (*self.cursor).right = (*node).right;
            // Update the parent of left and right to link to the cursor
            // and not the new parent node.
            (*(*node).left).parent = self.cursor;
            (*(*node).right).parent = self.cursor;

            // Remove the left and right node of the new node so we can
            // drop it without dropping the children.
            (*node).left = ptr::null_mut();
            (*node).right = ptr::null_mut();

            debug_assert!((*node).left.is_null());
            debug_assert!((*node).right.is_null());
            drop(Box::from_raw(node));
        }

        // If we're at the root right now instead of traversing all the way to
        // the deepest left node, we need to respect the value of `self.index`
        // (in case it is not zero) and instead try to get to that node.
        if self.is_root() && self.index != 0 {
            debug_assert!(!self.next_head.is_null());
            self.cursor = self.next_head;
            self.next_head = ptr::null_mut();
        } else {
            // Traverse the current cursor into the deepest newly added left node so that
            // our guarantee about the cursor not having children is preserved.
            self.traverse_to_deepest_left_node();
        }
    }

    #[inline(always)]
    fn is_root(&self) -> bool {
        debug_assert!(!self.cursor.is_null());
        unsafe { (*self.cursor).parent.is_null() }
    }

    #[inline(always)]
    fn current_hash(&self) -> &[u8; 32] {
        unsafe { &(*self.cursor).hash }
    }

    #[inline(always)]
    fn traverse_to_deepest_left_node(&mut self) {
        unsafe {
            while !(*self.cursor).left.is_null() {
                self.cursor = (*self.cursor).left;
            }
        }
    }

    /// Merge the current stack items into a new one.
    ///
    /// # Panics
    ///
    /// This function panics if the stack is not full. (i.e does not have 2 elements).
    ///
    /// # Guarantees
    ///
    /// After calling this function it is guranteed that:
    ///
    /// 1- The stack has exactly one item.
    /// 2- The new node in the stack has both its left and right children set.
    fn merge_stack(&mut self, is_root: bool) {
        assert!(self.stack.is_full());

        let right = self.stack.pop().unwrap();
        let left = self.stack.pop().unwrap();
        debug_assert!(!right.is_null(), "stack item is not supposed to be null.");
        debug_assert!(!left.is_null(), "stack item is not supposed to be null.");

        // SAFETY: The only function pushing to the stack is this same function
        // and we can guarantee that these are not null;
        let (left_cv, right_cv) = unsafe { (&(*left).hash, &(*right).hash) };

        let parent_hash = blake3::ursa::merge(left_cv, right_cv, is_root);
        let parent = Box::into_raw(Box::new(IncrementalTreeNode {
            parent: ptr::null_mut(),
            left,
            right,
            hash: parent_hash,
        }));

        // Push the new parent node into the stack.
        self.stack.push(parent);

        // SAFETY: The left and right are guranteed to not be null and they need to link to
        // the parent, and the parent will be pushed to the stack right after this line which
        // will result into its safe drop when the `IncrementalTree` is dropped.
        unsafe {
            debug_assert!(
                (*left).parent.is_null(),
                "parent node is supposed to be null."
            );
            debug_assert!(
                (*right).parent.is_null(),
                "parent node is supposed to be null."
            );
            (*left).parent = parent;
            (*right).parent = parent;
        }
    }
}

impl Drop for IncrementalVerifier {
    fn drop(&mut self) {
        if self.cursor.is_null() {
            return;
        }

        // SAFETY: Find the root of the tree from the current cursor by traversing
        // the tree up as much as we can, and free the leaf. The Drop implementation
        // of `IncrementalTreeNode` will be called recursively and will free the entire
        // data owned by this tree.
        unsafe {
            let mut current = self.cursor;
            while !(*current).parent.is_null() {
                current = (*current).parent;
            }
            debug_assert!(!current.is_null());
            debug_assert!((*current).parent.is_null());
            drop(Box::from_raw(current));
            self.cursor = ptr::null_mut();
        }

        // If there are any items left in the stack also free those.
        for pointer in self.stack.drain(..) {
            // SAFETY: The stack owns its pending items.
            unsafe {
                drop(Box::from_raw(pointer));
            }
        }
    }
}

impl Drop for IncrementalTreeNode {
    fn drop(&mut self) {
        // SAFETY: Each node owns its children and is responsible for
        // dropping them when its being drooped.
        unsafe {
            if !self.left.is_null() {
                drop(Box::from_raw(self.left));
                self.left = ptr::null_mut();
            }
            if !self.right.is_null() {
                drop(Box::from_raw(self.right));
                self.right = ptr::null_mut();
            }
        }
    }
}

/// A buffer containing a proof for a block of data.
pub struct ProofBuf {
    // The index at which the slice starts at in the boxed buffer.
    index: usize,
    buffer: Box<[u8]>,
}

impl ProofBuf {
    /// Construct a new proof for the given block index from the provided
    /// tree.
    pub fn new(tree: &[[u8; 32]], block: usize) -> Self {
        let walker = TreeWalker::new(block, tree.len());
        let size = walker.size_hint().0;
        let mut encoder = ProofEncoder::new(size);
        for (direction, index) in walker {
            debug_assert!(index < tree.len(), "Index overflow.");
            encoder.insert(direction, &tree[index]);
        }
        encoder.finalize()
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[self.index..]
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len() - self.index
    }
}

impl AsRef<[u8]> for ProofBuf {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Borrow<[u8]> for ProofBuf {
    #[inline(always)]
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Debug for ProofBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl PartialEq<&[u8]> for ProofBuf {
    fn eq(&self, other: &&[u8]) -> bool {
        self.as_slice().eq(*other)
    }
}

/// An encoder that manages a reverse buffer which can be used to convert the
/// root-to-leaf ordering of the [`TreeWalker`] to the proper stack ordering.
pub struct ProofEncoder {
    cursor: usize,
    size: usize,
    buffer: Box<[u8]>,
}

impl ProofEncoder {
    /// Create a new proof encoder for encoding a tree with the provided max number of
    /// items. An instance of ProofEncoder can not be used to encode more than the `n`
    /// items specified here. Providing an `n` smaller than the actual number of nodes
    /// can result in panics.
    pub fn new(n: usize) -> Self {
        // Compute the byte capacity for this encoder, which is 32-byte per hash and 1
        // byte per 8 one of these.
        let capacity = n * 32 + (n + 8 - 1) / 8;
        // Create a `Vec<u8>` with the given size and set its len to the byte capacity
        // it is not important for us to take care of initializing the items since the
        // type is a u8 and has no drop logic except the deallocatation of the slice
        // itself.
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

    /// Insert a new node into the tree, the direction determines whether or not we should
    /// be flipping the stack order when we're trying to rebuild the tree later on (on the
    /// client side).
    ///
    /// # Panics
    ///
    /// If more than the maximum number of times specified when constructing.
    pub fn insert(&mut self, direction: Direction, hash: &[u8; 32]) {
        assert!(self.cursor > 0);

        // Get the current non-finalized sign byte.
        let mut sign = self.buffer[self.cursor - 1];

        self.cursor -= 32;
        self.buffer[self.cursor..self.cursor + 32].copy_from_slice(hash);

        // update the sign byte.
        if direction == Direction::Left {
            sign |= 1 << (self.size & 7);
        }

        self.size += 1;

        // Always put the sign as the leading byte of the data without moving the
        // cursor, this way the finalize can return a valid proof for when it's
        // called when the number of items does not divide 8.
        self.buffer[self.cursor - 1] = sign;

        // If we have consumed a multiple of 8 hashes so far, consume the sign byte
        // by moving the cursor.
        if self.size & 7 == 0 {
            debug_assert!(self.cursor > 0);
            self.cursor -= 1;
            // If we have more data coming in, make sure the dirty byte which will
            // be used for the next sign byte is set to zero.
            if self.cursor > 0 {
                self.buffer[self.cursor - 1] = 0;
            }
        }
    }

    /// Finalize the result of the encoder and return the proof buffer.
    pub fn finalize(self) -> ProofBuf {
        // Here we don't want to consume or get a mutable reference to the internal buffer
        // we have, but also we might be called when the number of passed hashes does not
        // divide 8. In this case we already have the current sign byte as the leading byte,
        // so we need to return data start one byte before the cursor.
        //
        // Furthermore we could have been returning a Vec here, but that would imply that the
        // current allocated memory would needed to be copied first into the Vec (in case the
        // cursor != 0) and then freed as well, which is not really suitable for this use case
        // we want to provide the caller with the buffer in the valid range (starting from cursor)
        // and at the same time avoid any memory copy and extra allocation and deallocation which
        // might come with dropping the box and acquiring a vec.
        //
        // This way the caller will have access to the data, and can use it the way they want,
        // for example sending it over the wire, and then once they are done with reading the
        // data they can free the used memory.
        //
        // Another idea here is to also leverage a slab allocator on the Context object which we
        // are gonna have down the line which may improve the performance (not sure how much).
        if self.size & 7 > 0 {
            debug_assert!(self.cursor > 0);
            ProofBuf {
                buffer: self.buffer,
                index: self.cursor - 1,
            }
        } else {
            ProofBuf {
                buffer: self.buffer,
                index: self.cursor,
            }
        }
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
        let mut walker = Self {
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
        };

        if walker.target > walker.current {
            // If we know we're already out of bound, change the subtree_size to
            // zero so that the size_hint can also return zero for the upper bound.
            walker.subtree_size = 0;
        }

        walker
    }

    /// Return the index of the target element in the array representation of the
    /// complete tree.
    pub fn tree_index(&self) -> usize {
        self.target
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

#[inline(always)]
fn is_valid_proof_len(n: usize) -> bool {
    const SEG_SIZE: usize = 32 * 8 + 1;
    let sign_bytes = (n + SEG_SIZE - 1) / SEG_SIZE;
    let hash_bytes = n - sign_bytes;
    hash_bytes & 31 == 0 && n != 1 && n <= 32 * 47 + 6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_walker() {
        let mut walker = TreeWalker::new(2, 7);
        println!("{}", walker.current);
        for item in walker {
            println!("{:?}", item);
        }
    }

    #[test]
    fn encoder_zero_capacity() {
        let encoder = ProofEncoder::new(0);
        assert_eq!(0, encoder.buffer.len());
        assert_eq!(encoder.finalize().len(), 0);
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
        assert_eq!(expected.len(), encoder.buffer.len());
        assert_eq!(encoder.finalize(), expected.as_slice());

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
        assert_eq!(expected.len(), encoder.buffer.len());
        assert_eq!(encoder.finalize(), expected.as_slice());
    }

    #[test]
    fn valid_proof_len() {
        assert_eq!(is_valid_proof_len(0), true);
        assert_eq!(is_valid_proof_len(1), false);
        assert_eq!(is_valid_proof_len(2), false);
        assert_eq!(is_valid_proof_len(32), false);
        assert_eq!(is_valid_proof_len(33), true);
        assert_eq!(is_valid_proof_len(40), false);
        assert_eq!(is_valid_proof_len(64), false);
        assert_eq!(is_valid_proof_len(65), true);

        for full_seg in 0..5 {
            let bytes = full_seg * 32 * 8 + full_seg;
            assert_eq!(is_valid_proof_len(bytes), true, "failed for len={bytes}");

            for partial_seg in 1..8 {
                let bytes = bytes + 1 + partial_seg * 32;
                assert_eq!(is_valid_proof_len(bytes), true, "failed for len={bytes}");
                assert_eq!(
                    is_valid_proof_len(bytes - 1),
                    false,
                    "failed for len={bytes}"
                );
                assert_eq!(
                    is_valid_proof_len(bytes + 1),
                    false,
                    "failed for len={bytes}"
                );
            }
        }
    }

    #[test]
    fn incremental_tree_basic() {
        let mut tree_builder = blake3::ursa::HasherWithTree::new();
        for i in 0..4 {
            tree_builder.update(&[i; 256 * 1024]);
        }
        let output = tree_builder.finalize();

        for i in 0..4 {
            let proof = ProofBuf::new(&output.tree, i);
            let mut inc_tree = IncrementalVerifier::new(*output.hash.as_bytes());
            inc_tree.feed_proof(proof.as_slice());
        }
    }
}
