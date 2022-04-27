# skip-list

Implementing a skip list with rust

# Examples

```rust
let mut skip_list = SkipList::default();
// insert
assert_eq!(skip_list.insert(1, 10), None); // there is no value with key with 1
assert_eq!(skip_list.insert(2, 20), None);
assert_eq!(skip_list.insert(3, 30), None);

// get
assert_eq!(skip_list.get(&1), Some(&10));
assert_eq!(skip_list.get(&2), Some(&20));
assert_eq!(skip_list.get(&3), Some(&30));

// update
assert_eq!(skip_list.insert(1, 100), Some(10)); // return old data
assert_eq!(skip_list.insert(2, 200), Some(20));
assert_eq!(skip_list.insert(3, 300), Some(30));

// iterator
for (k, v) in skip_list.iter() {
    let value = k * 100;
    assert_eq!(*v, value);
}

// delete
assert_eq!(skip_list.delete(&1), Some(100));
assert_eq!(skip_list.delete(&10), None);
assert_eq!(skip_list.get(&1), None);
```