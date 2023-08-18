#![feature(test)]

extern crate test;

use ahash::{AHashMap, AHashSet, RandomState};
use indexmap::{IndexMap, IndexSet};
use std::collections::{BTreeMap, HashMap};
use test::Bencher;

#[bench]
fn ahashmap_insert(b: &mut Bencher) {
    let value = [0u64; 100];
    let c = 100_000;

    b.iter(|| {
        let mut map = AHashMap::new();
        for i in 0..c {
            map.insert((i as u128).to_be_bytes(), value);
        }
    });
}

#[bench]
fn hashmap_insert(b: &mut Bencher) {
    let value = [0u64; 100];
    let c = 100_000;

    b.iter(|| {
        let mut map = HashMap::new();
        for i in 0..c {
            map.insert((i as u128).to_be_bytes(), value);
        }
    });
}

#[bench]
fn btree_insert(b: &mut Bencher) {
    let value = [0u64; 100];
    let c = 100_000;

    b.iter(|| {
        let mut map = BTreeMap::new();
        for i in 0..c {
            map.insert((i as u128).to_be_bytes(), value);
        }
    });
}

#[bench]
fn ahashset_insert(b: &mut Bencher) {
    let c = 100_000;
    b.iter(|| {
        let mut set = AHashSet::new();
        for i in 0..c {
            set.insert((i as u128).to_be_bytes());
        }
    });
}

#[bench]
fn ahashset_lookup(b: &mut Bencher) {
    let c = 100_000;
    let mut set = AHashSet::new();
    for i in 0..c {
        set.insert((i as u128).to_be_bytes());
    }

    b.iter(|| {
        for i in 0..c {
            test::black_box(set.contains(&(i as u128).to_be_bytes()));
        }
    });
}

#[bench]
fn btree_lookup(b: &mut Bencher) {
    let c = 100_000;
    let mut map = BTreeMap::new();
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for i in 0..c {
            test::black_box(map.get(&(i as u128).to_be_bytes()));
        }
    });
}

#[bench]
fn hashmap_lookup(b: &mut Bencher) {
    let c = 100_000;
    let mut map = HashMap::new();
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for i in 0..c {
            test::black_box(map.get(&(i as u128).to_be_bytes()));
        }
    });
}

#[bench]
fn ahashmap_lookup(b: &mut Bencher) {
    let c = 100_000;
    let mut map = AHashMap::new();
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for i in 0..c {
            test::black_box(map.get(&(i as u128).to_be_bytes()));
        }
    });
}

#[bench]
fn ahashmap_loop(b: &mut Bencher) {
    let c = 100_000;
    let mut map = AHashMap::new();
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for (k, v) in &map {
            test::black_box(k);
            test::black_box(v);
        }
    });
}

#[bench]
fn hashmap_loop(b: &mut Bencher) {
    let c = 100_000;
    let mut map = HashMap::new();
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for (k, v) in &map {
            test::black_box(k);
            test::black_box(v);
        }
    });
}

#[bench]
fn btree_loop(b: &mut Bencher) {
    let c = 100_000;
    let mut map = BTreeMap::new();
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for (k, v) in &map {
            test::black_box(k);
            test::black_box(v);
        }
    });
}

#[bench]
fn ahashset_loop(b: &mut Bencher) {
    let c = 100_000;
    let mut set = AHashSet::new();
    for i in 0..c {
        set.insert((i as u128).to_be_bytes());
    }

    b.iter(|| {
        for k in &set {
            test::black_box(k);
        }
    });
}

#[bench]
fn ahashmap_remove(b: &mut Bencher) {
    let c = 100_000;
    let mut map = AHashMap::new();
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for i in 0..c {
            map.remove(&(i as u128).to_be_bytes());
        }
    });
}

#[bench]
fn hashmap_remove(b: &mut Bencher) {
    let c = 100_000;
    let mut map = HashMap::new();
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for i in 0..c {
            map.remove(&(i as u128).to_be_bytes());
        }
    });
}

#[bench]
fn btree_remove(b: &mut Bencher) {
    let c = 100_000;
    let mut map = BTreeMap::new();
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for i in 0..c {
            map.remove(&(i as u128).to_be_bytes());
        }
    });
}

#[bench]
fn ahashset_remove(b: &mut Bencher) {
    let c = 100_000;
    let mut set = AHashSet::new();
    for i in 0..c {
        set.insert((i as u128).to_be_bytes());
    }

    b.iter(|| {
        for i in 0..c {
            set.remove(&(i as u128).to_be_bytes());
        }
    });
}

#[bench]
fn indexmap_insert(b: &mut Bencher) {
    let c = 100_000;
    b.iter(|| {
        let mut map = IndexMap::with_hasher(RandomState::default());
        for i in 0..c {
            test::black_box(map.insert((i as u128).to_be_bytes(), ()));
        }
    });
}

#[bench]
fn indexmap_lookup(b: &mut Bencher) {
    let c = 100_000;
    let mut map = IndexMap::with_hasher(RandomState::default());
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for i in 0..c {
            test::black_box(map.get(&(i as u128).to_be_bytes()));
        }
    });
}

#[bench]
fn indexmap_loop(b: &mut Bencher) {
    let c = 100_000;
    let mut map = IndexMap::with_hasher(RandomState::default());
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for (k, v) in &map {
            test::black_box(k);
            test::black_box(v);
        }
    });
}

#[bench]
fn indexmap_remove(b: &mut Bencher) {
    let c = 100_000;
    let mut map = IndexMap::with_hasher(RandomState::default());
    for i in 0..c {
        map.insert((i as u128).to_be_bytes(), ());
    }

    b.iter(|| {
        for i in 0..c {
            test::black_box(map.remove(&(i as u128).to_be_bytes()));
        }
    });
}

#[bench]
fn indexset_lookup(b: &mut Bencher) {
    let c = 100_000;
    let mut set = IndexSet::with_hasher(RandomState::default());
    for i in 0..c {
        set.insert((i as u128).to_be_bytes());
    }

    b.iter(|| {
        for i in 0..c {
            test::black_box(set.contains(&(i as u128).to_be_bytes()));
        }
    });
}

#[bench]
fn indexset_loop(b: &mut Bencher) {
    let c = 100_000;
    let mut set = IndexSet::with_hasher(RandomState::default());
    for i in 0..c {
        set.insert((i as u128).to_be_bytes());
    }

    b.iter(|| {
        for k in &set {
            test::black_box(k);
        }
    });
}

#[bench]
fn indexset_remove(b: &mut Bencher) {
    let c = 100_000;
    let mut set = IndexSet::with_hasher(RandomState::default());
    for i in 0..c {
        set.insert((i as u128).to_be_bytes());
    }

    b.iter(|| {
        for i in 0..c {
            test::black_box(set.remove(&(i as u128).to_be_bytes()));
        }
    });
}

#[bench]
fn indexset_insert(b: &mut Bencher) {
    let c = 100_000;
    b.iter(|| {
        let mut set = IndexSet::with_hasher(RandomState::default());
        for i in 0..c {
            set.insert((i as u128).to_be_bytes());
        }
    });
}
