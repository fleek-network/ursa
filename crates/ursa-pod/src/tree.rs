use arrayref::array_ref;
use arrayvec::ArrayVec;
use std::ptr;
use std::{borrow::Borrow, cmp::Ordering, fmt::Debug};
use thiserror::Error;

/// An incremental verifier that can consume a stream of proofs and content
/// and verify the integrity of the content using a blake3 root hash.
pub struct IncrementalVerifier {
    iv: blake3::ursa::IV,
    cursor: *mut IncrementalVerifierTreeNode,
    index: usize,
    stack: ArrayVec<*mut IncrementalVerifierTreeNode, 2>,
    next_head: *mut IncrementalVerifierTreeNode,
    is_done: bool,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum IncrementalVerifierError {
    #[error("The proof provided to the verifier does not have a valid length.")]
    InvalidProofSize,
    #[error("The proof provided did not belong to the tree.")]
    HashMismatch,
    #[error("Verifier has already finished its job.")]
    VerifierTerminated,
}

struct IncrementalVerifierTreeNode {
    parent: *mut IncrementalVerifierTreeNode,
    left: *mut IncrementalVerifierTreeNode,
    right: *mut IncrementalVerifierTreeNode,
    hash: [u8; 32],
}

impl IncrementalVerifier {
    /// Create a new incremental verifier that verifies an stream of proofs and
    /// content against the provided root hash.
    pub fn new(root_hash: [u8; 32], index: usize) -> Self {
        let node = Box::new(IncrementalVerifierTreeNode {
            parent: ptr::null_mut(),
            left: ptr::null_mut(),
            right: ptr::null_mut(),
            hash: root_hash,
        });

        Self {
            iv: blake3::ursa::IV::new(),
            cursor: Box::into_raw(node),
            index,
            stack: ArrayVec::new(),
            next_head: ptr::null_mut(),
            is_done: false,
        }
    }

    /// Returns true if the stream is complete.
    pub fn is_done(&self) -> bool {
        self.is_done
    }

    /// Moves the verifier to the finished state.
    #[inline(always)]
    fn finish(&mut self) {
        self.is_done = true;
        unsafe {
            while !(*self.cursor).parent.is_null() {
                self.cursor = (*self.cursor).parent;
            }
        }
    }

    /// Verify the new content.
    pub fn verify(
        &mut self,
        block: blake3::ursa::BlockHasher,
    ) -> Result<(), IncrementalVerifierError> {
        if self.is_done {
            return Err(IncrementalVerifierError::VerifierTerminated);
        }

        // 1. Hash the content using a block hasher with the current index.
        // 2. Compare to the hash we have under the cursor.
        // 3. Move to the next node.

        let hash = block.finalize(self.is_root());
        if &hash != self.current_hash() {
            return Err(IncrementalVerifierError::HashMismatch);
        }

        // If we're in the root node (when the data is one block or less), we
        // should not move to the next.
        if !self.is_root() {
            let curr = self.cursor;
            self.move_to_next();
            self.index += 1;
            debug_assert!(!self.cursor.is_null());

            // If after moving to the next node we're still at the same node
            // this simply means that there is no more data, move to the done
            // state.
            if self.cursor == curr {
                self.finish();
            }
        }

        Ok(())
    }

    /// Go to the next element in the tree.
    ///
    /// # Assumes
    ///
    /// This function assumes that is not called on the `root` node.
    fn move_to_next(&mut self) {
        debug_assert!(!self.is_root(), "Next should not be called on the root.");

        // To traverse to the next node in the tree we need to follow the
        // following algorithm:
        //
        // - assume we're currently at node `i`.
        // - `(trailing_ones(i) + 1)P . 1R . *L`
        // P: Go to parent node
        // R: Go to the right node
        // L: Go to the left node
        // number on the left determines how many times to perform a step,
        // and * means as much as we can.

        // Step P:
        // Notice how this loop is always executed at least once.
        for _ in 0..self.index.trailing_ones() + 1 {
            // SAFETY: We can always assume that `self.cursor` is not null, so this
            // is a safe dereference of self.cursor, and the intention of this step
            // is to guard against setting the cursor to null in the next instructions.
            unsafe {
                if (*self.cursor).parent.is_null() {
                    self.finish();
                    return;
                }
            }

            // SAFETY: As mentioned `self.cursor` is never null, and we also already
            // checked for `self.cursor.parent` to not be null in the previous unsafe
            // block.
            self.cursor = unsafe { (*self.cursor).parent };
        }

        // SAFETY: Since the incremental verifier only moves to the right, this means
        // we will never going to access the node on the left side of a node which we
        // have already visited, so we can free the memory.
        unsafe {
            drop(Box::new((*self.cursor).left));
            (*self.cursor).left = ptr::null_mut();
        }

        // Step R:
        // SAFETY: Since this function (`next`) is never called when we're in the root
        // this means both of the left and right children are set during the initialization
        // of the `IncrementalVerifierTreeNode`.
        //
        // And we only ever set the `left` children to null, so we can always assume that for
        // a non-root/non-leaf node, the `right` child is *ALWAYS* set and is not null.
        //
        // And since we got to this current cursor by moving up the tree (on the left side)
        // this simply means that the right side is also set. See the `for` loop a few lines
        // ago where we always go through the branch at least 1 time before we reach this point
        // in the code.
        self.cursor = unsafe { (*self.cursor).right };
        debug_assert!(!self.cursor.is_null());

        // Step L:
        self.move_to_leftmost();
    }

    /// Feed some new proof to the verifier which it can use to expand its internal
    /// blake3 tree.
    pub fn feed_proof(&mut self, proof: &[u8]) -> Result<(), IncrementalVerifierError> {
        if !is_valid_proof_len(proof.len()) {
            return Err(IncrementalVerifierError::InvalidProofSize);
        }

        if proof.is_empty() {
            return Ok(());
        }

        for segment in proof.chunks(32 * 8 + 1) {
            let sign = segment[0];

            for (i, hash) in segment[1..].chunks_exact(32).enumerate() {
                let should_flip = (1 << (8 - i - 1)) & sign != 0;
                self.push(should_flip, *array_ref![hash, 0, 32]);
            }
        }

        self.finalize_expansion()
    }

    /// Push a new proof chunk to the stack of pending subtree and merge the
    /// two previous pushed values if they are present.
    ///
    /// # Guarantees
    ///
    /// This function guarantees that after getting called the value of `self.next_head`
    /// is no longer null.
    fn push(&mut self, flip: bool, hash: [u8; 32]) {
        // Merge the two previous subtree as non-root hashes.
        if self.stack.is_full() {
            self.merge_stack(false);
        }

        let node = Box::into_raw(Box::new(IncrementalVerifierTreeNode {
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

    /// # Panics
    ///
    /// If the stack does not have two elements.
    fn finalize_expansion(&mut self) -> Result<(), IncrementalVerifierError> {
        assert!(self.stack.is_full());

        // SAFETY: This function is only called after validating the proof len and since the
        // smallest valid proof (that goes through without early return) has two hashes, it
        // is guranteed that the stack has at least two elements so this call to `merge_stack`
        // will not panic.
        self.merge_stack(self.is_root());
        debug_assert_eq!(self.stack.len(), 1);

        // SAFETY: `merge_stack` guarantees the stack has exactly one item.
        let node = self.stack.pop().unwrap();
        debug_assert!(!node.is_null());

        // SAFETY: This block only contains debug assertions.
        unsafe {
            // the cursor *must* not have children.
            debug_assert!((*self.cursor).left.is_null());
            debug_assert!((*self.cursor).right.is_null());
            // the new parent node *must* have children.
            // This will always be true because a valid proof has at least two
            // hashes, which means it will be merged into one node that has both
            // a left and right child set.
            debug_assert!(!(*node).left.is_null());
            debug_assert!(!(*node).right.is_null());
        }

        // SAFETY:
        // 1. Dereferencing the `self.cursor` is safe since we don't set it to null.
        // 2. `self.merge_stack` guarantees that the new node in the stack which we
        //     popped has both children set.
        unsafe {
            if &(*node).hash != self.current_hash() {
                // This is an error and we need to return the error, we should also
                // remember that `node` is not referenced by anyone anymore so we should
                // remove it here before we return.
                debug_assert!(!(*node).left.is_null());
                debug_assert!(!(*node).right.is_null());
                drop(Box::from_raw(node));
                return Err(IncrementalVerifierError::HashMismatch);
            }

            // Set the left and right children of the current cursor.
            (*self.cursor).left = (*node).left;
            (*self.cursor).right = (*node).right;
        }

        // SAFETY: node.left and node.right are always set at this point and self.cursor is
        // also never null.
        unsafe {
            // Update the parent of left and right to link to the cursor
            // and not the new parent node.
            (*(*node).left).parent = self.cursor;
            (*(*node).right).parent = self.cursor;
        }

        // SAFETY: At this point of the code `self.cursor.left` and `self.cursor.right` are
        // set to the `node.left` and `node.right` so by setting them to null here, we are
        // not losing track of those pointers.
        unsafe {
            // Remove the left and right node of the new node so we can
            // drop it without dropping the children.
            (*node).left = ptr::null_mut();
            (*node).right = ptr::null_mut();
        }

        // SAFETY: We no longer need this node, and since it was popped from the stack it
        // is not referenced anywhere else, so we can free that memory, furthermore the
        // left and right children on the node are set to null at this point so dropping
        // the node will not drop those nodes.
        unsafe {
            debug_assert!((*node).left.is_null());
            debug_assert!((*node).right.is_null());
            drop(Box::from_raw(node));
        }

        // If we're at the root right now instead of traversing all the way to
        // the deepest left node, we need to respect the value of `self.index`
        // (in case it is not zero) and instead try to get to that node.
        if self.is_root() && self.index != 0 {
            debug_assert!(!self.next_head.is_null());
            // SAFETY: For any non-empty proof the `push` function is at least called once,
            // and it is guranteed that self.next_head is set to non-null pointer after calling
            // the `push` and since we are here in the code it means the stack was full which
            // translates into `push` function being at least called once.
            self.cursor = self.next_head;
            self.next_head = ptr::null_mut();
        } else {
            // Traverse the current cursor into the deepest newly added left node so that
            // our guarantee about the cursor not having children is preserved.
            self.move_to_leftmost();
        }

        Ok(())
    }

    /// Returns true if the current cursor is pointing to the root of the tree.
    #[inline(always)]
    fn is_root(&self) -> bool {
        debug_assert!(!self.cursor.is_null(), "cursor is null");
        // SAFETY: Dereferencing cursor is safe since we never set it a null value.
        unsafe { (*self.cursor).parent.is_null() }
    }

    /// Returns the hash of the current node in the tree.
    #[inline(always)]
    fn current_hash(&self) -> &[u8; 32] {
        debug_assert!(!self.cursor.is_null(), "cursor is null");
        // SAFETY: Dereferencing cursor is safe since we never set it a null value.
        unsafe { &(*self.cursor).hash }
    }

    /// Moves the cursor to the leftmost node under the cursor.
    #[inline(always)]
    fn move_to_leftmost(&mut self) {
        debug_assert!(!self.cursor.is_null(), "cursor is null");
        // SAFETY: We can always assume dereferencing the cursor is safe since
        // we guarantee never setting it to null.
        //
        // And even here we change the cursor to a new value, after we're checking
        // it's not null.
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

        let parent_hash = self.iv.merge(left_cv, right_cv, is_root);
        let parent = Box::into_raw(Box::new(IncrementalVerifierTreeNode {
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
        // of `IncrementalVerifierTreeNode` will be called recursively and will free the entire
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

impl Drop for IncrementalVerifierTreeNode {
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
    fn new_internal(tree: &[[u8; 32]], walker: TreeWalker) -> Self {
        let size = walker.size_hint().0;
        let mut encoder = ProofEncoder::new(size);
        for (direction, index) in walker {
            debug_assert!(index < tree.len(), "Index overflow.");
            encoder.insert(direction, &tree[index]);
        }
        encoder.finalize()
    }

    /// Construct a new proof for the given block index from the provided
    /// tree.
    pub fn new(tree: &[[u8; 32]], block: usize) -> Self {
        Self::new_internal(tree, TreeWalker::new(block, tree.len()))
    }

    /// Construct proof for the given block number assuming that previous
    /// blocks have already been sent.
    pub fn resume(tree: &[[u8; 32]], block: usize) -> Self {
        Self::new_internal(tree, TreeWalker::resume(block, tree.len()))
    }

    /// Return the proof as a slice.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[self.index..]
    }

    /// Return the length of the proof.
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
    pub fn finalize(mut self) -> ProofBuf {
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
            // shit the final sign byte.
            self.buffer[self.cursor - 1] <<= 8 - (self.size & 7);
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
        if tree_len <= 1 {
            return Self::empty();
        }

        let walker = Self {
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
            return Self::empty();
        }

        walker
    }

    /// Construct a new [`TreeWalker`] to walk the tree assuming that a previous walk
    /// to the previous block has been made, and does not visit the nodes that the previous
    /// walker has visited.
    ///
    /// # Panics
    ///
    /// If target is zero. It doesn't make sense to call this function with target=zero since
    /// we don't have a -1 block that is already visited.
    pub fn resume(target: usize, tree_len: usize) -> Self {
        assert_ne!(target, 0, "Block zero has no previous blocks.");

        // Compute the index of the target in the tree representation.
        let target_index = target * 2 - target.count_ones() as usize;
        // If the target is not in this tree (out of bound) or the tree size is not
        // large enough for a resume walk return the empty iterator.
        if target_index >= tree_len || tree_len < 3 {
            return Self::empty();
        }

        let distance_to_ancestor = target.trailing_zeros();
        let mut ancestor = target_index + ((1 << (distance_to_ancestor + 1)) - 2);

        let subtree_size = if ancestor >= tree_len - 2 {
            ancestor = tree_len - 2;
            let root_subtree_size = (tree_len + 2) / 2;
            let left_subtree_size = previous_pow_of_two(root_subtree_size);
            let right_subtree_size = root_subtree_size - left_subtree_size;
            right_subtree_size
        } else if distance_to_ancestor == 0 {
            0
        } else {
            1 << distance_to_ancestor
        };

        if subtree_size == 1 {
            return Self::empty();
        }

        Self {
            target: target_index,
            current: ancestor,
            subtree_size,
        }
    }

    #[inline(always)]
    const fn empty() -> Self {
        Self {
            target: 0,
            current: 0,
            subtree_size: 0,
        }
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
    Target,
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
            return Some((Direction::Target, self.current));
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
        // If we're done iterating return 0.
        if self.subtree_size == 0 {
            return (0, Some(0));
        }

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

/// Validates that the provided number of bytes is a valid number of bytes for a proof
/// buffer. A valid proof is either
#[inline(always)]
fn is_valid_proof_len(n: usize) -> bool {
    const SEG_SIZE: usize = 32 * 8 + 1;
    let sign_bytes = (n + SEG_SIZE - 1) / SEG_SIZE;
    let hash_bytes = n - sign_bytes;
    hash_bytes & 31 == 0 && n <= 32 * 47 + 6 && ((hash_bytes / 32) >= 2 || n == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a mock tree that has n leaf nodes, each leaf node `i` starting
    /// from 1 has their `i`-th bit set to 1, and merging two nodes is done
    /// via `|` operation.
    fn make_mock_tree(n: u8) -> Vec<u128> {
        let n = n as usize;
        assert!(n > 0 && n <= 128);
        let mut tree = Vec::with_capacity(n * 2 - 1);
        let mut stack = Vec::with_capacity(8);
        for counter in 0..n {
            let mut node = 1u128 << counter;
            let mut counter = counter;
            while counter & 1 == 1 {
                let prev = stack.pop().unwrap();
                tree.push(node);
                node = node | prev;
                counter >>= 1;
            }
            stack.push(node);
            tree.push(node);
        }

        while stack.len() >= 2 {
            let a = stack.pop().unwrap();
            let b = stack.pop().unwrap();
            tree.push(a | b);
            stack.push(a | b);
        }

        tree
    }

    #[test]
    fn tree_walker() {
        for size in 2..100 {
            let tree = make_mock_tree(size);

            for start in 0..size {
                let mut walk = TreeWalker::new(start as usize, tree.len()).collect::<Vec<_>>();
                walk.reverse();

                assert_eq!(walk[0].0, Direction::Target);
                assert_eq!(tree[walk[0].1], (1 << start));

                let mut current = tree[walk[0].1];

                for (direction, i) in &walk[1..] {
                    let node = tree[*i];

                    assert_eq!(
                        node & current,
                        0,
                        "the node should not have common bits with the current node."
                    );

                    match direction {
                        Direction::Target => panic!("Target should only appear at the start."),
                        Direction::Left => {
                            assert_eq!(((current >> 1) & node).count_ones(), 1);
                            current |= node;
                        }
                        Direction::Right => {
                            assert_eq!(((current << 1) & node).count_ones(), 1);
                            current |= node;
                        }
                    }
                }

                assert_eq!(tree[tree.len() - 1], current);
            }
        }
    }

    #[test]
    fn tree_walker_one_block() {
        let walk = TreeWalker::new(0, 1).collect::<Vec<_>>();
        assert_eq!(walk.len(), 0);
    }

    #[test]
    fn tree_walker_out_of_bound() {
        let walk = TreeWalker::new(2, 3).collect::<Vec<_>>();
        assert_eq!(walk.len(), 0);
    }

    #[test]
    fn walker_partial_tree() {
        let walk = TreeWalker::resume(2, 5).collect::<Vec<_>>();
        assert_eq!(walk.len(), 0);
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
        expected.push(0b10000000); // sign byte
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
        expected.push(0b01000000); // sign byte
        expected.extend_from_slice(&[1; 32]);
        expected.extend_from_slice(&[0; 32]);
        assert_eq!(encoder.finalize(), expected.as_slice());

        let mut encoder = ProofEncoder::new(2);
        encoder.insert(Direction::Left, &[0; 32]);
        encoder.insert(Direction::Left, &[1; 32]);
        expected.clear();
        expected.push(0b11000000); // sign byte
        expected.extend_from_slice(&[1; 32]);
        expected.extend_from_slice(&[0; 32]);
        assert_eq!(encoder.finalize(), expected.as_slice());

        let mut encoder = ProofEncoder::new(2);
        encoder.insert(Direction::Right, &[0; 32]);
        encoder.insert(Direction::Left, &[1; 32]);
        expected.clear();
        expected.push(0b10000000); // sign byte
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
        // [sign byte + 1 hash] -> not valid proof
        // since it does not expand anything.
        assert_eq!(is_valid_proof_len(33), false);
        assert_eq!(is_valid_proof_len(40), false);
        assert_eq!(is_valid_proof_len(64), false);
        assert_eq!(is_valid_proof_len(65), true);

        for full_seg in 0..5 {
            let bytes = full_seg * 32 * 8 + full_seg;
            assert_eq!(is_valid_proof_len(bytes), true, "failed for len={bytes}");

            for partial_seg in 1..8 {
                let bytes = bytes + 1 + partial_seg * 32;
                let is_valid = bytes > 64;
                assert_eq!(
                    is_valid_proof_len(bytes),
                    is_valid,
                    "failed for len={bytes}"
                );
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
    fn incremental_verifier_basic() {
        let mut tree_builder = blake3::ursa::HashTreeBuilder::new();
        (0..4).for_each(|i| tree_builder.update(&[i; 256 * 1024]));
        let output = tree_builder.finalize();

        for i in 0..4 {
            let proof = ProofBuf::new(&output.tree, i);
            let mut verifier = IncrementalVerifier::new(*output.hash.as_bytes(), i);
            verifier.feed_proof(proof.as_slice()).unwrap();

            let mut block = blake3::ursa::BlockHasher::new();
            block.set_block(i);
            block.update(&[i as u8; 256 * 1024]);
            verifier.verify(block).unwrap();

            // for even blocks we should be able to also verify the next block without
            // the need to feed new proof.
            if i % 2 == 0 {
                let mut block = blake3::ursa::BlockHasher::new();
                block.set_block(i + 1);
                block.update(&[i as u8 + 1; 256 * 1024]);
                verifier.verify(block).unwrap();
            }
        }
    }

    #[test]
    fn incremental_verifier_small_data() {
        let mut tree_builder = blake3::ursa::HashTreeBuilder::new();
        tree_builder.update(&[17; 64]);
        let output = tree_builder.finalize();

        let proof = ProofBuf::new(&output.tree, 0);
        assert_eq!(proof.len(), 0);

        let mut verifier = IncrementalVerifier::new(*output.hash.as_bytes(), 0);
        verifier.feed_proof(proof.as_slice()).unwrap();

        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(0);
        block.update(&[17; 64]);
        verifier.verify(block).unwrap();
    }

    #[test]
    fn incremental_verifier_resume_simple() {
        let mut tree_builder = blake3::ursa::HashTreeBuilder::new();
        (0..4).for_each(|i| tree_builder.update(&[i; 256 * 1024]));
        let output = tree_builder.finalize();

        let mut verifier = IncrementalVerifier::new(*output.hash.as_bytes(), 0);

        let proof = ProofBuf::new(&output.tree, 0);
        verifier.feed_proof(proof.as_slice()).unwrap();
        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(0);
        block.update(&[0; 256 * 1024]);
        verifier.verify(block).unwrap();
        assert_eq!(verifier.index, 1);
        assert_eq!(verifier.current_hash(), &output.tree[1]);

        let proof = ProofBuf::resume(&output.tree, 1);
        assert_eq!(proof.len(), 0);
        verifier.feed_proof(proof.as_slice()).unwrap();
        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(1);
        block.update(&[1; 256 * 1024]);
        verifier.verify(block).unwrap();
        assert_eq!(verifier.index, 2);

        // now the cursor should have moved to 5.
        //         6
        //    2        5
        // 0    1   [3  4] <- pruned
        assert_eq!(verifier.current_hash(), &output.tree[5]);
        let proof = ProofBuf::resume(&output.tree, 2);
        verifier.feed_proof(proof.as_slice()).unwrap();
        assert_eq!(verifier.current_hash(), &output.tree[3]);
        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(2);
        block.update(&[2; 256 * 1024]);
        verifier.verify(block).unwrap();
        assert_eq!(verifier.index, 3);
        assert_eq!(verifier.current_hash(), &output.tree[4]);

        let proof = ProofBuf::resume(&output.tree, 3);
        assert_eq!(proof.len(), 0);
        verifier.feed_proof(proof.as_slice()).unwrap();

        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(3);
        block.update(&[3; 256 * 1024]);
        verifier.verify(block).unwrap();
        assert_eq!(verifier.index, 4);
        assert_eq!(verifier.is_done, true);
        assert_eq!(verifier.is_root(), true);
    }

    #[test]
    fn incremental_verifier_partial_tree() {
        let mut tree_builder = blake3::ursa::HashTreeBuilder::new();
        (0..3).for_each(|i| tree_builder.update(&[i; 256 * 1024]));
        let output = tree_builder.finalize();
        let mut verifier = IncrementalVerifier::new(*output.hash.as_bytes(), 0);

        let proof = ProofBuf::new(&output.tree, 0);
        verifier.feed_proof(proof.as_slice()).unwrap();
        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(0);
        block.update(&[0; 256 * 1024]);
        verifier.verify(block).unwrap();
        assert_eq!(verifier.index, 1);
        assert_eq!(verifier.current_hash(), &output.tree[1]);

        let proof = ProofBuf::resume(&output.tree, 1);
        assert_eq!(proof.len(), 0);
        verifier.feed_proof(proof.as_slice()).unwrap();
        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(1);
        block.update(&[1; 256 * 1024]);
        verifier.verify(block).unwrap();
        assert_eq!(verifier.index, 2);

        assert_eq!(verifier.current_hash(), &output.tree[3]);
        let proof = ProofBuf::resume(&output.tree, 2);
        assert_eq!(proof.len(), 0);
        verifier.feed_proof(proof.as_slice()).unwrap();
        assert_eq!(verifier.current_hash(), &output.tree[3]);
        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(2);
        block.update(&[2; 256 * 1024]);
        verifier.verify(block).unwrap();
        assert_eq!(verifier.index, 3);
        assert_eq!(verifier.is_root(), true);
        assert_eq!(verifier.is_done(), true);
    }

    #[test]
    fn incremental_verifier_range_req() {
        let mut tree_builder = blake3::ursa::HashTreeBuilder::new();
        (0..4).for_each(|i| tree_builder.update(&[i; 256 * 1024]));
        let output = tree_builder.finalize();

        let mut verifier = IncrementalVerifier::new(*output.hash.as_bytes(), 1);

        let proof = ProofBuf::new(&output.tree, 1);
        verifier.feed_proof(proof.as_slice()).unwrap();
        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(1);
        block.update(&[1; 256 * 1024]);
        verifier.verify(block).unwrap();
        assert_eq!(verifier.index, 2);

        assert_eq!(verifier.current_hash(), &output.tree[5]);
        let proof = ProofBuf::resume(&output.tree, 2);
        verifier.feed_proof(proof.as_slice()).unwrap();
        assert_eq!(verifier.current_hash(), &output.tree[3]);
        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(2);
        block.update(&[2; 256 * 1024]);
        verifier.verify(block).unwrap();
        assert_eq!(verifier.index, 3);
        assert_eq!(verifier.current_hash(), &output.tree[4]);

        let proof = ProofBuf::resume(&output.tree, 3);
        assert_eq!(proof.len(), 0);
        verifier.feed_proof(proof.as_slice()).unwrap();

        let mut block = blake3::ursa::BlockHasher::new();
        block.set_block(3);
        block.update(&[3; 256 * 1024]);
        verifier.verify(block).unwrap();
        assert_eq!(verifier.index, 4);
        assert_eq!(verifier.is_done, true);
        assert_eq!(verifier.is_root(), true);
    }
}
